//! Canyon caves (the Hollow Reach theme): a rock plateau raised over the land everywhere but
//! along the roads and on the pads, so every road runs in a steep-walled canyon and every base
//! and warp gate stands in a sinkhole (`raise`). Rock slabs roof the canyons in stretches,
//! skylights between them and open where they come out by a pad, and roof each base's sinkhole
//! in a ring round its edge, the middle open to the sky (`build`). Ziplines run from the bases'
//! roofs up to the rim (`crate::zip`).
//!
//! The roofs are flat-topped prisms just under the plateau's surface: walked on from above
//! like the land, ceilings from below (bodies and aircraft stop under them,
//! `World::ceiling`), and they stop shots and sight. The land under them isn't changed.

use glam::{Vec2, Vec3};

use crate::base::BaseLayout;
use crate::collide::Collider;
use crate::noise::{self, smoothstep};
use crate::roads::NEAR_MAX;
use crate::terrain::{CELL, COAST, HALF, N, Pad, Terrain};
use crate::worldgen::{BaseKind, ContinentDef};

/// A canyon's half width at its floor (from the road's middle; the road and its shoulders fit
/// in it), and how far its walls take to rise.
pub const CANYON: f32 = 17.0;
pub const WALL: f32 = 5.0;
/// How far a sinkhole's wall stands off its pad's edge, and how far it takes to rise.
pub const SINK: f32 = 2.0;
pub const SINK_WALL: f32 = 8.0;
/// A roof slab's thickness, and the least headroom under one (over the road; over a base).
pub const ROOF: f32 = 4.5;
pub const HEADROOM: f32 = 14.0;
pub const BASE_HEADROOM: f32 = 24.0;
/// The canyons' roofs: in pieces this long, reaching this far out from the road's middle;
/// roofed stretches and skylights between them (m); and left open this far beyond a pad's
/// edge (its canyon's mouth).
const PIECE: f32 = 16.0;
const REACH: f32 = CANYON + WALL + 6.0;
const ROOFED: (f32, f32) = (120.0, 260.0);
const SKYLIGHT: (f32, f32) = (40.0, 90.0);
const MOUTH: f32 = 30.0;
/// A sinkhole's ring roof: from this share of the pad's radius out past its edge, in sectors.
const RING_IN: f32 = 0.62;
const RING_OUT: f32 = 14.0;
const SECTORS: usize = 16;

/// How high the plateau stands over the land at (x, z) (before the canyons and sinkholes
/// are cut out of it).
fn rise(seed: u32, x: f32, z: f32) -> f32 {
    let r = (x * x + z * z).sqrt();
    // (Down to the beaches round the coast.)
    let coast = 1.0 - smoothstep(COAST - 420.0, COAST - 220.0, r);
    let h = 30.0
        + 12.0 * noise::fbm(x / 300.0, z / 300.0, seed ^ 0xca1, 3, 0.5)
        + 10.0 * noise::ridged(x / 120.0, z / 120.0, seed ^ 0xca2, 3, 0.5);
    h * coast
}

/// How much of the plateau stands at (x, z) (0 in a canyon or sinkhole, 1 on top), its road
/// distance `road` (m).
fn cover(seed: u32, pads: &[Pad], x: f32, z: f32, road: f32) -> f32 {
    let wob = 3.0 * noise::fbm(x / 60.0, z / 60.0, seed ^ 0xca3, 2, 0.5);
    let mut k = smoothstep(CANYON + wob, CANYON + WALL + wob, road);
    let q = Vec2::new(x, z);
    for p in pads {
        let d = q.distance(p.center) - p.radius;
        if d < SINK + SINK_WALL {
            k = k.min(smoothstep(SINK, SINK + SINK_WALL, d));
        }
    }
    k
}

