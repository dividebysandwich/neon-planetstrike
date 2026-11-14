//! Roads between the bases and to the warp gates, along the lattice. Each is routed over the
//! land with A* on a 16 m grid (long gentle ways round steep ground and water, joining roads
//! already built), smoothed, given a height profile no steeper than `MAX_GRADE` (cutting
//! through rises, banked over dips) and graded into the heightfield: flat across `SHOULDER`
//! either side of its middle, its banks blending into the land beyond. All from the seed, as
//! the rest of the land.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use glam::{Vec2, Vec3};

use crate::noise::smoothstep;
use crate::terrain::{CELL, HALF, N, Pad, SEA, SEA_FLOOR, Terrain};

/// Half the drawn road's width.
pub const HALF_WIDTH: f32 = 7.0;
/// The land is flat this far either side of a road's middle.
pub const SHOULDER: f32 = 10.0;
/// Steepest rise over run along a road.
pub const MAX_GRADE: f32 = 0.15;
/// `Terrain::road_distance` of land this far or further from any road.
pub const NEAR_MAX: f32 = 255.0 / 8.0;
/// The widest bank from the shoulder into the land.
const MAX_BANK: f32 = 48.0;
/// The router's grid: every `STEP` samples (16 m).
const STEP: usize = 4;
const NR: usize = (N - 1) / STEP + 1;
/// Spacing of a road's points.
const SPACING: f32 = 4.0;

/// One end of a road: where it stops (in a base's gateway, in a warp gate), which way is out of
/// there, and how far straight out before it may turn (clear of the pad's structures).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoadEnd {
    pub at: Vec2,
    pub out: Vec2,
    pub reach: f32,
}

/// A road to build between two ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoadLink {
    pub a: RoadEnd,
    pub b: RoadEnd,
}

/// A road built: its middle, a point every few metres at the road's height.
#[derive(Clone, Debug, PartialEq)]
pub struct Road {
    pub points: Vec<Vec3>,
}

fn node_pos(i: usize, j: usize) -> Vec2 {
    Vec2::new(
        (i * STEP) as f32 * CELL - HALF,
        (j * STEP) as f32 * CELL - HALF,
    )
}

fn node_of(p: Vec2) -> (usize, usize) {
    let f = |v: f32| (((v + HALF) / (CELL * STEP as f32)).round() as i32).clamp(0, NR as i32 - 1);
    (f(p.x) as usize, f(p.y) as usize)
}

/// Routes, grades and carves the roads of `links` into `t` (in order: later ones join earlier
/// ones where they can). Returns the roads and each sample's distance to them (`road_near`).
pub(crate) fn build(t: &mut Terrain, pads: &[Pad], links: &[RoadLink]) -> (Vec<Road>, Vec<u8>) {
    let mut near = vec![255u8; N * N];
    let mut roads = Vec::new();
    if links.is_empty() {
        return (roads, Vec::new());
    }
    // Scratch: this road's distance and height at the samples it touches.
    let mut dist = vec![f32::INFINITY; N * N];
    let mut road_h = vec![0.0f32; N * N];
    let mut touched: Vec<usize> = Vec::new();
    for link in links {
        let mut way = route(t, pads, link, &near);
        merge(&mut way, &roads);
        let way = resample(&untangle(&way));
        let points = profile(t, pads, &way, &near);
        carve(
            t,
            pads,
            &points,
            &mut near,
            &mut dist,
            &mut road_h,
            &mut touched,
        );
        roads.push(Road { points });
    }
    (roads, near)
}

