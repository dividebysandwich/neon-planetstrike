//! Finding the way over a continent: a coarse grid over the land (16 m cells) with the ground's
//! height and what stands there (base walls, buildings, rocks), searched with A*. Steep steps
//! cost more and past a mover's climb can't be taken; water is slow going. Bots plan with it
//! and follow the waypoints; the last stretch (into a base, to a console) goes by the base's own
//! paths.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use glam::{Vec2, Vec3};

use crate::collide::World;
use crate::map::ContinentMap;
use crate::terrain::BOUND;

pub const CELL: f32 = 16.0;
const N: usize = (2.0 * BOUND / CELL) as usize;

/// How a mover gets about.
#[derive(Clone, Copy, Debug)]
pub struct Mode {
    /// Steepest rise over run it can take.
    pub climb: f32,
    /// Cost factor in water.
    pub water: f32,
    /// Squeezes through blocked cells' edges (infantry): costly instead of impossible.
    pub squeeze: bool,
}

pub const INFANTRY: Mode = Mode {
    climb: 1.1,
    water: 2.0,
    squeeze: true,
};

pub const GROUND_VEHICLE: Mode = Mode {
    climb: 0.55,
    water: 3.0,
    squeeze: false,
};

pub struct Nav {
    /// Ground (or water surface) height per cell.
    height: Vec<f32>,
    /// Something stands there (walls, buildings, big rocks).
    blocked: Vec<bool>,
    wet: Vec<bool>,
    /// Base gateways: the cell, the gateway's middle and the wall's normal.
    portals: Vec<(usize, Vec3, Vec3)>,
}

fn cell_of(p: Vec2) -> (i32, i32) {
    (
        ((p.x + BOUND) / CELL).floor() as i32,
        ((p.y + BOUND) / CELL).floor() as i32,
    )
}

fn center(i: i32, j: i32) -> Vec2 {
    Vec2::new(
        -BOUND + (i as f32 + 0.5) * CELL,
        -BOUND + (j as f32 + 0.5) * CELL,
    )
}

fn inside(i: i32, j: i32) -> bool {
    i >= 0 && j >= 0 && (i as usize) < N && (j as usize) < N
}

impl Nav {
    pub fn build(map: &ContinentMap, world: &World) -> Nav {
        let t = &map.terrain;
        let mut height = vec![0.0; N * N];
        let mut blocked = vec![false; N * N];
        let mut wet = vec![false; N * N];
        for j in 0..N as i32 {
            for i in 0..N as i32 {
                let c = center(i, j);
                let k = j as usize * N + i as usize;
                let land = t.height(c.x, c.y);
                let water = t.water(c.x, c.y);
                height[k] = land.max(water);
                wet[k] = water > land + 0.5;
                // Something tall over the ground anywhere in it (sampled every 3 m: walls are
                // 4 m thick): blocked; but not a high roof overhead (a cave's), only what
                // stands on the ground or tops out low (buildings' floors and roofs).
                const AT: [f32; 5] = [-6.0, -3.0, 0.0, 3.0, 6.0];
                let tall = AT
                    .iter()
                    .flat_map(|x| AT.iter().map(|z| Vec2::new(*x, *z)))
                    .any(|o| {
                        let p = c + o;
                        let g = t.surface(p.x, p.y);
                        world
                            .ground(p.x, p.y, g + 16.0)
                            .is_some_and(|top| top > g + 2.5)
                            || world.occupied(p.x, p.y, g + 2.5, g + 2.5)
                    });
                blocked[k] = tall;
            }
        }
        // Base gateways stay open (the walls either side fill the cell), and routes through
        // them go through the middle.
        let mut portals = Vec::new();
        for b in &map.bases {
            for f in &b.gate_fields {
                let crate::collide::Collider::Box { min, max } = *f else {
                    continue;
                };
                let mid = Vec2::new((min.x + max.x) / 2.0, (min.z + max.z) / 2.0);
                let (i, j) = cell_of(mid);
                if !inside(i, j) {
                    continue;
                }
                let k = j as usize * N + i as usize;
                blocked[k] = false;
                let normal = if max.x - min.x < max.z - min.z {
                    Vec3::X
                } else {
                    Vec3::Z
                };
                let y = t.surface(mid.x, mid.y);
                portals.push((k, Vec3::new(mid.x, y, mid.y), normal));
                // The cells straight in and out of it, too.
                for s in [-1.0, 1.0] {
                    let p = mid + Vec2::new(normal.x, normal.z) * CELL * s;
                    let (a, c) = cell_of(p);
                    if inside(a, c) {
                        blocked[c as usize * N + a as usize] = false;
                    }
                }
            }
        }
        Nav {
            height,
            blocked,
            wet,
            portals,
        }
    }

    fn k(i: i32, j: i32) -> usize {
        j as usize * N + i as usize
    }

