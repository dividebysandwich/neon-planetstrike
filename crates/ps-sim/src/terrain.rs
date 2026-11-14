//! A continent's land: a heightfield over a square `MAP_SIZE` metres across, centred on the
//! origin, sampled every `CELL` metres. An island rising from a shallow sea (wadeable: the sea
//! floor is never deeper than `SEA_FLOOR`), flattened under bases and warp gates, with roads
//! (`roads.rs`) graded into it between them. How rugged it is depends on the theme: crags
//! too steep to drive over keep away from the pads.

use glam::{Vec2, Vec3};

use crate::noise::{self, smoothstep};
use crate::roads::{self, Road, RoadLink};
use crate::theme::Theme;

pub const MAP_SIZE: f32 = 4096.0;
pub const HALF: f32 = MAP_SIZE / 2.0;
pub const CELL: f32 = 4.0;
/// Samples along each side.
pub const N: usize = (MAP_SIZE / CELL) as usize + 1;
/// The sea's surface and floor.
pub const SEA: f32 = 0.0;
pub const SEA_FLOOR: f32 = -2.5;
/// The coast's distance from the middle (the island's land stays within it).
pub const COAST: f32 = 1880.0;
/// Nobody goes beyond this from the middle along x or z (a forcefield at the map's edge).
pub const BOUND: f32 = 2000.0;

/// A flat area: bases and warp gates stand on these.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pad {
    pub center: Vec2,
    /// Flat out to this radius, blending into the land over `blend` beyond.
    pub radius: f32,
    pub blend: f32,
}

#[derive(Clone)]
pub struct Terrain {
    pub theme: Theme,
    pub heights: Vec<f32>,
    /// Each pad's height (the same order as given).
    pub pad_heights: Vec<f32>,
    /// Lakes: their water stands at `level` within `radius` of `center`.
    pub lakes: Vec<Lake>,
    pub roads: Vec<Road>,
    /// Each sample's distance to the nearest road's middle, in eighths of a metre (255: that
    /// far or further).
    pub road_near: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lake {
    pub center: Vec2,
    pub radius: f32,
    pub level: f32,
}

/// A mountain raised over the land (a volcano with a crater on top).
#[derive(Clone, Copy, Debug)]
struct Peak {
    center: Vec2,
    radius: f32,
    height: f32,
    crater: bool,
}

/// The theme's mountains and lakes: how many, how big.
#[allow(clippy::type_complexity)]
fn features(theme: Theme) -> (usize, (f32, f32), (f32, f32), usize, (f32, f32)) {
    // (peaks, peak radius range, peak height range, lakes, lake radius range)
    match theme {
        Theme::Glacier => (5, (260.0, 420.0), (180.0, 320.0), 3, (60.0, 120.0)),
        Theme::NeonJungle => (4, (220.0, 380.0), (120.0, 220.0), 4, (60.0, 140.0)),
        Theme::Ember => (3, (260.0, 380.0), (160.0, 240.0), 2, (50.0, 90.0)),
        Theme::CircuitMesa => (2, (300.0, 420.0), (90.0, 150.0), 2, (60.0, 110.0)),
        Theme::SynthDunes => (2, (300.0, 450.0), (80.0, 140.0), 1, (50.0, 80.0)),
        Theme::FungalMarsh => (1, (250.0, 350.0), (60.0, 110.0), 7, (60.0, 160.0)),
        Theme::Haven => (2, (300.0, 400.0), (100.0, 160.0), 2, (60.0, 110.0)),
    }
}

impl std::fmt::Debug for Terrain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Terrain")
            .field("theme", &self.theme)
            .finish()
    }
}

