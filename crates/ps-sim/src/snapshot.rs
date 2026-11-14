//! What a client is sent of its continent every few ticks: the players and vehicles it can
//! see, its own state in full, and the effects (shots, impacts) since the last one. Positions
//! and angles go over the wire quantized (`q`), so a snapshot of dozens of players stays small.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::types::{Hand, PlayerId, Team, VehicleId};
use crate::vehicle::VehicleKind;
use crate::weapons::{Ammo, Loadout, Weapon};

/// Quantizing serde helpers.
pub mod q {
    use glam::{Quat, Vec3};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Positions in centimetres.
    pub mod pos {
        use super::*;
        pub fn serialize<S: Serializer>(v: &Vec3, s: S) -> Result<S::Ok, S::Error> {
            let q = |f: f32| (f * 100.0).round() as i32;
            (q(v.x), q(v.y), q(v.z)).serialize(s)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec3, D::Error> {
            let (x, y, z) = <(i32, i32, i32)>::deserialize(d)?;
            Ok(Vec3::new(x as f32, y as f32, z as f32) * 0.01)
        }
    }

    /// Small offsets (relative to the feet) in millimetres.
    pub mod offset {
        use super::*;
        pub fn serialize<S: Serializer>(v: &Vec3, s: S) -> Result<S::Ok, S::Error> {
            let q = |f: f32| (f * 1000.0).round().clamp(-32000.0, 32000.0) as i16;
            (q(v.x), q(v.y), q(v.z)).serialize(s)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec3, D::Error> {
            let (x, y, z) = <(i16, i16, i16)>::deserialize(d)?;
            Ok(Vec3::new(x as f32, y as f32, z as f32) * 0.001)
        }
    }

    /// Velocities in decimetres per second.
    pub mod vel {
        use super::*;
        pub fn serialize<S: Serializer>(v: &Vec3, s: S) -> Result<S::Ok, S::Error> {
            let q = |f: f32| (f * 10.0).round() as i32;
            (q(v.x), q(v.y), q(v.z)).serialize(s)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec3, D::Error> {
            let (x, y, z) = <(i32, i32, i32)>::deserialize(d)?;
            Ok(Vec3::new(x as f32, y as f32, z as f32) * 0.1)
        }
    }

    /// Angles in 1/10000 radian.
    pub mod angle {
        use super::*;
        pub fn serialize<S: Serializer>(a: &f32, s: S) -> Result<S::Ok, S::Error> {
            let w = crate::types::wrap_angle(*a);
            ((w * 10000.0).round() as i16).serialize(s)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<f32, D::Error> {
            Ok(i16::deserialize(d)? as f32 / 10000.0)
        }
    }

    /// Rotations as four i16 components.
    pub mod rot {
        use super::*;
        pub fn serialize<S: Serializer>(r: &Quat, s: S) -> Result<S::Ok, S::Error> {
            let r = if r.is_finite() {
                r.normalize()
            } else {
                Quat::IDENTITY
            };
            let q = |f: f32| (f * 32000.0).round() as i16;
            (q(r.x), q(r.y), q(r.z), q(r.w)).serialize(s)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Quat, D::Error> {
            let (x, y, z, w) = <(i16, i16, i16, i16)>::deserialize(d)?;
            let q = Quat::from_xyzw(x as f32, y as f32, z as f32, w as f32);
            Ok(if q.length_squared() > 0.0 {
                q.normalize()
            } else {
                Quat::IDENTITY
            })
        }
    }

