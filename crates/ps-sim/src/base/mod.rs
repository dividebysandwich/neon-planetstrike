//! A base's structures, from its site.
//!
//! - **Bases** (outposts, center bases, sanctuary homes): a square compound of tall walls you
//!   can walk on (a parapet outside, a railing inside; enclosed stair towers up to them, their
//!   top landings level with the walkway), bastions on the corners, two vehicle gateways under
//!   the walls' lintels (the walkways run on over them, under a gallery) closed to enemy
//!   vehicles by forcefields (infantry walk through), wall turrets on pillars, the resource
//!   silo, and the main building: on the ground floor the spawn room (a forcefield across its
//!   doorway lets only the owners through), the hall with terminals, the core room with the
//!   console that is hacked and the generator room powering the base; usually an upper floor of
//!   offices; enclosed stairs in its corner up to every floor and out onto the roof through
//!   their head house. Furnished with crates, racks and desks. In the yard: the vehicle pads
//!   with their terminals and the rearm pads. Which plan a base has (where its buildings,
//!   stairs and bastions are, what stands in its yard) comes from its seed (`plans`).
//! - **Towers**: three storeys up walled stairs inside, the capture console on the top floor,
//!   turrets on the roof, a spawn bunker below ground, and an apron round the foot.
//!
//! No edge one can walk to is open: walkways, landings, roofs and the floors round stairwells
//! have a parapet, a wall or a railing (`RAIL_H` high: over it only by jumping).
//!
//! Ramps are drawn as stairs. The buildings and walls are dressed with drawn-only `Detail`s
//! (angled cornices, plinths, piers and buttresses, framed and glazed windows, string courses,
//! vents, gateway pylons) that nothing collides with.
//!
//! Laid out in the site's frame (its front towards -Z) and turned by quarter turns, so every
//! collider stays axis-aligned (the details needn't be).

mod blocks;
mod builder;
mod building;
mod facade;
mod plans;
mod ring;

use builder::{Builder, rot};
use plans::tower::tower;

use glam::{Vec2, Vec3};

use crate::collide::{Collider, round_prism};
use crate::noise::Seq;
use crate::worldgen::{BaseKind, BaseSite, Capture};

/// What a structure box is drawn as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Wall,
    Parapet,
    Building,
    Roof,
    Floor,
    Interior,
    Ramp,
    Pillar,
    /// Furniture: crates, racks, desks.
    Decor,
    /// A railing: drawn as posts and rails (solid as a thin wall).
    Rail,
    /// The enclosed stairs' walls.
    Stairwell,
    /// A corner bastion's platform, and what stands over a gateway.
    Bastion,
    Gatehouse,
    /// A roof nobody walks on: over stairs, galleries, sheds.
    Canopy,
    /// Low cover in the yard.
    Cover,
    /// A turret's pillar (drawn with the turret).
    Mount,
}

/// What a detail is drawn as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finish {
    /// The walls' plating.
    Body,
    /// Dark glass.
    Glass,
    /// A neon strip in the owner's colour.
    Trim,
}