/// The land's height before pads are flattened into it, with the theme's crags at `rugged`
/// (0..1: none near pads, all of them away from them).
pub fn raw_height(theme: Theme, seed: u32, x: f32, z: f32, rugged: f32) -> f32 {
    let st = theme.style();
    // Twist the shapes a little.
    let wx = x + st.warp * noise::fbm(x / 900.0, z / 900.0, seed ^ 0x51, 3, 0.5);
    let wz = z + st.warp * noise::fbm(x / 900.0 + 31.7, z / 900.0 - 11.3, seed ^ 0x52, 3, 0.5);
    let hills = noise::fbm(wx / st.hill_scale, wz / st.hill_scale, seed ^ 0x10, 5, 0.48);
    let mut h = st.base + st.hill_amp * (hills * 0.5 + 0.5);
    if st.ridge_amp > 0.0 {
        // Mountains only where a slow noise says so.
        let region = smoothstep(
            -0.05,
            0.45,
            noise::fbm(wx / 1600.0, wz / 1600.0, seed ^ 0x20, 2, 0.5),
        );
        let r = noise::ridged(
            wx / st.ridge_scale,
            wz / st.ridge_scale,
            seed ^ 0x21,
            5,
            0.5,
        );
        h += st.ridge_amp * region * r * r;
    }
    if st.dune_amp > 0.0 {
        let dir = Vec2::new(0.8, 0.6);
        let along = (wx * dir.x + wz * dir.y) / st.dune_len;
        let wobble = noise::perlin(wx / 300.0, wz / 300.0, seed ^ 0x30);
        let phase = along + wobble * 1.5;
        // Asymmetric dune profile: a gentle back slope and a steeper face.
        let f = phase - phase.floor();
        let profile = if f < 0.75 { f / 0.75 } else { (1.0 - f) / 0.25 };
        h += st.dune_amp * profile * profile;
    }
    if st.crater_depth > 0.0 {
        let (d, hash) = noise::cells(wx / st.crater_scale, wz / st.crater_scale, seed ^ 0x40);
        if noise::unit(hash) < 0.55 {
            let bowl = 1.0 - smoothstep(0.0, 0.32, d);
            let rim = smoothstep(0.22, 0.32, d) * (1.0 - smoothstep(0.32, 0.45, d));
            h += st.crater_depth * (rim * 0.35 - bowl);
        }
    }
    if st.crag_cover > 0.0 && rugged > 0.0 {
        // Highlands where a slow noise says so (about `crag_cover` of the land): steep ridged
        // crags with sheer flanks.
        let at = 0.36 * (1.0 - 2.0 * st.crag_cover);
        let region = smoothstep(
            at - 0.08,
            at + 0.08,
            noise::fbm(wx / 1100.0, wz / 1100.0, seed ^ 0x70, 3, 0.5),
        );
        if region > 0.0 {
            let r = noise::ridged(wx / st.crag_scale, wz / st.crag_scale, seed ^ 0x71, 5, 0.55);
            h += st.crag_amp * region * rugged * r.powf(1.5);
        }
    }
    if st.terrace > 0.0 {
        let t = h / st.terrace;
        let k = t.floor();
        let f = t - k;
        // Mostly flat steps with steep risers.
        h = (k + smoothstep(0.7, 1.0, f)) * st.terrace;
    }
    // The island: land falls into the sea near the coast (a wobbly circle).
    let r = (x * x + z * z).sqrt();
    let wob = noise::fbm(x / 500.0, z / 500.0, seed ^ 0x60, 3, 0.5) * 140.0;
    let coast = 1.0 - smoothstep(COAST - 260.0, COAST, r + wob);
    SEA_FLOOR + (h.max(1.5) - SEA_FLOOR) * coast
}