    /// 0..100 (health, armor) in a byte.
    pub mod percent {
        use super::*;
        pub fn serialize<S: Serializer>(v: &f32, s: S) -> Result<S::Ok, S::Error> {
            (v.clamp(0.0, 100.0).ceil() as u8).serialize(s)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<f32, D::Error> {
            Ok(u8::deserialize(d)? as f32)
        }
    }
}

/// A pose relative to the feet (VR heads and hands).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct QPose {
    #[serde(with = "q::offset")]
    pub pos: Vec3,
    #[serde(with = "q::rot")]
    pub rot: Quat,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VrPose {
    pub head: QPose,
    pub hands: [QPose; 2],
    pub gun_hand: Option<Hand>,
}

pub mod flags {
    pub const CROUCH: u16 = 1;
    pub const FIRING: u16 = 2;
    pub const DEAD: u16 = 4;
    pub const BEAM: u16 = 8;
    pub const SPRINT: u16 = 16;
    pub const RELOAD: u16 = 32;
    pub const AIR: u16 = 64;
    /// Just spawned: protected for a moment.
    pub const FRESH: u16 = 128;
    /// Dead, and a medic of its faction's can still revive it (sent to its faction only).
    pub const DOWNED: u16 = 256;
    /// Wears a MAX suit.
    pub const MAX: u16 = 512;
    /// Cloaked (an enemy sees it only near, and faint).
    pub const CLOAKED: u16 = 1024;
    /// Wears the infiltration suit.
    pub const STEALTH: u16 = 2048;
    /// Hacking something.
    pub const HACKING: u16 = 4096;
    /// An enemy the viewer's faction spotted, or its interlink sees (near one of its bases).
    pub const SPOTTED: u16 = 8192;
    /// Its jump jets firing.
    pub const JET: u16 = 16384;
    /// Falling under a chute.
    pub const CHUTE: u16 = 32768;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerSnap {
    pub id: PlayerId,
    pub team: Team,
    #[serde(with = "q::pos")]
    pub feet: Vec3,
    #[serde(with = "q::angle")]
    pub yaw: f32,
    #[serde(with = "q::angle")]
    pub pitch: f32,
    pub flags: u16,
    pub weapon: Weapon,
    #[serde(with = "q::percent")]
    pub health: f32,
    /// In a vehicle: which and which seat (the body goes where the seat is).
    pub seat: Option<(VehicleId, u8)>,
    pub vr: Option<VrPose>,
}

impl PlayerSnap {
    pub fn has(&self, f: u16) -> bool {
        self.flags & f != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GunSnap {
    pub mag: u16,
    pub reserve: u16,
    /// Seconds of reloading left (0: not reloading).
    pub reload: f32,
}

/// The receiving player's own state, in full.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeSnap {
    #[serde(with = "q::pos")]
    pub feet: Vec3,
    pub vy: f32,
    pub health: f32,
    pub armor: f32,
    pub loadout: Loadout,
    pub slot: u8,
    pub guns: [GunSnap; crate::weapons::SLOTS],
    pub grenades: u8,
    /// Dead: seconds until a spawn may be chosen (0: now).
    pub dead: Option<f32>,
    /// Dead and revivable: seconds left in which a medic may revive, and how far along a
    /// revive is (0..1).
    pub downed: Option<(f32, f32)>,
    pub seat: Option<(VehicleId, u8)>,
    /// The seat's gun: rounds left, of how many (none for a seat without one).
    pub seat_ammo: Option<(u16, u16)>,
    /// The seat's second gun (missiles), likewise.
    pub alt_ammo: Option<(u16, u16)>,
    /// Manning a turret (its index on the continent).
    pub turret: Option<u16>,
    /// Hanging on a zipline (its index on the continent).
    pub zip: Option<u16>,
    pub kills: u16,
    pub deaths: u16,
    pub assists: u16,
    /// XP earned this session.
    pub score: u32,
    /// The player's lock-on: the vehicle, and how far along (0..1, locked at 1).
    pub lock: Option<(VehicleId, f32)>,
    /// The vehicle the player is in: 1 an enemy is locking on, 2 a missile is coming; and
    /// (aircraft) seconds until its flares are ready again.
    pub warning: u8,
    pub flare_wait: f32,
    /// The infiltrator's cloak: how full (0..1).
    pub cloak: f32,
    /// A hack under way: how far along (0..1).
    pub hack: Option<f32>,
    /// Deployables left to put down (the loadout's kit).
    pub kit_left: u8,
    /// A structure pulled from a harvester's terminal, carried to put down.
    pub carrying: Option<crate::construct::StructureKind>,
    /// Jump jet fuel (0..1), and falling under a chute.
    pub fuel: f32,
    pub chute: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VehicleSnap {
    pub id: VehicleId,
    pub kind: VehicleKind,
    pub team: Team,
    #[serde(with = "q::pos")]
    pub pos: Vec3,
    #[serde(with = "q::rot")]
    pub rot: Quat,
    #[serde(with = "q::vel")]
    pub vel: Vec3,
    /// The turrets' aim (yaw relative to the hull, pitch), per gun seat.
    pub aim: Vec<(i16, i16)>,
    #[serde(with = "q::percent")]
    pub health: f32,
    /// Occupants by seat.
    pub seats: Vec<Option<PlayerId>>,
    pub flags: u8,
    /// A harvester's cargo (resources, up to `sim::CARGO`).
    pub cargo: u8,
    /// Its main gun's choice and its defensive slot.
    pub loadout: crate::vehicle::VehicleLoadout,
}

pub mod vflags {
    /// The spawn bus is deployed.
    pub const DEPLOYED: u8 = 1;
    /// Cloaked (deployed bus) as far as this viewer is concerned (drawn faint).
    pub const CLOAKED: u8 = 2;
    pub const FIRING: u8 = 4;
    pub const BURNING: u8 = 8;
    /// A harvester's beam is on.
    pub const HARVESTING: u8 = 16;
    /// A harvester is unloading into a silo.
    pub const UNLOADING: u8 = 32;
    /// An aircraft's flares are burning.
    pub const FLARES: u8 = 64;
    /// Spotted by the viewer's faction.
    pub const SPOTTED: u8 = 128;
}

/// What a client draws and hears without needing it reliably.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Fx {
    /// A projectile left `from` with `vel` (the client flies it until its `Impact` or end).
    Shot {
        id: u32,
        owner: PlayerId,
        ammo: Ammo,
        #[serde(with = "q::pos")]
        from: Vec3,
        #[serde(with = "q::vel")]
        vel: Vec3,
        /// A missile homing on this vehicle.
        target: Option<VehicleId>,
    },
    /// Projectile `id` ended at `at`: on the ground, a body, a vehicle (`what`), exploding if
    /// it has splash.
    Impact {
        id: u32,
        ammo: Ammo,
        #[serde(with = "q::pos")]
        at: Vec3,
        what: ImpactOn,
    },
    /// A vehicle blew up.
    Wreck {
        #[serde(with = "q::pos")]
        at: Vec3,
        kind: VehicleKind,
    },
    /// A turret fired (its bolt follows as a `Shot`).
    Muzzle {
        #[serde(with = "q::pos")]
        at: Vec3,
        ammo: Ammo,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImpactOn {
    Ground,
    Body,
    Vehicle,
    Shield,
    Air,
}

impl Fx {
    /// Where it happens (to send it only to those near).
    pub fn at(&self) -> Vec3 {
        match *self {
            Fx::Shot { from, .. } => from,
            Fx::Impact { at, .. } | Fx::Wreck { at, .. } | Fx::Muzzle { at, .. } => at,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TurretSnap {
    pub index: u8,
    pub base: u8,
    #[serde(with = "q::angle")]
    pub yaw: f32,
    #[serde(with = "q::angle")]
    pub pitch: f32,
    #[serde(with = "q::percent")]
    pub health: f32,
    pub team: Option<Team>,
    pub occupant: Option<PlayerId>,
    /// Powered (the base's generator up, resources left).
    pub powered: bool,
}

/// A base's state seen from its continent: the generator's health and the silo's resources.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BaseSnap {
    #[serde(with = "q::percent")]
    pub generator: f32,
    pub resources: u16,
    /// Terminals hacked (a bit per terminal) by `hacker`'s faction (the latest's).
    pub hacked: u16,
    pub hacker: Option<Team>,
    /// Its capture points A, B, C (where it's taken at them).
    pub points: Option<[PointSnap; 3]>,
}

/// A capture point: who holds it, who is taking it and how far (percent), and whether two
/// factions are in it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PointSnap {
    pub owner: Option<Team>,
    pub by: Option<Team>,
    pub progress: u8,
    pub contested: bool,
}

/// A built structure.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StructSnap {
    pub id: u32,
    pub kind: crate::construct::StructureKind,
    pub team: Team,
    #[serde(with = "q::pos")]
    pub pos: Vec3,
    #[serde(with = "q::angle")]
    pub yaw: f32,
    /// A Bastion's aim (world yaw, pitch).
    #[serde(with = "q::angle")]
    pub aim_yaw: f32,
    #[serde(with = "q::angle")]
    pub aim_pitch: f32,
    #[serde(with = "q::percent")]
    pub health: f32,
    pub firing: bool,
    /// A silo's fill (percent).
    pub fill: u8,
    /// Kept going by a silo (a silo: not empty); else decaying.
    pub supplied: bool,
}

/// An engineer's deployable.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeploySnap {
    pub id: u32,
    pub kind: crate::weapons::Kit,
    pub team: Team,
    pub owner: PlayerId,
    #[serde(with = "q::pos")]
    pub pos: Vec3,
    /// A spitfire's aim (yaw, pitch); others: its facing.
    #[serde(with = "q::angle")]
    pub yaw: f32,
    #[serde(with = "q::angle")]
    pub pitch: f32,
    #[serde(with = "q::percent")]
    pub health: f32,
    pub firing: bool,
}

/// A resource node to harvest.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeSnap {
    pub id: u32,
    #[serde(with = "q::pos")]
    pub pos: Vec3,
    pub amount: u16,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub tick: u64,
    pub players: Vec<PlayerSnap>,
    pub vehicles: Vec<VehicleSnap>,
    pub turrets: Vec<TurretSnap>,
    pub bases: Vec<BaseSnap>,
    pub nodes: Vec<NodeSnap>,
    pub deployables: Vec<DeploySnap>,
    pub structures: Vec<StructSnap>,
    pub me: Option<MeSnap>,
    pub fx: Vec<Fx>,
    /// Not refreshed in this one (far away): as last heard of (see `ps_net::delta`).
    #[serde(skip)]
    pub held_players: Vec<PlayerId>,
    #[serde(skip)]
    pub held_vehicles: Vec<VehicleId>,
}
