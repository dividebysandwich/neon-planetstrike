//! Where a continent's scenery stands: rocks, trees, crystals, ... scattered by the theme's
//! `Scatter` table, kept off bases, gates, roads and too steep land. The same on the server
//! (which only needs the solid ones, for collision) and every client (which draws them all).

use glam::{Vec2, Vec3};

use crate::collide::{Collider, round_prism};
use crate::noise::{self, smoothstep};
use crate::terrain::{HALF, SEA, Terrain};
use crate::theme::SceneryKind;
use crate::worldgen::{ContinentDef, GATE_PAD};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Item {
    pub kind: SceneryKind,
    pub pos: Vec3,
    pub scale: f32,
    pub yaw: f32,
    /// A hash for per-item variety (colour, shape).
    pub hash: u32,
}

impl Item {
    pub fn collider(&self) -> Option<Collider> {
        let (r, h) = self.kind.collider()?;
        let sides = match self.kind {
            SceneryKind::Rock | SceneryKind::Boulder | SceneryKind::Basalt => 6,
            SceneryKind::Crystal | SceneryKind::IceShard | SceneryKind::Monolith => 4,
            _ => 8,
        };
        // (Sunk a little into the ground so slopes leave no gap under them.)
        Some(round_prism(
            self.pos - Vec3::Y * 1.0,
            r * self.scale,
            h * self.scale + 1.0,
            sides,
            self.yaw,
        ))
    }
}

/// Places the continent's scenery: only the solid pieces when `solid_only`.
pub fn place(def: &ContinentDef, terrain: &Terrain, solid_only: bool) -> Vec<Item> {
    let keep_out: Vec<(Vec2, f32)> = def
        .bases
        .iter()
        .map(|b| (b.pos, b.pad_radius() + 25.0))
        .chain(def.gates.iter().map(|g| (g.pos, GATE_PAD + 20.0)))
        .collect();
    let seed = def.terrain_seed;
    let mut out = Vec::new();
    for (si, sc) in def.theme.scatter().iter().enumerate() {
        if solid_only && sc.kind.collider().is_none() {
            continue;
        }
        let spacing = (10_000.0 / sc.per_ha).sqrt();
        let n = (2.0 * HALF / spacing).ceil() as i32;
        let salt = seed ^ (si as u32 + 1).wrapping_mul(0x9e37_79b9);
        for j in 0..n {
            for i in 0..n {
                let h = noise::hash2(i, j, salt);
                let x = -HALF + (i as f32 + noise::unit(h)) * spacing;
                let z = -HALF + (j as f32 + noise::unit(h.rotate_left(11))) * spacing;
                // Groves: a slow noise thins the scatter out between them.
                if sc.clump > 0.0 {
                    let g = noise::fbm(x / 260.0, z / 260.0, salt ^ 0x77, 3, 0.5);
                    let p = (1.0 - sc.clump) + sc.clump * smoothstep(-0.1, 0.35, g) * 1.6;
                    if noise::unit(h.rotate_left(5)) > p {
                        continue;
                    }
                }
                if keep_out
                    .iter()
                    .any(|(c, r)| c.distance(Vec2::new(x, z)) < *r)
                {
                    continue;
                }
                if terrain.road_distance(x, z) < crate::roads::SHOULDER + 4.0 {
                    continue;
                }
                let y = terrain.height(x, z);
                if y < SEA + sc.min_height || terrain.slope(x, z) > sc.max_slope {
                    continue;
                }
                let k = noise::unit(h.rotate_left(17));
                out.push(Item {
                    kind: sc.kind,
                    pos: Vec3::new(x, y, z),
                    scale: sc.scale.0 + (sc.scale.1 - sc.scale.0) * k,
                    yaw: noise::unit(h.rotate_left(23)) * std::f32::consts::TAU,
                    hash: h,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worldgen::{WorldDef, WorldParams};

    #[test]
    fn scenery_keeps_off_bases() {
        let w = WorldDef::generate(WorldParams {
            seed: 3,
            contested: 4,
        });
        let def = &w.continents[4];
        let t = def.terrain();
        let items = place(def, &t, false);
        let solid = place(def, &t, true);
        assert!(
            items.len() > solid.len() && !solid.is_empty(),
            "{}",
            items.len()
        );
        for it in &items {
            for b in &def.bases {
                assert!(b.pos.distance(Vec2::new(it.pos.x, it.pos.z)) > b.pad_radius());
            }
        }
        assert!(solid.iter().all(|i| i.collider().is_some()));
        // Nothing on the roads.
        assert!(!t.roads.is_empty());
        for r in &t.roads {
            for p in r.points.iter().step_by(4) {
                for it in &items {
                    let d = Vec2::new(it.pos.x - p.x, it.pos.z - p.z).length();
                    assert!(
                        d > crate::roads::HALF_WIDTH + 2.0,
                        "{:?} on a road",
                        it.kind
                    );
                }
            }
        }
    }
}