impl Terrain {
    /// The land of a continent of `theme` from `seed`, flattened under `pads`, with roads along
    /// `links`.
    pub fn generate(theme: Theme, seed: u32, pads: &[Pad], links: &[RoadLink]) -> Terrain {
        // Crags only well away from the pads (towers' less so).
        let rugged = |x: f32, z: f32| -> f32 {
            let q = Vec2::new(x, z);
            pads.iter().fold(1.0f32, |k, p| {
                let calm = if p.radius < 40.0 { 50.0 } else { 110.0 };
                let edge = p.radius + p.blend;
                k.min(smoothstep(edge, edge + calm, q.distance(p.center)))
            })
        };
        let pad_heights: Vec<f32> = pads
            .iter()
            .map(|p| {
                // The pad at about the land's mean height there (never under the sea).
                let mut sum = 0.0;
                let k = 8;
                for i in 0..k {
                    let a = i as f32 / k as f32 * std::f32::consts::TAU;
                    let q = p.center + Vec2::new(a.cos(), a.sin()) * p.radius * 0.7;
                    sum += raw_height(theme, seed, q.x, q.y, 0.0);
                }
                let mid = raw_height(theme, seed, p.center.x, p.center.y, 0.0);
                ((sum / k as f32 + mid) * 0.5).max(4.0)
            })
            .collect();
        // Mountains and lakes, away from the pads (and each other).
        let (n_peaks, pr, ph, n_lakes, lr) = features(theme);
        let mut rng = crate::noise::Seq::new(seed as u64, 0x7e4a);
        let clear = |c: Vec2, r: f32| {
            c.length() + r < COAST - 200.0
                && pads
                    .iter()
                    .all(|p| p.center.distance(c) > p.radius + p.blend + r)
        };
        let mut peaks: Vec<Peak> = Vec::new();
        for _ in 0..400 {
            if peaks.len() >= n_peaks {
                break;
            }
            let c = Vec2::new(rng.range(-1500.0, 1500.0), rng.range(-1500.0, 1500.0));
            let radius = rng.range(pr.0, pr.1);
            if !clear(c, radius * 1.3)
                || peaks
                    .iter()
                    .any(|p| p.center.distance(c) < p.radius + radius)
            {
                continue;
            }
            peaks.push(Peak {
                center: c,
                radius,
                height: rng.range(ph.0, ph.1),
                crater: theme == Theme::Ember,
            });
        }
        let peak = |x: f32, z: f32| -> f32 {
            let mut add = 0.0;
            for p in &peaks {
                let d = Vec2::new(x, z).distance(p.center) / p.radius;
                if d >= 1.0 {
                    continue;
                }
                // A bell, roughened by ridges; volcanoes hollowed at the top.
                let bell = (1.0 - d * d).powi(2);
                let rough = 0.75 + 0.5 * noise::ridged(x / 120.0, z / 120.0, seed ^ 0x99, 4, 0.5);
                let mut h = p.height * bell * rough;
                if p.crater && d < 0.18 {
                    h -= p.height * 0.35 * (1.0 - d / 0.18);
                }
                add += h;
            }
            add
        };
        let mut lakes: Vec<Lake> = Vec::new();
        for _ in 0..400 {
            if lakes.len() >= n_lakes {
                break;
            }
            let c = Vec2::new(rng.range(-1400.0, 1400.0), rng.range(-1400.0, 1400.0));
            let radius = rng.range(lr.0, lr.1);
            if !clear(c, radius + 40.0)
                || lakes
                    .iter()
                    .any(|l| l.center.distance(c) < l.radius + radius + 80.0)
                || peaks.iter().any(|p| p.center.distance(c) < p.radius * 0.8)
            {
                continue;
            }
            // The water just below the lowest point of its shore, so it never spills.
            let mut level = f32::MAX;
            for k in 0..24 {
                let a = k as f32 / 24.0 * std::f32::consts::TAU;
                let q = c + Vec2::new(a.cos(), a.sin()) * radius;
                level =
                    level.min(raw_height(theme, seed, q.x, q.y, rugged(q.x, q.y)) + peak(q.x, q.y));
            }
            level -= 0.4;
            if level < SEA + 1.5 {
                continue;
            }
            lakes.push(Lake {
                center: c,
                radius,
                level,
            });
        }
        let mut heights = vec![0.0; N * N];
        for j in 0..N {
            let z = j as f32 * CELL - HALF;
            for i in 0..N {
                let x = i as f32 * CELL - HALF;
                let mut h = raw_height(theme, seed, x, z, rugged(x, z)) + peak(x, z);
                for l in &lakes {
                    let d = Vec2::new(x, z).distance(l.center);
                    if d < l.radius {
                        // A basin, 2.5 m deep in the middle.
                        let k = d / l.radius;
                        h = h.min(l.level - 2.5 * (1.0 - k * k) - 0.2);
                    }
                }
                for (p, &ph) in pads.iter().zip(&pad_heights) {
                    let d = Vec2::new(x, z).distance(p.center);
                    if d < p.radius + p.blend {
                        let k = 1.0 - smoothstep(p.radius, p.radius + p.blend, d);
                        h = h + (ph - h) * k;
                    }
                }
                heights[j * N + i] = h;
            }
        }
        let mut t = Terrain {
            theme,
            heights,
            pad_heights,
            lakes,
            roads: Vec::new(),
            road_near: Vec::new(),
        };
        let (roads, near) = roads::build(&mut t, pads, links);
        t.roads = roads;
        t.road_near = near;
        t
    }

