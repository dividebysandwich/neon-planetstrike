//! A continent built from its definition: the land, the scenery, the bases' structures and the
//! collision world over all of them. The server builds one per continent; a client builds the
//! one it is on.

use std::sync::Arc;

use glam::Vec3;

use crate::base::{self, BaseLayout};
use crate::collide::World;
use crate::scenery::{self, Item};
use crate::terrain::{BOUND, Terrain};
use crate::worldgen::{ContinentDef, Gate};

pub struct ContinentMap {
    pub def: Arc<ContinentDef>,
    pub terrain: Arc<Terrain>,
    /// The scenery (only the solid pieces on the server).
    pub scenery: Vec<Item>,
    pub bases: Vec<BaseLayout>,
    /// Each gate's middle, on its pad.
    pub gates: Vec<Vec3>,
    /// Its caves' roofs (canyon cave continents).
    pub caves: crate::caves::Caves,
    pub world: World,
}

impl ContinentMap {
    /// Builds the continent (all the scenery when `full`, for drawing; else the solid pieces).
    pub fn build(def: Arc<ContinentDef>, full: bool) -> ContinentMap {
        let terrain = Arc::new(def.terrain());
        let scenery = scenery::place(&def, &terrain, !full);
        let bases: Vec<BaseLayout> = def
            .bases
            .iter()
            .enumerate()
            .map(|(i, site)| {
                base::layout(
                    site,
                    terrain.pad_heights[i],
                    def.terrain_seed ^ (i as u32 * 7919),
                )
            })
            .collect();
        let gates = def
            .gates
            .iter()
            .enumerate()
            .map(|(i, g): (usize, &Gate)| {
                Vec3::new(g.pos.x, terrain.pad_heights[def.bases.len() + i], g.pos.y)
            })
            .collect::<Vec<_>>();
        let mut colliders: Vec<_> = scenery.iter().filter_map(Item::collider).collect();
        for b in &bases {
            colliders.extend(b.colliders.iter().copied());
        }
        for g in &gates {
            colliders.extend(base::gate_colliders(*g));
        }
        let caves = crate::caves::build(&def, &terrain, &bases);
        colliders.extend(caves.roofs.iter().copied());
        let mut world = World::new(colliders, Some(terrain.clone()));
        for b in &bases {
            world.holes.extend(b.holes.iter().copied());
        }
        for b in &bases {
            world
                .shields
                .extend(b.shields.iter().map(|s| (s.collider, None)));
        }
        let mut caves = caves;
        caves.lines = crate::caves::lines(&def, &terrain, &bases, &world);
        ContinentMap {
            def,
            terrain,
            scenery,
            bases,
            gates,
            caves,
            world,
        }
    }

    /// Keeps a position inside the map's bounds.
    pub fn clamp(p: Vec3) -> Vec3 {
        Vec3::new(p.x.clamp(-BOUND, BOUND), p.y, p.z.clamp(-BOUND, BOUND))
    }
}
