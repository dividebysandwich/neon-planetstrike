//! A base's structures, from its site.
//!
//! - **Bases** (outposts, center bases, sanctuary homes): a square compound of tall walls you
//!   can walk on (ramps up along the inside of every wall), two vehicle gateways closed to enemy
//!   vehicles by forcefields (infantry walk through), wall turrets, the resource silo, and the
//!   main building in the middle: on the ground floor the spawn room (a forcefield across its
//!   doorway lets only the owners through), the hall with terminals, the core room with the
//!   console that is hacked and the generator room powering the base; usually an upper floor of
//!   offices up a stair ramp; a roof reached from outside. Furnished with crates, racks and
//!   desks. In the yard: the vehicle pads with their terminals and the rearm pads.
//! - **Towers**: three storeys up ramps inside, the capture console on the top floor, turrets
//!   on the roof, a spawn bunker below ground, and an apron round the foot.
//!
//! Ramps are drawn as stairs. The buildings and walls are dressed with drawn-only `Detail`s
//! (angled cornices, plinths, piers and buttresses, framed and glazed windows, string courses,
//! vents, gateway pylons) that nothing collides with.
//!
//! Laid out in the site's frame (its front towards -Z) and turned by quarter turns, so every
//! collider stays axis-aligned (the details needn't be).

mod blocks;
mod builder;
mod facade;
mod plans;