    /// How far (x, z) is from the nearest road's middle (32 m: that far or further).
    pub fn road_distance(&self, x: f32, z: f32) -> f32 {
        if self.road_near.is_empty() {
            return roads::NEAR_MAX;
        }
        let gx = ((x + HALF) / CELL).clamp(0.0, (N - 1) as f32 - 1e-3);
        let gz = ((z + HALF) / CELL).clamp(0.0, (N - 1) as f32 - 1e-3);
        let (i, j) = (gx.floor() as usize, gz.floor() as usize);
        let (fx, fz) = (gx - i as f32, gz - j as f32);
        let at = |i: usize, j: usize| self.road_near[j * N + i] as f32 / 8.0;
        let a = at(i, j) + (at(i + 1, j) - at(i, j)) * fx;
        let b = at(i, j + 1) + (at(i + 1, j + 1) - at(i, j + 1)) * fx;
        a + (b - a) * fz
    }

    /// The sample at grid (i, j)'s distance to the nearest road (as `road_distance`).
    pub fn road_at(&self, i: i32, j: i32) -> f32 {
        if self.road_near.is_empty() {
            return roads::NEAR_MAX;
        }
        let i = i.clamp(0, N as i32 - 1) as usize;
        let j = j.clamp(0, N as i32 - 1) as usize;
        self.road_near[j * N + i] as f32 / 8.0
    }

    /// The water's surface at (x, z): a lake's level, or the sea's.
    pub fn water(&self, x: f32, z: f32) -> f32 {
        let p = Vec2::new(x, z);
        self.lakes
            .iter()
            .find(|l| l.center.distance(p) < l.radius)
            .map_or(SEA, |l| l.level)
    }

    /// The sample at grid (i, j), clamped to the grid.
    #[inline]
    pub fn at(&self, i: i32, j: i32) -> f32 {
        let i = i.clamp(0, N as i32 - 1) as usize;
        let j = j.clamp(0, N as i32 - 1) as usize;
        self.heights[j * N + i]
    }

    /// The land's height at (x, z) (bilinear between the samples, split along the cells'
    /// diagonal as the client's mesh is, so feet stand on what is drawn).
    pub fn height(&self, x: f32, z: f32) -> f32 {
        let gx = ((x + HALF) / CELL).clamp(0.0, (N - 1) as f32 - 1e-3);
        let gz = ((z + HALF) / CELL).clamp(0.0, (N - 1) as f32 - 1e-3);
        let (i, j) = (gx.floor() as i32, gz.floor() as i32);
        let (fx, fz) = (gx - i as f32, gz - j as f32);
        let h00 = self.at(i, j);
        let h10 = self.at(i + 1, j);
        let h01 = self.at(i, j + 1);
        let h11 = self.at(i + 1, j + 1);
        // Two triangles: (00, 10, 11) and (00, 11, 01).
        if fx >= fz {
            h00 + (h10 - h00) * fx + (h11 - h10) * fz
        } else {
            h00 + (h11 - h01) * fx + (h01 - h00) * fz
        }
    }

    /// The ground's height including the sea's surface (what a body standing there wades in).
    pub fn surface(&self, x: f32, z: f32) -> f32 {
        self.height(x, z).max(self.water(x, z))
    }