    /// The open cell nearest point `p` (in a blocked cell) within a few cells.
    fn open_near(&self, p: Vec2) -> Option<(i32, i32)> {
        let (i, j) = cell_of(p);
        let mut best: Option<((i32, i32), f32)> = None;
        for r in 0..6i32 {
            for dj in -r..=r {
                for di in -r..=r {
                    if di.abs().max(dj.abs()) != r {
                        continue;
                    }
                    let (a, b) = (i + di, j + dj);
                    if inside(a, b) && !self.blocked[Self::k(a, b)] {
                        let d = (center(a, b) - p).length();
                        if best.is_none_or(|(_, bd)| d < bd) {
                            best = Some(((a, b), d));
                        }
                    }
                }
            }
            // (A ring further out can still hold a nearer centre than the first found.)
            if best.is_some_and(|(_, d)| d < r as f32 * CELL) {
                break;
            }
        }
        best.map(|b| b.0)
    }

    /// The way from `from` to `to` for `mode`: waypoints (the last one `to` itself), or None
    /// if there is none.
    pub fn route(&self, from: Vec3, to: Vec3, mode: Mode) -> Option<Vec<Vec3>> {
        let (si, sj) = cell_of(Vec2::new(from.x, from.z));
        let (gi, gj) = cell_of(Vec2::new(to.x, to.z));
        if !inside(si, sj) || !inside(gi, gj) {
            return None;
        }
        let (si, sj) = if self.blocked[Self::k(si, sj)] {
            self.open_near(Vec2::new(from.x, from.z))
                .unwrap_or((si, sj))
        } else {
            (si, sj)
        };
        let (gi, gj) = if self.blocked[Self::k(gi, gj)] {
            self.open_near(Vec2::new(to.x, to.z))?
        } else {
            (gi, gj)
        };
        let start = Self::k(si, sj);
        let goal = Self::k(gi, gj);
        let mut cost = vec![f32::INFINITY; N * N];
        let mut came = vec![u32::MAX; N * N];
        let mut open = BinaryHeap::new();
        let h = |k: usize| {
            let (i, j) = ((k % N) as i32, (k / N) as i32);
            let (dx, dy) = ((i - gi).abs() as f32, (j - gj).abs() as f32);
            (dx.max(dy) + 0.414 * dx.min(dy)) * CELL
        };
        cost[start] = 0.0;
        open.push(Reverse(((h(start) * 10.0) as u32, start as u32)));
        let mut expanded = 0;
        while let Some(Reverse((_, k))) = open.pop() {
            let k = k as usize;
            if k == goal {
                break;
            }
            expanded += 1;
            if expanded > 40_000 {
                return None;
            }
            let (i, j) = ((k % N) as i32, (k / N) as i32);
            for (di, dj) in [
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ] {
                let (a, b) = (i + di, j + dj);
                if !inside(a, b) {
                    continue;
                }
                let n = Self::k(a, b);
                // No cutting corners past blocked cells.
                if di != 0
                    && dj != 0
                    && (self.blocked[Self::k(i + di, j)] || self.blocked[Self::k(i, j + dj)])
                {
                    continue;
                }
                let run = if di != 0 && dj != 0 { 1.414 } else { 1.0 } * CELL;
                let rise = (self.height[n] - self.height[k]).abs();
                let slope = rise / run;
                let mut step = run * (1.0 + slope * 3.0);
                if slope > mode.climb {
                    if !mode.squeeze {
                        continue;
                    }
                    step *= 8.0;
                }
                if self.blocked[n] && n != goal {
                    if !mode.squeeze {
                        continue;
                    }
                    step *= 12.0;
                }
                if self.wet[n] {
                    step *= mode.water;
                }
                let c = cost[k] + step;
                if c < cost[n] {
                    cost[n] = c;
                    came[n] = k as u32;
                    open.push(Reverse((((c + h(n)) * 10.0) as u32, n as u32)));
                }
            }
        }
        if !cost[goal].is_finite() {
            return None;
        }
        // Back from the goal, keeping only the turns.
        let mut cells = vec![goal];
        let mut k = goal;
        while k != start {
            k = came[k] as usize;
            cells.push(k);
        }
        cells.reverse();
        let mut out = Vec::new();
        for w in 1..cells.len().saturating_sub(1) {
            let (a, b, c) = (cells[w - 1], cells[w], cells[w + 1]);
            if let Some(&(_, mid, n)) = self.portals.iter().find(|p| p.0 == b) {
                // Through a gateway: square on to it, through the middle, out straight.
                let (ai, aj) = ((a % N) as f32, (a / N) as f32);
                let (ci, cj) = ((c % N) as f32, (c / N) as f32);
                let dir = Vec3::new(ci - ai, 0.0, cj - aj);
                let s = if dir.dot(n) >= 0.0 { 1.0 } else { -1.0 };
                out.push(mid - n * s * 14.0);
                out.push(mid);
                out.push(mid + n * s * 14.0);
                continue;
            }
            let d1 = b as i64 - a as i64;
            let d2 = c as i64 - b as i64;
            if d1 != d2 {
                let (i, j) = ((b % N) as i32, (b / N) as i32);
                let p = center(i, j);
                out.push(Vec3::new(p.x, self.height[b], p.y));
            }
        }
        out.push(to);
        Some(out)
    }
}