/// Raises the plateau over the land (its roads and pads already in it).
pub fn raise(t: &mut Terrain, pads: &[Pad], seed: u32) {
    for j in 0..N {
        let z = j as f32 * CELL - HALF;
        for i in 0..N {
            let x = i as f32 * CELL - HALF;
            let road = if t.road_near.is_empty() {
                NEAR_MAX
            } else {
                t.road_near[j * N + i] as f32 / 8.0
            };
            let k = cover(seed, pads, x, z, road);
            if k > 0.0 {
                t.heights[j * N + i] += rise(seed, x, z) * k;
            }
        }
    }
}

/// A continent's caves: the roofs over its canyons and sinkholes.
#[derive(Clone, Debug, Default)]
pub struct Caves {
    pub roofs: Vec<Collider>,
}

/// The lowest of the land's heights at `pts`.
fn lowest(t: &Terrain, pts: &[Vec2]) -> f32 {
    pts.iter()
        .map(|p| t.height(p.x, p.y))
        .fold(f32::MAX, f32::min)
}

/// The roads' points, in 32 m cells (to find those under a roof).
struct RoadGrid(std::collections::HashMap<(i32, i32), Vec<Vec3>>);

impl RoadGrid {
    fn new(t: &Terrain) -> RoadGrid {
        let mut m: std::collections::HashMap<(i32, i32), Vec<Vec3>> = Default::default();
        for r in &t.roads {
            for p in &r.points {
                m.entry(Self::cell(p.x, p.z)).or_default().push(*p);
            }
        }
        RoadGrid(m)
    }

    fn cell(x: f32, z: f32) -> (i32, i32) {
        ((x / 32.0).floor() as i32, (z / 32.0).floor() as i32)
    }

    /// The highest road under the convex polygon `poly` (a little round it too).
    fn highest(&self, poly: &[Vec2]) -> Option<f32> {
        let c = Collider::prism(poly, 0.0, 1.0);
        let (lo, hi) = c.bounds();
        let (a, b) = (
            Self::cell(lo.x - 8.0, lo.z - 8.0),
            Self::cell(hi.x + 8.0, hi.z + 8.0),
        );
        let mut best: Option<f32> = None;
        for j in a.1..=b.1 {
            for i in a.0..=b.0 {
                for p in self.0.get(&(i, j)).into_iter().flatten() {
                    let Collider::Prism { n, pts, .. } = c else {
                        continue;
                    };
                    if Collider::prism_push(n, &pts, Vec2::new(p.x, p.z), 8.0).is_some()
                        && best.is_none_or(|y| p.y > y)
                    {
                        best = Some(p.y);
                    }
                }
            }
        }
        best
    }
}

/// A flat roof over the convex polygon `poly`, just under the land at `rim` (where it meets
/// the land), if there's `headroom` under it over `floor`.
fn roof_over(
    t: &Terrain,
    poly: &[Vec2],
    rim: &[Vec2],
    floor: f32,
    headroom: f32,
) -> Option<Collider> {
    let top = lowest(t, rim) - 0.3;
    let bottom = top - ROOF;
    (bottom - floor >= headroom).then(|| Collider::prism(poly, bottom, top))
}