    /// The land's normal at (x, z).
    pub fn normal(&self, x: f32, z: f32) -> Vec3 {
        let e = CELL;
        let dx = self.height(x + e, z) - self.height(x - e, z);
        let dz = self.height(x, z + e) - self.height(x, z - e);
        Vec3::new(-dx, 2.0 * e, -dz).normalize()
    }

    /// Rise over run of the land at (x, z).
    pub fn slope(&self, x: f32, z: f32) -> f32 {
        let n = self.normal(x, z);
        (1.0 - n.y * n.y).max(0.0).sqrt() / n.y.max(1e-3)
    }

    /// The first point where the segment `a`..`b` goes into the land: fraction along it and the
    /// point.
    pub fn ray(&self, a: Vec3, b: Vec3) -> Option<(f32, Vec3)> {
        let d = b - a;
        let len = d.length();
        if len < 1e-6 {
            return None;
        }
        let steps = ((len / (CELL * 0.5)).ceil() as usize).max(1);
        let above = |p: Vec3| p.y - self.height(p.x, p.z);
        let mut prev_t = 0.0;
        let mut prev = above(a);
        if prev < 0.0 {
            return Some((0.0, a));
        }
        for s in 1..=steps {
            let t = s as f32 / steps as f32;
            let p = a + d * t;
            let h = above(p);
            if h < 0.0 {
                // Refine between the last point above and this one.
                let (mut lo, mut hi) = (prev_t, t);
                for _ in 0..8 {
                    let m = (lo + hi) * 0.5;
                    if above(a + d * m) < 0.0 {
                        hi = m;
                    } else {
                        lo = m;
                    }
                }
                let _ = prev;
                return Some((hi, a + d * hi));
            }
            prev_t = t;
            prev = h;
        }
        None
    }

    /// Grid coordinates of (x, z) (for the client's map and meshes).
    pub fn grid_of(x: f32, z: f32) -> (f32, f32) {
        ((x + HALF) / CELL, (z + HALF) / CELL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn island_with_flat_pads() {
        let pads = [Pad {
            center: Vec2::new(300.0, -200.0),
            radius: 80.0,
            blend: 60.0,
        }];
        for theme in Theme::CONTESTED {
            let t = Terrain::generate(theme, 99, &pads, &[]);
            // The sea at the map's edge, land in the middle.
            assert!(
                t.height(HALF - 10.0, HALF - 10.0) <= SEA_FLOOR + 0.01,
                "{theme:?}"
            );
            assert!(t.height(0.0, 0.0) > SEA, "{theme:?}");
            // Flat on the pad.
            let ph = t.pad_heights[0];
            for (dx, dz) in [(0.0, 0.0), (50.0, 10.0), (-30.0, -60.0)] {
                let h = t.height(300.0 + dx, -200.0 + dz);
                assert!((h - ph).abs() < 0.05, "{theme:?} {h} vs {ph}");
            }
            let mut hi = f32::MIN;
            for i in 0..200 {
                let x = (i as f32 * 37.0) % 3000.0 - 1500.0;
                let z = (i as f32 * 91.0) % 3000.0 - 1500.0;
                hi = hi.max(t.height(x, z));
            }
            assert!(hi < 600.0, "{theme:?} {hi}");
        }
    }

    #[test]
    fn rays_hit_the_ground() {
        let t = Terrain::generate(Theme::NeonJungle, 5, &[], &[]);
        let h = t.height(10.0, 10.0);
        let (f, p) = t
            .ray(
                Vec3::new(10.0, h + 50.0, 10.0),
                Vec3::new(10.0, h - 50.0, 10.0),
            )
            .unwrap();
        assert!((f - 0.5).abs() < 0.01 && (p.y - h).abs() < 0.2, "{f} {p}");
        assert!(
            t.ray(Vec3::new(0.0, 900.0, 0.0), Vec3::new(50.0, 900.0, 0.0))
                .is_none()
        );
    }
}