use blocks::{building, compound};
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
    b.out.storeys = storeys;
    compound(&mut b, half, site.kind);
    let points = site.kind == BaseKind::Outpost && site.capture == Capture::Points;
    building(
        &mut b,
        storeys,
        site.kind != BaseKind::Home && !points,
        &mut rng,
    );
    // Paths (base frame): the front gateway, the building's front door, the hall, the doors.
    let c = BLD_C;
    let (z0, z1) = (c.y - BLD_HALF.y, c.y + BLD_HALF.y);
    let (x0, x1) = (c.x - BLD_HALF.x, c.x + BLD_HALF.x);
    let inner = half - WALL_T;
    let gate_out = (0.0, -half - 8.0);
    let gate_in = (0.0, -inner + 5.0);
    let door_out = (0.0, z0 - 3.0);
    let door_in = (0.0, z0 + 2.5);
    // (Between the hall's pillar and the core room's wall.)
    let hall = (x1 - 16.0, z1 - 13.5);
    let core_door = (x1 - 7.0, z1 - 12.0 - 1.5);
    let core_in = (x1 - 7.0, z1 - 12.0 + 2.0);
    let console_front = (x1 - 7.0, z1 - 5.0 - 1.6);
    let way = b.chain(
        None,
        0.0,
        &[
            gate_out,
            gate_in,
            door_out,
            door_in,
            hall,
            core_door,
            core_in,
            console_front,
        ],
    );
    b.mark("gate_out", way[0]);
    b.mark("gate_in", way[1]);
    b.mark("console", way[7]);
    let sdoor_x = x0 + 6.0;
    let sz = z1 - 11.0;
    let out = b.chain(
        Some(way[3]),
        0.0,
        &[(sdoor_x, sz - 2.0), (sdoor_x, sz + 2.0)],
    );
    b.mark("spawn", out[1]);
    b.out.to_console = b.walk("gate_out", "console");
    b.out.from_spawn = b.walk("spawn", "gate_out");
    if points {
        // A in the front yard on the way in, B in the core room (where the console would
        // be), C in the back left of the yard; each a pylon.
        let spots = [
            ((0.0, -30.0), 8.0, (4.0, -30.0), (-2.5, -32.0), Some(vec![])),
            (
                (x1 - 7.0, z1 - 7.0),
                5.0,
                (x1 - 7.0, z1 - 4.0),
                console_front,
                None,
            ),
            (
                (-38.0, 36.0),
                8.0,
                (-38.0, 36.0),
                (-35.5, 33.5),
                Some(vec![(-30.0, -12.0), (-30.0, 24.0)]),
            ),
        ];
        for (at, radius, pylon, stand, via) in spots {
            // (The walk to it: from inside the gateway, or the one to the console.)
            let to = match via {
                Some(mut via) => {
                    via.push(stand);
                    *b.chain(Some(way[1]), 0.0, &via).last().unwrap()
                }
                None => way[7],
            };
            let path = b.out.graph.path(way[0], to).unwrap();
            let pylon = b.p(pylon.0, 0.0, pylon.1);
            b.out
                .colliders
                .push(round_prism(pylon, PYLON_R, PYLON_H, 6, 0.0));
            b.out.styles.push(Style::Pillar);
            let point = CapturePoint {
                pos: b.p(at.0, 0.0, at.1),
                radius,
                pylon,
                stand: b.p(stand.0, 0.0, stand.1),
                path,
            };
            b.out.points.push(point);
        }
    }
    // The yard: vehicle pads with their terminals, rearm pads, the silo.
    type Spots = &'static [(f32, f32)];
    let (pads, rearm, silo): (Spots, Spots, (f32, f32)) = match site.kind {
        BaseKind::Outpost => (&[(-34.0, -36.0)], &[(34.0, -36.0)], (42.0, 34.0)),
        _ => (
            &[(-40.0, -46.0), (40.0, 46.0)],
            &[(42.0, -46.0), (-42.0, 46.0)],
            (58.0, -4.0),
        ),
    };
    for &(x, z) in pads {
        let i = b.out.pads.len();
        let at = b.p(x, 0.0, z);
        let yaw = b.yaw(0.0);
        b.out.pads.push((at, yaw));
        // The terminal beside the pad, facing the pad.
        let tx = x + if x < 0.0 { 11.0 } else { -11.0 };
        let pos = b.p(tx, 0.0, z);
        let yaw = b.yaw(if x < 0.0 {
            -std::f32::consts::FRAC_PI_2
        } else {
            std::f32::consts::FRAC_PI_2
        });
        b.out.terminals.push(Terminal {
            kind: TerminalKind::Vehicle,
            pos,
            yaw,
            pad: Some(i),
        });
    }
    for &(x, z) in rearm {
        let at = b.p(x, 0.0, z);
        b.out.rearm.push(at);
    }
    let s = b.p(silo.0, 0.0, silo.1);
    b.out.silo = Some(s);
    b.out
        .colliders
        .push(round_prism(s - Vec3::Y, SILO_R, SILO_H + 1.0, 8, 0.0));
    b.out.styles.push(Style::Pillar);
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

    /// Whether `p` is inside the main building (the tower).
    pub fn in_building(&self, p: Vec3) -> bool {
        let (a, b) = self.building;
        p.x > a.x && p.x < b.x && p.z > a.z && p.z < b.z && p.y < b.y + 0.5 && p.y > a.y - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collide::World;

    #[test]
    fn spawns_are_free_and_walk_out_of_the_building() {
        for (kind, facing) in [
            (BaseKind::Outpost, 0.0),
            (BaseKind::Center, 1.6),
            (BaseKind::Home, 3.1),
            (BaseKind::Outpost, -1.5),
            (BaseKind::Tower, 1.6),
        ] {
            for seed in 0..4 {
                let site = BaseSite {
                    name: "t".into(),
                    kind,
                    pos: Vec2::new(100.0, -50.0),
                    facing,
                    capture: Capture::Console,
                };
                let l = layout(&site, 10.0, seed);
                assert_eq!(l.colliders.len(), l.styles.len());
                let mut cs = l.colliders.clone();
                if kind != BaseKind::Tower {
                    cs.push(Collider::Box {
                        min: Vec3::new(-500.0, 9.0, -500.0),
                        max: Vec3::new(500.0, 10.0, 500.0),
                    });
                }
                let w = World::new(cs, None);
                for &(s, _) in &l.spawns {
                    let mut p = s;
                    assert!(
                        !w.colliders
                            .iter()
                            .any(|c| crate::collide::push_out(c, &mut p)),
                        "{kind:?} spawn {s} blocked"
                    );
                    assert!(l.in_building(s), "{kind:?} spawn {s} outside");
                    // Something to stand on.
                    let g = w.ground(s.x, s.z, s.y + 0.2).unwrap();
                    assert!((g - s.y).abs() < 0.05, "{kind:?} spawn {s} floor {g}");
                }
                assert_eq!(l.console.is_some(), kind != BaseKind::Home);
                assert!(
                    l.terminals
                        .iter()
                        .any(|t| t.kind == TerminalKind::Equipment)
                );
                if kind != BaseKind::Tower {
                    assert!(l.terminals.iter().any(|t| t.kind == TerminalKind::Vehicle));
                    assert!(l.generator.is_some() && l.silo.is_some());
                    assert_eq!(l.gate_fields.len(), 2);
                } else {
                    assert_eq!(l.holes.len(), 1);
                }
            }
        }
    }

    #[test]
    fn capture_points_are_clear_and_bots_reach_them() {
        for facing in [0.0, 1.6, -1.5, 3.1] {
            for seed in 0..3 {
                let site = BaseSite {
                    name: "t".into(),
                    kind: BaseKind::Outpost,
                    pos: Vec2::new(100.0, -50.0),
                    facing,
                    capture: Capture::Points,
                };
                let l = layout(&site, 0.0, seed);
                assert!(l.console.is_none());
                assert_eq!(l.points.len(), 3);
                let mut cs = l.colliders.clone();
                cs.push(Collider::Box {
                    min: Vec3::new(-500.0, -1.0, -500.0),
                    max: Vec3::new(500.0, 0.0, 500.0),
                });
                let w = World::new(cs, None);
                for (i, pt) in l.points.iter().enumerate() {
                    // The stand spot is clear, on the floor, in the zone.
                    let mut p = pt.stand;
                    assert!(
                        !w.colliders
                            .iter()
                            .any(|c| crate::collide::push_out(c, &mut p)),
                        "point {i} at {facing}: stand {} blocked",
                        pt.stand
                    );
                    assert!(pt.stand.with_y(0.0).distance(pt.pos.with_y(0.0)) < pt.radius * 0.6);
                    assert!(pt.pylon.with_y(0.0).distance(pt.pos.with_y(0.0)) < pt.radius);
                    // Walking the path gets there.
                    let mut feet = pt.path[0];
                    let mut vy = 0.0;
                    for &target in &pt.path[1..] {
                        for _ in 0..2400 {
                            let d = Vec2::new(target.x - feet.x, target.z - feet.z);
                            if d.length() < 0.4 {
                                break;
                            }
                            let step = d.normalize() * (5.0f32 / 60.0).min(d.length());
                            (feet, vy) = w.move_body(feet, step, vy, 1.0 / 60.0);
                        }
                        assert!(
                            Vec2::new(target.x - feet.x, target.z - feet.z).length() < 0.6,
                            "point {i} at {facing}: stuck at {feet} on the way to {target}"
                        );
                    }
                    // The zones are apart, and B is inside.
                    assert_eq!(l.in_building(pt.pos), i == 1, "point {i}");
                }
            }
        }
    }

    #[test]
    fn gateways_are_where_the_forcefields_are() {
        for q in 0..4 {
            let site = BaseSite {
                name: "t".into(),
                kind: BaseKind::Outpost,
                pos: Vec2::new(100.0, -50.0),
                facing: q as f32 * std::f32::consts::FRAC_PI_2,
                capture: Capture::Console,
            };
            let l = layout(&site, 0.0, 1);
            let gw = gateways(&site);
            assert_eq!(gw.len(), 2);
            for f in &l.gate_fields {
                let Collider::Box { min, max } = *f else {
                    panic!()
                };
                let mid = Vec2::new(min.x + max.x, min.z + max.z) / 2.0;
                // Each forcefield just inside a gateway, the way out away from the middle.
                assert!(
                    gw.iter()
                        .any(|(at, out)| at.distance(mid) < WALL_T
                            && (*at - site.pos).dot(*out) > 0.0),
                    "{q}: {mid} {gw:?}"
                );
            }
        }
    }

    #[test]
    fn bots_paths_reach_the_console() {
        // Walking each path point to point gets to the console (up the ramps in a tower).
        for kind in [BaseKind::Outpost, BaseKind::Center, BaseKind::Tower] {
            let site = BaseSite {
                name: "t".into(),
                kind,
                pos: Vec2::ZERO,
                facing: 0.0,
                capture: Capture::Console,
            };
            let l = layout(&site, 0.0, 1);
            let mut cs = l.colliders.clone();
            if kind == BaseKind::Tower {
                // (The land around the tower; the bunker's hole in it.)
                for (a, e) in [
                    (Vec2::new(-300.0, -300.0), Vec2::new(300.0, -8.5)),
                    (Vec2::new(-300.0, 8.5), Vec2::new(300.0, 300.0)),
                    (Vec2::new(-300.0, -8.5), Vec2::new(-8.5, 8.5)),
                    (Vec2::new(8.5, -8.5), Vec2::new(300.0, 8.5)),
                ] {
                    cs.push(Collider::Box {
                        min: Vec3::new(a.x, -1.0, a.y),
                        max: Vec3::new(e.x, 0.0, e.y),
                    });
                }
            } else {
                cs.push(Collider::Box {
                    min: Vec3::new(-300.0, -1.0, -300.0),
                    max: Vec3::new(300.0, 0.0, 300.0),
                });
            }
            let w = World::new(cs, None);
            let mut feet = l.to_console[0];
            let mut vy = 0.0;
            for &target in &l.to_console[1..] {
                for _ in 0..1200 {
                    let d = Vec2::new(target.x - feet.x, target.z - feet.z);
                    if d.length() < 0.4 {
                        break;
                    }
                    let step = d.normalize() * (5.0f32 / 60.0).min(d.length());
                    (feet, vy) = w.move_body(feet, step, vy, 1.0 / 60.0);
                }
                assert!(
                    Vec2::new(target.x - feet.x, target.z - feet.z).length() < 0.6,
                    "{kind:?}: stuck at {feet} on the way to {target}"
                );
            }
            let (console, _) = l.console.unwrap();
            assert!(
                (console.y - feet.y).abs() < 0.3,
                "{kind:?}: at {feet}, console {console}"
            );
            assert!(
                (console - feet).length() < USE_REACH,
                "{kind:?}: {feet} vs {console}"
            );
        }
    }

    /// A fingerprint of a layout's solid parts and fixtures.
    fn fingerprint(l: &BaseLayout) -> u64 {
        let mut h = 0xcbf29ce484222325u64;
        let mut eat = |v: f32| {
            // (Rounded to the millimetre.)
            for b in ((v * 1000.0).round() as i64).to_le_bytes() {
                h = (h ^ b as u64).wrapping_mul(0x100000001b3);
            }
        };
        for c in &l.colliders {
            let (a, b) = c.bounds();
            for v in [a, b] {
                eat(v.x);
                eat(v.y);
                eat(v.z);
            }
            if let Collider::Ramp { dir, .. } = c {
                eat(*dir as f32);
            }
        }
        for t in &l.terminals {
            eat(t.pos.x);
            eat(t.pos.y);
            eat(t.pos.z);
            eat(t.yaw);
        }
        for (s, yaw) in &l.spawns {
            eat(s.x);
            eat(s.y);
            eat(s.z);
            eat(*yaw);
        }
        h
    }

    #[test]
    fn layouts_are_the_same_every_time() {
        // Clients build the bases from the seed: the same layout for the same site and seed,
        // here and after any change that isn't meant to move a thing (the fingerprints).
        let mut got = Vec::new();
        for (kind, facing) in [
            (BaseKind::Outpost, 0.0),
            (BaseKind::Center, 1.6),
            (BaseKind::Home, 3.1),
            (BaseKind::Tower, -1.5),
        ] {
            for seed in 0..4 {
                let site = BaseSite {
                    name: "t".into(),
                    kind,
                    pos: Vec2::new(100.0, -50.0),
                    facing,
                    capture: Capture::Console,
                };
                let a = fingerprint(&layout(&site, 10.0, seed));
                assert_eq!(a, fingerprint(&layout(&site, 10.0, seed)));
                got.push(a);
            }
        }
        assert_eq!(got, GOLDEN, "{got:#x?}");
    }

    const GOLDEN: [u64; 16] = [
        0x338b152e3677b8a6,
        0xfc04f2994d3bd66b,
        0xdbc9c14bb2a78a81,
        0x4519062d9512e140,
        0x58eea4fb5931979c,
        0x234133e24aab2288,
        0xce5bf4bcf896b3f1,
        0xc9f3df3e601fb93d,
        0x3abb033112895c1,
        0x57b36436df20a081,
        0xb93f46476fa111b6,
        0xfef91209970f0762,
        0x32315e7d655bf02c,
        0x32315e7d655bf02c,
        0x32315e7d655bf02c,
        0x32315e7d655bf02c,
    ];
}