/// A piece of the architecture that is drawn but not solid (cornices, buttresses, plinths,
/// window frames and panes, vents): a section of four corners at its start (`pts[..4]`, in
/// order round it) swept to the same four at its end (`pts[4..]`), in the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Detail {
    pub pts: [Vec3; 8],
    pub finish: Finish,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalKind {
    /// Pick a loadout; also refills ammo and armor.
    Equipment,
    /// Heals to full.
    Medical,
    /// Spawns vehicles on the pad next to it.
    Vehicle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Terminal {
    pub kind: TerminalKind,
    pub pos: Vec3,
    /// The way its screen faces (yaw, 0 = -Z): towards whoever uses it.
    pub yaw: f32,
    /// Vehicle terminals: the pad they spawn on.
    pub pad: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shield {
    pub collider: Collider,
}

/// A light the client hangs: a floodlight on the walls (pointing at `aim`), or a room's
/// ceiling light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lamp {
    pub pos: Vec3,
    pub aim: Option<Vec3>,
}

/// A capture point (`Capture::Points` bases): a zone round its pylon held by standing in it.
#[derive(Clone, Debug, PartialEq)]
pub struct CapturePoint {
    /// The zone's middle (on the floor) and its radius.
    pub pos: Vec3,
    pub radius: f32,
    /// The pylon's foot (it marks the point; it's solid).
    pub pylon: Vec3,
    /// A clear spot in the zone to stand on, and the way to it for bots from outside the
    /// front gateway.
    pub stand: Vec3,
    pub path: Vec<Vec3>,
}

/// The points' letters.
pub const POINT_NAMES: [char; 3] = ['A', 'B', 'C'];
/// A pylon's radius and height.
pub const PYLON_R: f32 = 0.45;
pub const PYLON_H: f32 = 4.2;

#[derive(Clone, Debug)]
pub struct BaseLayout {
    pub kind: BaseKind,
    pub center: Vec3,
    /// Quarter turns of the base's frame.
    pub quarter: u8,
    /// The compound's half width (to the walls' outer face; a tower's footprint).
    pub half: f32,
    pub wall_h: f32,
    pub colliders: Vec<Collider>,
    pub styles: Vec<Style>,
    /// Team forcefields bodies of other factions don't pass (spawn rooms, the tower bunker).
    pub shields: Vec<Shield>,
    /// Forcefields across the gateways: other factions' vehicles don't pass.
    pub gate_fields: Vec<Collider>,
    /// Spawn points (feet, yaw).
    pub spawns: Vec<(Vec3, f32)>,
    /// The console (hacked to capture the base): where it stands and faces.
    pub console: Option<(Vec3, f32)>,
    pub terminals: Vec<Terminal>,
    /// Vehicle pads (middle, yaw vehicles face).
    pub pads: Vec<(Vec3, f32)>,
    pub rearm: Vec<Vec3>,
    /// Turrets' mounts.
    pub turrets: Vec<Vec3>,
    /// The generator (its middle on the floor) and the resource silo (its foot).
    pub generator: Option<Vec3>,
    pub silo: Option<Vec3>,
    pub lamps: Vec<Lamp>,
    /// The architecture's drawn-only details.
    pub details: Vec<Detail>,
    /// Areas (XZ min, max) where the land isn't solid (a tower's bunker under it). The land
    /// isn't drawn over them either: its cells touching one are left out, and the tower's
    /// apron covers the ground between.
    pub holes: Vec<(Vec2, Vec2)>,
    /// The inside of the main building (for "inside" tests: min, max).
    pub building: (Vec3, Vec3),
    pub storeys: u8,
    /// Ways through the base for bots: from outside the front gateway to the console, and from
    /// the spawn room out through the front gateway.
    pub to_console: Vec<Vec3>,
    pub from_spawn: Vec<Vec3>,
    /// Capture points A, B, C (none where the console is hacked).
    pub points: Vec<CapturePoint>,
    /// Which of its kind's plans it is, and whether mirrored left to right.
    pub variant: u8,
    pub flip: bool,
    /// The insides of all its buildings (the main one's first; min, max).
    pub interiors: Vec<(Vec3, Vec3)>,
    /// Where ziplines land on the main building's roof.
    pub zip_anchors: Vec<Vec3>,
    /// Where each turret's gunner gets out (as `turrets`).
    pub turret_stands: Vec<Vec3>,
    /// Where bots holding the base gather in the yard.
    pub rally: Vec3,
    /// Where one can walk in the base: the gateways, doors, halls, stairs and landings, and
    /// the straight walks between them (the ways above are found through it).
    pub graph: Graph,
}

/// A base's walk graph: nodes (on the floor, in the world) and the straight walks between
/// them, either way.
#[derive(Clone, Debug, Default)]
pub struct Graph {
    pub nodes: Vec<Vec3>,
    pub links: Vec<(u32, u32)>,
}

impl Graph {
    /// The shortest walk from node `from` to node `to`: the nodes' points, both ends included.
    pub fn path(&self, from: usize, to: usize) -> Option<Vec<Vec3>> {
        let n = self.nodes.len();
        let mut dist = vec![f32::MAX; n];
        let mut prev = vec![usize::MAX; n];
        let mut done = vec![false; n];
        dist[from] = 0.0;
        loop {
            let at = (0..n)
                .filter(|&i| !done[i] && dist[i] < f32::MAX)
                .min_by(|&a, &b| dist[a].total_cmp(&dist[b]))?;
            if at == to {
                break;
            }
            done[at] = true;
            for &(a, b) in &self.links {
                let (a, b) = (a as usize, b as usize);
                let other = if a == at {
                    b
                } else if b == at {
                    a
                } else {
                    continue;
                };
                let d = dist[at] + self.nodes[at].distance(self.nodes[other]);
                if d < dist[other] {
                    dist[other] = d;
                    prev[other] = at;
                }
            }
        }
        let mut out = vec![self.nodes[to]];
        let mut at = to;
        while at != from {
            at = prev[at];
            out.push(self.nodes[at]);
        }
        out.reverse();
        Some(out)
    }
}

pub const WALL_T: f32 = 4.0;
pub const WALL_H: f32 = 12.0;
pub const GATEWAY: f32 = 18.0;
/// The gateways' height under the walls' lintels.
pub const GATE_CLEAR: f32 = 8.0;
/// The walls' parapet's height; the railings' height and thickness.
pub const PARAPET_H: f32 = 1.3;
pub const RAIL_H: f32 = 1.1;
pub const RAIL_T: f32 = 0.15;
/// A bastion: how far in from the corner along the walls, and out past their faces.
pub const BASTION_IN: f32 = 10.0;
pub const BASTION_OUT: f32 = 3.0;
/// Nothing of a base stands higher than this over its pad (the canyon caves' roofs).
pub const MAX_HEIGHT: f32 = 21.0;
pub const BLD_HALF: Vec2 = Vec2::new(18.0, 14.0);
pub const BLD_C: Vec2 = Vec2::new(0.0, 8.0);
/// A storey's height (floor to floor) and the floor slabs' thickness.
pub const STOREY: f32 = 5.4;
pub const SLAB: f32 = 0.4;
pub const T: f32 = 0.6;
pub const DOOR_W: f32 = 5.0;
pub const DOOR_H: f32 = 4.0;
pub const RAMP_LEN: f32 = 24.0;
pub const RAMP_W: f32 = 3.5;
/// The rearm pad's reach, and the vehicle pads'.
pub const REARM_RADIUS: f32 = 8.0;
pub const PAD_RADIUS: f32 = 7.0;
/// How close a player stands to use a terminal or console.
pub const USE_REACH: f32 = 2.6;
/// A turret's pivot above its mount, its barrels' length, and where its gunner's eyes are
/// from the pivot. Mounts stand on pillars `TURRET_RISE` over the wall (or roof) for a view
/// down past its foot.
pub const TURRET_PIVOT: f32 = 2.4;
pub const TURRET_RISE: f32 = 3.5;
pub const TURRET_PILLAR: f32 = 0.8;
pub const TURRET_BARREL: f32 = 2.6;
pub const TURRET_EYE: Vec3 = Vec3::new(0.0, 0.5, 0.6);
/// The generator's half extents, and the silo's radius and height.
pub const GENERATOR_HALF: Vec3 = Vec3::new(2.0, 1.75, 2.0);
pub const SILO_R: f32 = 4.0;
pub const SILO_H: f32 = 12.0;
/// A harvester unloads within this of a silo.
pub const SILO_REACH: f32 = 16.0;
/// A tower's half width, storeys and bunker depth.
pub const TOWER_HALF: f32 = 8.0;

/// Which quarter turn the site's facing comes to.
fn quarter(site: &BaseSite) -> u8 {
    ((site.facing / std::f32::consts::FRAC_PI_2).round() as i32).rem_euclid(4) as u8
}

/// Half the compound's (the tower's) width.
fn half_of(kind: BaseKind) -> f32 {
    match kind {
        BaseKind::Outpost => 64.0,
        BaseKind::Center => 88.0,
        BaseKind::Home => 100.0,
        BaseKind::Tower => TOWER_HALF,
    }
}

/// The compound's vehicle gateways, front and back (none for a tower): the middle of each on
/// the wall's outer face, and which way is out.
pub fn gateways(site: &BaseSite) -> Vec<(Vec2, Vec2)> {
    if site.kind == BaseKind::Tower {
        return Vec::new();
    }
    let (q, half) = (quarter(site), half_of(site.kind));
    [-1.0f32, 1.0]
        .iter()
        .map(|&s| {
            let out = rot(q, Vec2::new(0.0, s));
            (site.pos + out * half, out)
        })
        .collect()
}

/// The structures of the site, its pad at height `y`.
pub fn layout(site: &BaseSite, y: f32, seed: u32) -> BaseLayout {
    let q = quarter(site);
    let half = half_of(site.kind);
    let origin = Vec3::new(site.pos.x, y, site.pos.y);
    let mut b = Builder {
        origin,
        q,
        out: BaseLayout {
            kind: site.kind,
            center: origin,
            quarter: q,
            half,
            wall_h: WALL_H,
            colliders: Vec::new(),
            styles: Vec::new(),
            shields: Vec::new(),
            gate_fields: Vec::new(),
            spawns: Vec::new(),
            console: None,
            terminals: Vec::new(),
            pads: Vec::new(),
            rearm: Vec::new(),
            turrets: Vec::new(),
            generator: None,
            silo: None,
            lamps: Vec::new(),
            details: Vec::new(),
            holes: Vec::new(),
            building: (Vec3::ZERO, Vec3::ZERO),
            storeys: 1,
            to_console: Vec::new(),
            from_spawn: Vec::new(),
            points: Vec::new(),
            graph: Graph::default(),
            variant: 0,
            flip: false,
            interiors: Vec::new(),
            zip_anchors: Vec::new(),
            turret_stands: Vec::new(),
            rally: Vec3::ZERO,
        },
        frames: Vec::new(),
        marks: Vec::new(),
    };
    let mut rng = Seq::new(seed as u64, 0xba5e);
    if site.kind == BaseKind::Tower {
        b.at(Vec2::ZERO, 0.0, 0, false, tower);
        return b.out;
    }
    let storeys = match site.kind {
        BaseKind::Outpost if rng.chance(0.45) => 1,
        _ => 2,
    };
    plans::compound(
        &mut b,
        &mut plans::Ctx {
            kind: site.kind,
            half,
            storeys,
            points: site.kind == BaseKind::Outpost && site.capture == Capture::Points,
            rng: &mut rng,
        },
    );
    b.out
}

/// The warp gate's pillars around its middle `at`.
pub fn gate_colliders(at: Vec3) -> Vec<Collider> {
    (0..GATE_PILLARS)
        .map(|i| {
            let a = i as f32 / GATE_PILLARS as f32 * std::f32::consts::TAU + 0.26;
            let p = at + Vec3::new(a.cos(), 0.0, a.sin()) * GATE_RING;
            round_prism(p - Vec3::Y, 2.4, GATE_H + 1.0, 6, a)
        })
        .collect()
}

/// The warp gate's ring of pillars (count, radius, height) and its core: what enters the core
/// (within `GATE_CORE` of the middle, below `GATE_H`) is warped.
pub const GATE_PILLARS: usize = 8;
pub const GATE_RING: f32 = 40.0;
pub const GATE_H: f32 = 55.0;
pub const GATE_CORE: f32 = 14.0;
/// The gate's bubble (the shell over it: an ellipsoid this wide and high): nobody fires inside
/// it, and no shot goes through it either way.
pub const GATE_BUBBLE_R: f32 = 38.0;
pub const GATE_BUBBLE_H: f32 = 48.0;

/// Whether `p` is inside the bubble of the gate whose middle is `gate`.
pub fn in_gate_bubble(gate: Vec3, p: Vec3) -> bool {
    let d = p - gate;
    if d.y < -4.0 {
        return false;
    }
    let (x, y, z) = (
        d.x / GATE_BUBBLE_R,
        d.y.max(0.0) / GATE_BUBBLE_H,
        d.z / GATE_BUBBLE_R,
    );
    x * x + y * y + z * z < 1.0
}

impl BaseLayout {
    /// Whether `p` is inside the compound's walls (a tower's footprint).
    pub fn inside(&self, p: Vec3) -> bool {
        (p.x - self.center.x).abs() < self.half && (p.z - self.center.z).abs() < self.half
    }

    /// Whether `p` is inside one of its buildings (the tower).
    pub fn in_building(&self, p: Vec3) -> bool {
        self.interiors.iter().any(|&(a, b)| {
            p.x > a.x && p.x < b.x && p.z > a.z && p.z < b.z && p.y < b.y + 0.5 && p.y > a.y - 1.0
        })
    }
}

#[cfg(test)]
mod tests;