/// The way from one end to the other (points on the ground plane, every `SPACING` metres).
fn route(t: &Terrain, pads: &[Pad], link: &RoadLink, near: &[u8]) -> Vec<Vec2> {
    let a_out = link.a.at + link.a.out * link.a.reach;
    let b_out = link.b.at + link.b.out * link.b.reach;
    let start = node_of(a_out);
    let goal = node_of(b_out);
    let k = |(i, j): (usize, usize)| j * NR + i;
    let h_of = |i: usize, j: usize| t.at((i * STEP) as i32, (j * STEP) as i32);
    // Not through bases, towers or gates (the ends' own nodes excepted).
    let solid = |i: usize, j: usize| {
        let p = node_pos(i, j);
        pads.iter()
            .any(|pd| pd.center.distance(p) < pd.radius + 4.0)
    };
    let wet = |i: usize, j: usize| {
        let p = node_pos(i, j);
        h_of(i, j) < t.water(p.x, p.y) + 0.3
    };
    let on_road = |i: usize, j: usize| near[(j * STEP) * N + i * STEP] < (SHOULDER * 8.0) as u8;
    let (s, g) = (k(start), k(goal));
    let mut cost = vec![f32::INFINITY; NR * NR];
    let mut came = vec![u32::MAX; NR * NR];
    let mut open = BinaryHeap::new();
    let goal_p = node_pos(goal.0, goal.1);
    // (Never more than the cheapest step, along a road, costs.)
    let h = |i: usize, j: usize| node_pos(i, j).distance(goal_p) * 0.6;
    cost[s] = 0.0;
    open.push(Reverse((h(start.0, start.1).to_bits(), s as u32)));
    while let Some(Reverse((_, n))) = open.pop() {
        let n = n as usize;
        if n == g {
            break;
        }
        let (i, j) = (n % NR, n / NR);
        let here = h_of(i, j);
        for (di, dj) in [
            (1i32, 0i32),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ] {
            let (a, b) = (i as i32 + di, j as i32 + dj);
            if a < 0 || b < 0 || a >= NR as i32 || b >= NR as i32 {
                continue;
            }
            let (a, b) = (a as usize, b as usize);
            let m = b * NR + a;
            if m != g && solid(a, b) {
                continue;
            }
            let run = if di != 0 && dj != 0 { 1.414 } else { 1.0 } * CELL * STEP as f32;
            let grade = (h_of(a, b) - here).abs() / run;
            // Steep ground costs much more than going round it.
            let mut step = run * (1.0 + (grade / 0.12).powi(2) * 0.6).min(40.0);
            if wet(a, b) {
                step *= 5.0;
            }
            if on_road(a, b) {
                step *= 0.6;
            }
            let c = cost[n] + step;
            if c < cost[m] {
                cost[m] = c;
                came[m] = n as u32;
                open.push(Reverse(((c + h(a, b)).to_bits(), m as u32)));
            }
        }
    }
    let mut nodes = Vec::new();
    if cost[g].is_finite() {
        let mut n = g;
        while n != s {
            nodes.push(n);
            n = came[n] as usize;
        }
        nodes.reverse();
    }
    // Straight out of each end, then the nodes between (the ends' own nodes dropped, the
    // ends' straights standing for them).
    let mut pts = vec![link.a.at, a_out];
    let len = nodes.len();
    for (q, &n) in nodes.iter().enumerate() {
        if q + 1 == len {
            break;
        }
        pts.push(node_pos(n % NR, n / NR));
    }
    pts.push(b_out);
    pts.push(link.b.at);
    // Straighten the grid's zigzags (keeping its detours), then round off the corners.
    let mut pts = simplify(&pts, 20.0);
    for _ in 0..4 {
        pts = round_corners(&pts, 36.0);
    }
    resample(&pts)
}

/// Points along `pts` every `SPACING` metres (and its last).
fn resample(pts: &[Vec2]) -> Vec<Vec2> {
    let mut out = vec![pts[0]];
    let mut carry = 0.0;
    for w in pts.windows(2) {
        let (p, q) = (w[0], w[1]);
        let seg = p.distance(q);
        let mut d = SPACING - carry;
        while d < seg {
            out.push(p + (q - p) * (d / seg));
            d += SPACING;
        }
        carry = seg - (d - SPACING);
    }
    let end = pts[pts.len() - 1];
    if out.len() > 1 && out[out.len() - 1].distance(end) < 1.0 {
        out.pop();
    }
    out.push(end);
    out
}