/// Following a route: replanned when the destination moves or now and then.
#[derive(Clone, Debug, Default)]
pub struct Route {
    points: Vec<Vec3>,
    next: usize,
    to: Option<Vec3>,
    age: f32,
}

impl Route {
    /// Where to head now on the way from `from` to `to` (`reach`: how near a waypoint counts
    /// as there).
    pub fn toward(
        &mut self,
        nav: &Nav,
        from: Vec3,
        to: Vec3,
        mode: Mode,
        reach: f32,
        dt: f32,
    ) -> Vec3 {
        self.age += dt;
        let moved = self.to.is_none_or(|t| (t - to).length() > 25.0);
        if moved || self.age > 12.0 {
            self.age = 0.0;
            self.to = Some(to);
            self.next = 0;
            self.points = nav.route(from, to, mode).unwrap_or_else(|| vec![to]);
        }
        let flat = |p: Vec3| Vec2::new(p.x - from.x, p.z - from.z).length();
        while self.next + 1 < self.points.len() && flat(self.points[self.next]) < reach {
            self.next += 1;
        }
        // Skip ahead to a later point that's nearer (rounded a corner already).
        if self.next + 1 < self.points.len()
            && flat(self.points[self.next + 1]) < flat(self.points[self.next])
        {
            self.next += 1;
        }
        self.points.get(self.next).copied().unwrap_or(to)
    }

    pub fn clear(&mut self) {
        *self = Route::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worldgen::{WorldDef, WorldParams};
    use std::sync::Arc;

    #[test]
    fn routes_go_round_walls_and_mountains() {
        let def = WorldDef::generate(WorldParams {
            seed: 11,
            contested: 4,
        });
        let c = Arc::new(def.continents[3].clone());
        let map = ContinentMap::build(c, false);
        let nav = Nav::build(&map, &map.world);
        // From outside one base to outside another, on wheels.
        let a = map.bases[1].center + Vec3::new(map.bases[1].half + 40.0, 0.0, 0.0);
        let b = map.bases[2].center + Vec3::new(0.0, 0.0, map.bases[2].half + 40.0);
        let route = nav.route(a, b, GROUND_VEHICLE).expect("a way");
        assert!(route.len() >= 2);
        // No waypoint is on a blocked cell or on a step too steep.
        for w in &route[..route.len() - 1] {
            let (i, j) = cell_of(Vec2::new(w.x, w.z));
            assert!(!nav.blocked[Nav::k(i, j)], "waypoint {w} blocked");
        }
        // Out of a base's courtyard through a gateway.
        let inside_base = map.bases[0].center + Vec3::new(0.0, 0.0, -map.bases[0].half * 0.6);
        assert!(nav.route(inside_base, b, GROUND_VEHICLE).is_some());
    }

    #[test]
    fn ground_vehicles_reach_along_the_lattice_on_every_theme() {
        use crate::theme::Theme;
        use crate::worldgen::Node;
        let w = WorldDef::generate(WorldParams {
            seed: 21,
            contested: 6,
        });
        for theme in Theme::CONTESTED {
            let def = w.continents.iter().find(|c| c.theme == theme).unwrap();
            let map = ContinentMap::build(Arc::new(def.clone()), false);
            let nav = Nav::build(&map, &map.world);
            // Just outside each end of each link (a base's gateway, a gate's pad).
            let outside = |n: Node, to: Vec2| -> Vec3 {
                let p = def.node_pos(n);
                let r = match n {
                    Node::Base(b) => def.bases[b as usize].pad_radius() + 20.0,
                    Node::Gate(_) => crate::worldgen::GATE_PAD + 20.0,
                };
                let q = p + (to - p).normalize() * r;
                Vec3::new(q.x, map.terrain.surface(q.x, q.y), q.y)
            };
            for &(a, b) in &def.lattice {
                let (pa, pb) = (def.node_pos(a), def.node_pos(b));
                let route = nav.route(outside(a, pb), outside(b, pa), GROUND_VEHICLE);
                assert!(route.is_some(), "{theme:?}: no way from {a:?} to {b:?}");
            }
        }
    }

    #[test]
    fn routes_start_from_the_open_cell_nearest() {
        // Outside a tower's door (in a cell the tower blocks): the open cell nearest the door,
        // not one round the other side.
        let def = WorldDef::generate(WorldParams {
            seed: 11,
            contested: 4,
        });
        let c = Arc::new(def.continents[3].clone());
        let map = ContinentMap::build(c, false);
        let nav = Nav::build(&map, &map.world);
        for l in &map.bases {
            for &p in l.to_console.first().into_iter().chain(l.from_spawn.last()) {
                let p = Vec2::new(p.x, p.z);
                let (i, j) = cell_of(p);
                if !nav.blocked[Nav::k(i, j)] {
                    continue;
                }
                let got = nav.open_near(p).expect("an open cell");
                let d = (center(got.0, got.1) - p).length();
                for b in j - 3..=j + 3 {
                    for a in i - 3..=i + 3 {
                        if inside(a, b) && !nav.blocked[Nav::k(a, b)] {
                            assert!(d <= (center(a, b) - p).length() + 1e-3);
                        }
                    }
                }
            }
        }
    }
}