/// The roofs: over the canyons in stretches, and a ring round each base's sinkhole.
pub fn build(def: &ContinentDef, t: &Terrain, layouts: &[BaseLayout]) -> Caves {
    let mut caves = Caves::default();
    if !def.theme.caves() {
        return caves;
    }
    let pads = def.pads();
    let grid = RoadGrid::new(t);
    let mut rng = noise::Seq::new(def.terrain_seed as u64, 0xca5e);
    for road in &t.roads {
        let pts = &road.points;
        if pts.len() < 2 {
            continue;
        }
        // Along the road: roofed, then a skylight, by turns (starting at a random point of
        // the turn).
        let mut roofed = rng.chance(0.6);
        let mut left = rng.range(0.0, ROOFED.1);
        let mut s = 0.0;
        let mut i = 0;
        while i + 1 < pts.len() {
            // A piece: from point i to the first point PIECE further along.
            let a = pts[i];
            let mut j = i + 1;
            let mut len = 0.0;
            while j < pts.len() {
                len = Vec2::new(pts[j].x - a.x, pts[j].z - a.z).length();
                if len >= PIECE {
                    break;
                }
                j += 1;
            }
            let j = j.min(pts.len() - 1);
            let b = pts[j];
            s += len;
            left -= len;
            if left <= 0.0 {
                roofed = !roofed;
                left = if roofed {
                    rng.range(ROOFED.0, ROOFED.1)
                } else {
                    rng.range(SKYLIGHT.0, SKYLIGHT.1)
                };
            }
            i = j;
            let (a2, b2) = (Vec2::new(a.x, a.z), Vec2::new(b.x, b.z));
            let along = (b2 - a2).normalize_or_zero();
            if !roofed || along == Vec2::ZERO {
                continue;
            }
            // Open by the pads: the canyons' mouths.
            let mouth = pads.iter().any(|p| {
                let r = p.radius + SINK + SINK_WALL + MOUTH;
                a2.distance(p.center) < r || b2.distance(p.center) < r
            });
            if mouth {
                continue;
            }
            let side = along.perp() * REACH;
            let (a2, b2) = (a2 - along * 2.0, b2 + along * 2.0);
            let poly = [a2 - side, b2 - side, b2 + side, a2 + side];
            // (Into the land on both sides: its corners and the middles of its long sides.)
            let rim = [
                poly[0],
                poly[1],
                poly[2],
                poly[3],
                (poly[0] + poly[1]) / 2.0,
                (poly[2] + poly[3]) / 2.0,
            ];
            // (Over whichever road is highest under it: they meet and cross.)
            let floor = grid.highest(&poly).unwrap_or(a.y).max(a.y.max(b.y));
            if let Some(c) = roof_over(t, &poly, &rim, floor, HEADROOM) {
                caves.roofs.push(c);
            }
        }
        let _ = s;
    }
    // The sinkholes' rings (not round the towers: their pits are small).
    for (b, site) in def.bases.iter().enumerate() {
        if site.kind == BaseKind::Tower {
            continue;
        }
        let p = &pads[b];
        let floor = layouts.get(b).map_or(t.pad_heights[b], |l| l.center.y);
        let (r0, r1) = (p.radius * RING_IN, p.radius + RING_OUT);
        for k in 0..SECTORS {
            let a0 = k as f32 / SECTORS as f32 * std::f32::consts::TAU;
            let a1 = (k + 1) as f32 / SECTORS as f32 * std::f32::consts::TAU;
            let dir = |a: f32| Vec2::new(a.cos(), a.sin());
            let poly = [
                p.center + dir(a0) * r0,
                p.center + dir(a0) * r1,
                p.center + dir(a1) * r1,
                p.center + dir(a1) * r0,
            ];
            // (Just under the rim: the outer edge's land.)
            let rim = [poly[1], poly[2], (poly[1] + poly[2]) / 2.0];
            let top = lowest(t, &rim) - 0.3;
            let bottom = top - ROOF;
            let floor = grid.highest(&poly).map_or(floor, |y| y.max(floor));
            if bottom - floor >= BASE_HEADROOM {
                caves.roofs.push(Collider::prism(&poly, bottom, top));
            }
        }
    }
    caves
}