/// Where `way` runs beside a road built before, onto it (gradually: no strands side by side).
fn merge(way: &mut [Vec2], roads: &[Road]) {
    const ON: f32 = 2.0 * HALF_WIDTH;
    const NEAR: f32 = 2.0 * SHOULDER + 4.0;
    for p in way.iter_mut() {
        let mut best = (f32::MAX, *p);
        for r in roads {
            for w in r.points.windows(2) {
                let (a, b) = (Vec2::new(w[0].x, w[0].z), Vec2::new(w[1].x, w[1].z));
                let ab = b - a;
                let s = ((*p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
                let q = a + ab * s;
                let d = p.distance(q);
                if d < best.0 {
                    best = (d, q);
                }
            }
        }
        if best.0 < NEAR {
            *p = p.lerp(best.1, 1.0 - smoothstep(ON, NEAR, best.0));
        }
    }
}

/// `way` without the points that double back (pulled onto a road behind them, say).
fn untangle(way: &[Vec2]) -> Vec<Vec2> {
    let mut out: Vec<Vec2> = Vec::with_capacity(way.len());
    for (k, &p) in way.iter().enumerate() {
        let n = out.len();
        let last = k + 1 == way.len();
        if n >= 2 && !last {
            let (a, b) = (out[n - 2], out[n - 1]);
            if p.distance(b) < 1.0
                || (p - b).normalize_or_zero().dot((b - a).normalize_or_zero()) < 0.3
            {
                continue;
            }
        }
        out.push(p);
    }
    out
}

/// `pts` with the points no further than `tolerance` from the line through their neighbours
/// left out (Douglas-Peucker). The first two and last two stay.
fn simplify(pts: &[Vec2], tolerance: f32) -> Vec<Vec2> {
    let n = pts.len();
    if n <= 4 {
        return pts.to_vec();
    }
    let mut keep = vec![false; n];
    for k in [0, 1, n - 2, n - 1] {
        keep[k] = true;
    }
    let mut stack = vec![(1usize, n - 2)];
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let (p, q) = (pts[a], pts[b]);
        let ab = q - p;
        let len2 = ab.length_squared().max(1e-6);
        let (mut worst, mut at) = (0.0, a);
        for (k, &c) in pts.iter().enumerate().take(b).skip(a + 1) {
            let s = ((c - p).dot(ab) / len2).clamp(0.0, 1.0);
            let d = c.distance(p + ab * s);
            if d > worst {
                worst = d;
                at = k;
            }
        }
        if worst > tolerance {
            keep[at] = true;
            stack.push((a, at));
            stack.push((at, b));
        }
    }
    pts.iter()
        .zip(&keep)
        .filter(|(_, k)| **k)
        .map(|(p, _)| *p)
        .collect()
}

/// One round of corner cutting (Chaikin's), cutting no more than `most` metres from a corner.
/// The ends stay where they are.
fn round_corners(pts: &[Vec2], most: f32) -> Vec<Vec2> {
    let n = pts.len();
    if n <= 2 {
        return pts.to_vec();
    }
    let mut out = vec![pts[0]];
    for k in 0..n - 1 {
        let (p, q) = (pts[k], pts[k + 1]);
        let len = p.distance(q).max(1e-3);
        let cut = (0.25 * len).min(most) / len;
        if k > 0 {
            out.push(p + (q - p) * cut);
        }
        if k + 2 < n {
            out.push(q + (p - q) * cut);
        }
    }
    out.push(pts[n - 1]);
    out
}

/// The road's heights along `way`: the land's, smoothed, kept above water, no steeper than
/// `MAX_GRADE`, and level with the pads and roads it crosses.
fn profile(t: &Terrain, pads: &[Pad], way: &[Vec2], near: &[u8]) -> Vec<Vec3> {
    let n = way.len();
    // (Where it runs along a road built before, that one's height.)
    let on_road = |p: Vec2| {
        let (i, j) = (
            ((p.x + HALF) / CELL).round() as usize,
            ((p.y + HALF) / CELL).round() as usize,
        );
        near[j.min(N - 1) * N + i.min(N - 1)] < (SHOULDER * 8.0) as u8
    };
    // Level with the pads (fixed), and with roads before (held: not smoothed, but eased to
    // the grade where they don't fit it).
    let on_pad: Vec<Option<f32>> = way
        .iter()
        .map(|p| {
            pads.iter()
                .zip(&t.pad_heights)
                .find(|(pd, _)| pd.center.distance(*p) < pd.radius)
                .map(|(_, &h)| h)
        })
        .collect();
    let held: Vec<bool> = way
        .iter()
        .zip(&on_pad)
        .map(|(p, pad)| pad.is_some() || on_road(*p))
        .collect();
    let mut h: Vec<f32> = way
        .iter()
        .zip(&on_pad)
        .map(|(p, pad)| pad.unwrap_or_else(|| t.height(p.x, p.y).max(t.water(p.x, p.y) + 0.8)))
        .collect();
    // Even out the bumps (about 40 m either way).
    const W: usize = 10;
    for _ in 0..3 {
        let prev = h.clone();
        for q in 0..n {
            if held[q] {
                continue;
            }
            let (lo, hi) = (q.saturating_sub(W), (q + W).min(n - 1));
            h[q] = prev[lo..=hi].iter().sum::<f32>() / (hi - lo + 1) as f32;
        }
    }
    // Never under water (the sea's floor is shallow: a causeway).
    for (q, p) in way.iter().enumerate() {
        if on_pad[q].is_none() {
            h[q] = h[q]
                .max(t.water(p.x, p.y) + 0.8)
                .max(SEA_FLOOR.max(SEA) + 0.8);
        }
    }
    // No steeper than the grade allows (from both ways, the pads holding).
    let g = MAX_GRADE * SPACING;
    for _ in 0..4 {
        for q in 1..n {
            if on_pad[q].is_none() {
                h[q] = h[q].clamp(h[q - 1] - g, h[q - 1] + g);
            }
        }
        for q in (0..n - 1).rev() {
            if on_pad[q].is_none() {
                h[q] = h[q].clamp(h[q + 1] - g, h[q + 1] + g);
            }
        }
    }
    way.iter()
        .zip(&h)
        .map(|(p, &y)| Vec3::new(p.x, y, p.y))
        .collect()
}

/// Grades the road into the land: flat to `SHOULDER`, banks beyond (wider where the cut or
/// fill is deeper), the pads left as they are and roads built before kept.
fn carve(
    t: &mut Terrain,
    pads: &[Pad],
    points: &[Vec3],
    near: &mut [u8],
    dist: &mut [f32],
    road_h: &mut [f32],
    touched: &mut Vec<usize>,
) {
    let reach = SHOULDER + MAX_BANK;
    let to_i = |v: f32| (v + HALF) / CELL;
    for w in points.windows(2) {
        let (a, b) = (Vec2::new(w[0].x, w[0].z), Vec2::new(w[1].x, w[1].z));
        let ab = b - a;
        let len2 = ab.length_squared().max(1e-6);
        let lo = a.min(b) - Vec2::splat(reach);
        let hi = a.max(b) + Vec2::splat(reach);
        let (i0, i1) = (
            to_i(lo.x).floor().max(0.0) as usize,
            (to_i(hi.x).ceil() as usize).min(N - 1),
        );
        let (j0, j1) = (
            to_i(lo.y).floor().max(0.0) as usize,
            (to_i(hi.y).ceil() as usize).min(N - 1),
        );
        for j in j0..=j1 {
            for i in i0..=i1 {
                let p = Vec2::new(i as f32 * CELL - HALF, j as f32 * CELL - HALF);
                let s = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
                let d = p.distance(a + ab * s);
                if d > reach {
                    continue;
                }
                let k = j * N + i;
                if dist[k].is_infinite() {
                    touched.push(k);
                }
                if d < dist[k] {
                    dist[k] = d;
                    road_h[k] = w[0].y + (w[1].y - w[0].y) * s;
                }
            }
        }
    }
    for &k in touched.iter() {
        let d = dist[k];
        let earlier = near[k] as f32 / 8.0;
        dist[k] = f32::INFINITY;
        let (i, j) = (k % N, k / N);
        let p = Vec2::new(i as f32 * CELL - HALF, j as f32 * CELL - HALF);
        near[k] = near[k].min((d * 8.0).min(255.0) as u8);
        // On an earlier road and nearer to it: that one's.
        if earlier < SHOULDER && earlier < d {
            continue;
        }
        if pads.iter().any(|pd| pd.center.distance(p) < pd.radius) {
            continue;
        }
        let h = t.heights[k];
        let hr = road_h[k];
        let bank = ((h - hr).abs() * 1.6).clamp(8.0, MAX_BANK);
        let w = 1.0 - smoothstep(SHOULDER, SHOULDER + bank, d);
        t.heights[k] = h + (hr - h) * w;
    }
    touched.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crate::worldgen::{ContinentDef, WorldDef, WorldParams};

    /// A continent of each contested theme.
    fn continents(seed: u64) -> Vec<ContinentDef> {
        let w = WorldDef::generate(WorldParams { seed, contested: 6 });
        Theme::CONTESTED
            .iter()
            .map(|th| {
                w.continents
                    .iter()
                    .find(|c| c.theme == *th)
                    .expect("each theme")
                    .clone()
            })
            .collect()
    }

    /// The share of the land (above the sea, off the pads) a ground vehicle can't climb.
    fn too_steep(t: &Terrain, def: &ContinentDef) -> f32 {
        let pads = def.pads();
        let (mut land, mut steep) = (0, 0);
        let mut z = -1600.0;
        while z < 1600.0 {
            let mut x = -1600.0;
            while x < 1600.0 {
                let p = Vec2::new(x, z);
                if t.height(x, z) > 1.0
                    && pads
                        .iter()
                        .all(|pd| pd.center.distance(p) > pd.radius + pd.blend)
                {
                    land += 1;
                    let dx = (t.height(x + 8.0, z) - t.height(x - 8.0, z)) / 16.0;
                    let dz = (t.height(x, z + 8.0) - t.height(x, z - 8.0)) / 16.0;
                    if (dx * dx + dz * dz).sqrt() > 0.6 {
                        steep += 1;
                    }
                }
                x += 16.0;
            }
            z += 16.0;
        }
        steep as f32 / land.max(1) as f32
    }

    #[test]
    fn roads_are_gentle_and_flat() {
        for def in continents(1).into_iter().chain(continents(2)) {
            let t = def.terrain();
            assert_eq!(t.roads.len(), def.lattice.len(), "{:?}", def.theme);
            let (mut worst_grade, mut worst_dip, mut worst_slope) = (0.0f32, 0.0f32, 0.0f32);
            for r in &t.roads {
                for w in r.points.windows(2) {
                    let run = Vec2::new(w[1].x - w[0].x, w[1].z - w[0].z).length();
                    worst_grade = worst_grade.max((w[1].y - w[0].y).abs() / run.max(0.5));
                }
                for (q, p) in r.points.iter().enumerate() {
                    let a = r.points[q.saturating_sub(1)];
                    let b = r.points[(q + 1).min(r.points.len() - 1)];
                    let along = Vec2::new(b.x - a.x, b.z - a.z).normalize_or_zero();
                    let across = along.perp();
                    // The land is the road's height along its middle, and gentle across it.
                    worst_dip = worst_dip.max((t.height(p.x, p.z) - p.y).abs());
                    // (Within a cell of its edge the land may slope down a switchback's bank.)
                    for s in [-HALF_WIDTH + CELL, 0.0, HALF_WIDTH - CELL] {
                        let e = Vec2::new(p.x, p.z) + across * s;
                        worst_slope = worst_slope.max(t.slope(e.x, e.y));
                    }
                    assert!(t.road_distance(p.x, p.z) < 3.0);
                }
            }
            println!(
                "{:?}: grade {worst_grade:.2}, middle {worst_dip:.2} m, slope {worst_slope:.2}",
                def.theme
            );
            // (A little steeper where a road joins another, and at junctions.)
            assert!(
                worst_grade <= MAX_GRADE + 0.08,
                "{:?} {worst_grade}",
                def.theme
            );
            assert!(worst_dip < 0.8, "{:?} {worst_dip}", def.theme);
            assert!(worst_slope < 0.45, "{:?} {worst_slope}", def.theme);
        }
    }

    #[test]
    fn relief_differs_by_theme() {
        let defs = continents(3);
        let steep: Vec<f32> = defs.iter().map(|d| too_steep(&d.terrain(), d)).collect();
        let of = |th: Theme| steep[Theme::CONTESTED.iter().position(|&x| x == th).unwrap()];
        // Nearly flat ...
        assert!(of(Theme::FungalMarsh) < 0.03 && of(Theme::SynthDunes) < 0.06);
        // ... to rugged, with much of the land too steep to drive.
        assert!(of(Theme::CircuitMesa) > of(Theme::SynthDunes));
        assert!(of(Theme::Ember) > of(Theme::SynthDunes));
        assert!(of(Theme::NeonJungle) > 0.15 && of(Theme::Glacier) > 0.18);
    }

    #[test]
    fn pads_stay_flat() {
        for def in continents(4) {
            let t = def.terrain();
            for (p, &h) in def.pads().iter().zip(&t.pad_heights) {
                for k in 0..16 {
                    let a = k as f32 / 16.0 * std::f32::consts::TAU;
                    for r in [0.0, 0.5, 1.0] {
                        // (Within a cell of the edge the land slopes into the blend.)
                        let q = p.center
                            + Vec2::new(a.cos(), a.sin()) * (p.radius * r).min(p.radius - 6.0);
                        let y = t.height(q.x, q.y);
                        assert!((y - h).abs() < 0.05, "{:?} {y} vs {h}", def.theme);
                    }
                }
            }
        }
    }

    #[test]
    fn same_seed_same_roads() {
        let (a, b) = (continents(5), continents(5));
        let (ta, tb) = (a[2].terrain(), b[2].terrain());
        assert_eq!(ta.roads, tb.roads);
        assert!(ta.heights == tb.heights);
    }

    #[test]
    #[ignore]
    fn picture() {
        // PS_PIC_DIR=... cargo test --release -p ps-sim picture -- --ignored: each theme's
        // land (steep in red), roads (yellow) and pads (cyan).
        let dir = std::env::var("PS_PIC_DIR").unwrap_or_else(|_| "/tmp".into());
        for def in continents(1) {
            let t = def.terrain();
            let pads = def.pads();
            let n = 1024usize;
            let mut img = format!("P6 {n} {n} 255\n").into_bytes();
            for j in 0..n {
                for i in 0..n {
                    let x = (i as f32 + 0.5) / n as f32 * 2.0 * HALF - HALF;
                    let z = (j as f32 + 0.5) / n as f32 * 2.0 * HALF - HALF;
                    let h = t.height(x, z);
                    let v = (40.0 + h.clamp(0.0, 300.0) * 0.6) as u8;
                    let mut c = if h < t.water(x, z) {
                        [20u8, 40, 120]
                    } else if t.slope(x, z) > 0.6 {
                        [v.saturating_add(90), v / 2, v / 2]
                    } else {
                        [v, v, v]
                    };
                    if t.road_distance(x, z) < HALF_WIDTH {
                        c = [255, 255, 0];
                    }
                    if pads
                        .iter()
                        .any(|p| p.center.distance(Vec2::new(x, z)) < p.radius * 0.6)
                    {
                        c = [0, 200, 255];
                    }
                    img.extend(c);
                }
            }
            std::fs::write(format!("{dir}/roads_{:?}.ppm", def.theme), img).unwrap();
        }
    }
}
