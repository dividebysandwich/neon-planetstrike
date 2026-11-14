//! Construction (as PlanetSide 2's): a harvester's construction terminal hands out structures
//! for resources from its load; whoever pulled one carries it and puts it down out in the
//! field (not in a base). A mobile silo keeps the structures near it going: each draws a little
//! from it all the time, and harvesters refill it; without a silo that isn't empty they decay
//! (an empty silo too) until they fall. The rules are in `sim/build.rs`; this is what each is
//! (shared with the client: its look, its placing ghost), and how one is kept on the server's
//! disk.

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use crate::collide::Collider;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructureKind {
    /// A mobile silo: holds resources for the structures near it (`SILO_RANGE`).
    Silo,
    /// A barrier 6 m wide and 3.5 m high: stops bodies, vehicles and shots.
    Wall,
    /// An armoured bunker with an automatic turret on top, against infantry and vehicles.
    Bastion,
    /// A forward spawn tube: its faction may spawn at it (one per base).
    Tube,
}

/// How near the builder one may put a structure down (m).
pub const BUILD_REACH: f32 = 40.0;
/// No building within this many times a base's half size of its middle (nor near a tower).
pub const NO_BUILD: f32 = 1.3;
/// A mobile silo keeps the structures this near going (m).
pub const SILO_RANGE: f32 = 120.0;
/// What a mobile silo holds full (three harvester loads), and what it starts with.
pub const SILO_HOLDS: f32 = crate::state::SILO_CAP / 2.0;
pub const SILO_START: f32 = SILO_HOLDS * 0.1;
/// Health (percent) a structure loses a second without a silo to draw on (an empty silo too).
pub const DECAY: f32 = 0.5;
/// A harvester unloads into a mobile silo this near (m).
pub const SILO_UNLOAD: f32 = 12.0;
/// How near a harvester its construction terminal may be used (m from its hull).
pub const TERMINAL_REACH: f32 = 4.0;
/// Below this share of what it holds, a silo warns its faction.
pub const SILO_LOW: f32 = 0.15;
/// The Bastion's gun: pivot over the ground, reach, rate.
pub const BASTION_PIVOT: f32 = 3.3;
pub const BASTION_RANGE: f32 = 150.0;
pub const BASTION_AIR_RANGE: f32 = 220.0;
pub const BASTION_INTERVAL: f32 = 0.3;
/// Where one spawns at a tube: this far in front of it.
pub const TUBE_SPAWN: f32 = 2.6;

impl StructureKind {
    pub const ALL: [StructureKind; 4] = [
        StructureKind::Silo,
        StructureKind::Wall,
        StructureKind::Bastion,
        StructureKind::Tube,
    ];

    pub fn name(self) -> &'static str {
        match self {
            StructureKind::Silo => "Mobile Silo",
            StructureKind::Wall => "Wall",
            StructureKind::Bastion => "Bastion Turret",
            StructureKind::Tube => "Forward Spawn Tube",
        }
    }

    /// Resources it takes from the harvester's load (a full load: `sim::CARGO`).
    pub fn cost(self) -> f32 {
        match self {
            StructureKind::Silo => 40.0,
            StructureKind::Wall => 15.0,
            StructureKind::Bastion => 50.0,
            StructureKind::Tube => 60.0,
        }
    }

    /// Resources a second it draws from its silo (a silo: from itself).
    pub fn upkeep(self) -> f32 {
        match self {
            StructureKind::Silo => 0.02,
            StructureKind::Wall => 0.02,
            StructureKind::Bastion => 0.06,
            StructureKind::Tube => 0.05,
        }
    }

    /// Half its width and depth (x, z) and its height: the box it fills, from the ground up.
    pub fn size(self) -> (Vec2, f32) {
        match self {
            StructureKind::Silo => (Vec2::new(1.8, 1.8), 4.5),
            StructureKind::Wall => (Vec2::new(3.0, 0.4), 3.5),
            StructureKind::Bastion => (Vec2::new(1.6, 1.6), 2.9),
            StructureKind::Tube => (Vec2::new(1.2, 1.2), 3.2),
        }
    }

    /// Its corners (x, z) standing at `at` turned by `yaw` (as the game turns things: 0 facing
    /// -Z).
    pub fn corners(self, at: Vec3, yaw: f32) -> [Vec2; 4] {
        let (h, _) = self.size();
        let (s, c) = yaw.sin_cos();
        // (Local x, z to world: the rotation about +Y by `yaw`.)
        let w = |x: f32, z: f32| Vec2::new(at.x + x * c + z * s, at.z - x * s + z * c);
        [w(-h.x, -h.y), w(-h.x, h.y), w(h.x, h.y), w(h.x, -h.y)]
    }

    /// The collider it is standing at `at` turned by `yaw`.
    pub fn collider(self, at: Vec3, yaw: f32) -> Collider {
        let pts = self.corners(at, yaw);
        Collider::prism(&pts, at.y, at.y + self.size().1)
    }

    /// Where a segment from `a` to `b` first meets it (the fraction along), standing at `at`
    /// turned by `yaw`.
    pub fn hit(self, at: Vec3, yaw: f32, a: Vec3, b: Vec3) -> Option<f32> {
        let (h, height) = self.size();
        let to_local = glam::Quat::from_rotation_y(-yaw);
        let (la, lb) = (to_local * (a - at), to_local * (b - at));
        let seg = lb - la;
        let len = seg.length();
        if len < 1e-6 {
            return None;
        }
        crate::collide::ray_box(
            la,
            seg / len,
            Vec3::new(-h.x, 0.0, -h.y),
            Vec3::new(h.x, height, h.y),
            len,
        )
        .map(|(t, _)| t / len)
    }

    pub fn max_health(self) -> f32 {
        match self {
            StructureKind::Silo => 100.0,
            StructureKind::Wall => 100.0,
            StructureKind::Bastion => 100.0,
            StructureKind::Tube => 80.0,
        }
    }
}

/// A structure as the server keeps it on disk (over a restart).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedStructure {
    pub kind: StructureKind,
    pub team: crate::Team,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub health: f32,
    /// A silo's resources.
    pub resources: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turned_structure_is_hit_where_it_stands() {
        let at = Vec3::new(10.0, 2.0, 5.0);
        // A wall turned a quarter: 6 m along z, 0.8 m along x.
        let yaw = std::f32::consts::FRAC_PI_2;
        let k = StructureKind::Wall;
        let a = at + Vec3::new(-5.0, 1.0, 2.5);
        let b = at + Vec3::new(5.0, 1.0, 2.5);
        let f = k.hit(at, yaw, a, b).expect("through it");
        assert!((a.lerp(b, f).x - (at.x - 0.4)).abs() < 1e-3);
        // Past its end.
        assert!(
            k.hit(
                at,
                yaw,
                at + Vec3::new(-5.0, 1.0, 3.5),
                at + Vec3::new(5.0, 1.0, 3.5)
            )
            .is_none()
        );
        // Over it.
        assert!(
            k.hit(
                at,
                yaw,
                at + Vec3::new(-5.0, 3.8, 0.0),
                at + Vec3::new(5.0, 3.8, 0.0)
            )
            .is_none()
        );
        let (min, max) = k.collider(at, yaw).bounds();
        assert!((max.z - min.z - 6.0).abs() < 1e-3 && (max.x - min.x - 0.8).abs() < 1e-3);
    }
}