/// Whether there's a cave roof over `p` within `up` metres.
pub fn roofed(caves: &Caves, p: Vec3, up: f32) -> bool {
    caves.roofs.iter().any(|c| {
        let (lo, hi) = c.bounds();
        p.x >= lo.x
            && p.x <= hi.x
            && p.z >= lo.z
            && p.z <= hi.z
            && lo.y >= p.y
            && lo.y < p.y + up
            && c.top_at(p.x, p.z).is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::ContinentMap;
    use crate::nav::{GROUND_VEHICLE, Nav};
    use crate::worldgen::{WorldDef, WorldParams};
    use std::sync::Arc;

    fn cave_map(seed: u64) -> ContinentMap {
        let w = WorldDef::generate(WorldParams { seed, contested: 6 });
        let cd = w
            .continents
            .iter()
            .find(|c| c.theme.caves())
            .expect("a cave continent in every world")
            .clone();
        ContinentMap::build(Arc::new(cd), false)
    }

    #[test]
    fn every_world_has_caves() {
        for seed in 0..30 {
            for contested in [4, 6, 9] {
                let w = WorldDef::generate(WorldParams { seed, contested });
                assert!(
                    w.continents.iter().any(|c| c.theme.caves()),
                    "seed {seed} ({contested})"
                );
            }
        }
    }

    #[test]
    fn caves_are_roofed_with_headroom_and_bases_stand_in_the_open() {
        let m = cave_map(11);
        let caves = &m.caves;
        assert!(caves.roofs.len() > 40, "{} roofs", caves.roofs.len());
        // Over the roads, the roofs leave room to drive.
        for r in &m.terrain.roads {
            for p in &r.points {
                assert!(!roofed(caves, *p, HEADROOM - 0.5), "{p}: roof too low");
            }
        }
        // Spawns, consoles and points under the open sky, or far below a roof.
        for l in &m.bases {
            let spots = l
                .spawns
                .iter()
                .map(|s| s.0)
                .chain(l.console.map(|c| c.0))
                .chain(l.points.iter().map(|p| p.stand));
            for s in spots {
                assert!(!roofed(caves, s, 20.0), "{s} under a roof");
            }
        }
        // The plateau stands well over the pads round about.
        let mut high = 0;
        let mut all = 0;
        for (i, p) in m.def.pads().iter().enumerate() {
            for k in 0..16 {
                let a = k as f32 / 16.0 * std::f32::consts::TAU;
                let q = p.center + Vec2::new(a.cos(), a.sin()) * (p.radius + 24.0);
                if m.terrain.road_distance(q.x, q.y) < NEAR_MAX - 0.5 {
                    continue;
                }
                all += 1;
                if m.terrain.height(q.x, q.y) > m.terrain.pad_heights[i] + 15.0 {
                    high += 1;
                }
            }
        }
        assert!(high * 10 > all * 7, "{high} of {all} round the pads high");
    }

    #[test]
    fn vehicles_get_out_of_the_towers_pits() {
        // (The canyons along the lattice: `nav`'s test of every theme.)
        for seed in [11, 23] {
            let m = cave_map(seed);
            let nav = Nav::build(&m, &m.world);
            // Outside a base's gateway facing `to` (where its road starts).
            let outside = |b: usize, to: Vec2| -> Vec3 {
                let site = &m.def.bases[b];
                let d = (to - site.pos).normalize();
                let (at, out) = crate::base::gateways(site)
                    .into_iter()
                    .max_by(|x, y| x.1.dot(d).total_cmp(&y.1.dot(d)))
                    .unwrap();
                let q = at + out * 10.0;
                Vec3::new(q.x, m.terrain.surface(q.x, q.y), q.y)
            };
            for (t, site) in m.def.bases.iter().enumerate() {
                if site.kind != BaseKind::Tower {
                    continue;
                }
                let base = (0..m.def.bases.len())
                    .filter(|&b| m.def.bases[b].kind != BaseKind::Tower)
                    .min_by(|&a, &b| {
                        let d = |i: usize| m.def.bases[i].pos.distance(site.pos);
                        d(a).total_cmp(&d(b))
                    })
                    .unwrap();
                // From the tower's door to outside its base.
                let door = m.bases[t].to_console[0];
                let to = outside(base, site.pos);
                assert!(
                    nav.route(door, to, GROUND_VEHICLE).is_some(),
                    "seed {seed}: no way from tower {t} to base {base}"
                );
            }
        }
    }
}
