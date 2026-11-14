//! One continent's simulation, stepped at `TICK` by its own thread on the server: players
//! (infantry and seated), vehicles, wall turrets, projectiles, consoles and terminals. Who owns
//! the bases comes from the server's world state (`set_base`); what changes it (a console
//! solved, a gate entered) goes back in `solved` and `warps`.
//!
//! Projectiles are flown here alone, with lag compensation: a shot from a client flies
//! through the other players where that client saw them (up to `MAX_LAG` ticks back).

use std::collections::VecDeque;
use std::sync::Arc;

use glam::{Quat, Vec2, Vec3};

use crate::base::{
    GATE_CORE, GATE_H, GENERATOR_HALF, PAD_RADIUS, REARM_RADIUS, SILO_REACH, TURRET_BARREL,
    TURRET_EYE, TURRET_PIVOT, TerminalKind, USE_REACH, in_gate_bubble,
};
use crate::capture::PointRt;
use crate::collide::{Collider, World};
use crate::events::{Audience, Cause, Deny, Outgoing, Request, SimEvent, SpawnPoint};
use crate::map::ContinentMap;
use crate::noise::{self, Seq};
use crate::player::{self, Body, eye_height};
use crate::progress::{self, Award};
use crate::puzzle::Puzzle;
use crate::snapshot::{
    Fx, GunSnap, ImpactOn, MeSnap, PlayerSnap, QPose, Snapshot, TurretSnap, VehicleSnap, VrPose,
    flags, vflags,
};
use crate::state::Hack;
use crate::types::{PlayerCmd, PlayerId, Team, VehicleId, look_dir, wrap_angle, yaw_of};
use crate::vehicle::{self, Motion, MotionKind, VehicleKind};
use crate::weapons::{self, Ammo, Loadout, ShotKind, Weapon};
use crate::worldgen::{BaseIdx, BaseKind, ContinentId, Facility};

pub const TICK: f32 = 1.0 / 60.0;
pub const RESPAWN_TIME: f32 = 8.0;
pub const SPAWN_PROTECT: f32 = 2.0;
/// Seconds an aircraft's flares burn, and between them.
pub const FLARE_TIME: f32 = 3.0;
pub const FLARE_WAIT: f32 = 12.0;
/// Seconds after falling on foot in which a medic may revive a player where it fell.
pub const REVIVE_WINDOW: f32 = 30.0;
/// Seconds of the medgun's beam on a body that revive it.
pub const REVIVE_TIME: f32 = 2.5;
/// Health a revived player comes back with (no armor).
pub const REVIVE_HEALTH: f32 = 40.0;
/// How near the body (m) a medic of its faction has to be for a dead bot to wait for it.
pub const REVIVE_WAIT_RANGE: f32 = 40.0;
/// Ticks of positions kept for lag compensation, and the most rewound.
pub const HISTORY: usize = 40;
pub const MAX_LAG: u64 = 30;
/// How far a client sees: infantry, ground vehicles, aircraft.
pub const VIEW_INFANTRY: f32 = 500.0;
pub const VIEW_VEHICLE: f32 = 900.0;
pub const VIEW_AIR: f32 = 1400.0;
/// A deployed spawn bus is seen by enemies only this close.
pub const CLOAK_SEEN: f32 = 25.0;
/// VR heads and hands are sent this close.
pub const POSE_RANGE: f32 = 60.0;
/// Ground vehicles faster than this run infantry over.
pub const ROADKILL_SPEED: f32 = 4.0;
/// Empty vehicles go after this long.
pub const ABANDON_TIME: f32 = 240.0;
/// Most vehicles from one base's pads alive at once.
pub const VEHICLES_PER_BASE: usize = 6;

pub const TURRET_RANGE: f32 = 130.0;
pub const TURRET_AIR_RANGE: f32 = 260.0;
/// Seconds between a manned turret's shots; left to itself it fires at `TURRET_AUTO_RATE` of
/// that rate.
pub const TURRET_INTERVAL: f32 = 0.24;
pub const TURRET_AUTO_RATE: f32 = 0.3;
/// Turrets miss by this much (cone half-angle).
pub const TURRET_SPREAD: f32 = 0.02;
/// How far a manned turret turns (rad/s), and how far it looks down and up.
pub const TURRET_TURN: f32 = 3.0;
pub const TURRET_PITCH: (f32, f32) = (-0.9, 1.2);
pub const TURRET_DOWN_TIME: f32 = 300.0;
/// What a turret's shot takes from its base's silo.
pub const TURRET_SHOT_COST: f32 = 0.15;
/// Resource nodes on a continent at once, and what a harvester carries and how fast it mines
/// and unloads (per second).
pub const NODES: usize = 8;
pub const CARGO: f32 = crate::state::SILO_CAP / 6.0;
/// What a node holds: one of these shares of a harvester's load.
pub const NODE_SIZES: [f32; 3] = [0.3, 0.5, 0.7];
/// A mined out node grows back somewhere after this long (s, at random between).
pub const NODE_REGROW: (f32, f32) = (60.0, 180.0);
pub const HARVEST_RATE: f32 = 10.0;
pub const UNLOAD_RATE: f32 = 25.0;
pub const HARVEST_REACH: f32 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gun {
    pub weapon: Weapon,
    pub mag: u16,
    pub reserve: u16,
    pub cooldown: f32,
    pub reload: f32,
}

impl Gun {
    pub fn new(weapon: Weapon) -> Gun {
        let s = weapon.stats();
        Gun {
            weapon,
            mag: s.mag,
            reserve: s.reserve,
            cooldown: 0.0,
            reload: 0.0,
        }
    }

    fn snap(&self) -> GunSnap {
        GunSnap {
            mag: self.mag,
            reserve: self.reserve,
            reload: self.reload,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub team: Team,
    pub bot: bool,
    pub body: Body,
    pub cmd: PlayerCmd,
    prev_fire: bool,
    reloads_seen: Option<u32>,
    throws_seen: Option<u32>,
    deploys_seen: Option<u32>,
    switches_seen: Option<u32>,
    pub health: f32,
    pub armor: f32,
    pub loadout: Loadout,
    pub guns: [Gun; weapons::SLOTS],
    pub grenades: u8,
    pub grenade_cooldown: f32,
    /// Dead for this long (None: alive).
    pub dead: Option<f32>,
    /// Fell on foot: a medic may revive it within `REVIVE_WINDOW`.
    pub downed: bool,
    /// Seconds of a medgun's beam on the body so far, and the tick it last had it.
    pub revive: f32,
    revive_tick: u64,
    pub protect: f32,
    pub seat: Option<(VehicleId, u8)>,
    /// Manning a turret (its index).
    pub turret: Option<usize>,
    pub beaming: bool,
    pub firing: bool,
    pub history: VecDeque<(u64, Vec3, bool)>,
    pub kills: u16,
    pub deaths: u16,
    pub assists: u16,
    /// XP earned this session (it goes with the player from continent to continent).
    pub score: u32,
    /// A lock-on (a launcher in hand, or the seat's missiles): the vehicle, and seconds held
    /// in the cone (locked at `weapons::LOCK_TIME`).
    pub lock: Option<(VehicleId, f32)>,
    /// Recent damage taken from enemies: who, how much, when (tick); for assists.
    hurt_by: Vec<(PlayerId, f32, u64)>,
    /// Heal, repair and delivery XP not yet given (`progress::LUMP`).
    heal_xp: f32,
    repair_xp: f32,
    deliver_xp: f32,
    /// Seconds until it may spawn another vehicle.
    pub vehicle_cooldown: f32,
    pub shots: u32,
    /// The infiltrator's cloak: how full (0..1), and whether it's on.
    pub cloak: f32,
    pub cloaked: bool,
    /// A hack under way: what, and seconds at it so far.
    pub hacking: Option<(crate::events::HackTarget, f32)>,
    /// Deployables left to put down.
    pub kit_left: u8,
    /// The tick it may spot again.
    pub spot_ready: u64,
    /// A structure pulled from a harvester's terminal, carried to put down.
    pub carrying: Option<crate::construct::StructureKind>,
    /// Blinded by a flash grenade for this long more (s).
    pub flashed: f32,
}

impl Player {
    pub fn new(id: PlayerId, name: &str, team: Team, bot: bool) -> Player {
        let loadout = Loadout::default();
        Player {
            id,
            name: name.to_string(),
            team,
            bot,
            body: Body::default(),
            cmd: PlayerCmd::default(),
            prev_fire: false,
            reloads_seen: None,
            throws_seen: None,
            deploys_seen: None,
            switches_seen: None,
            health: 0.0,
            armor: 0.0,
            loadout,
            guns: loadout.weapons().map(Gun::new),
            grenades: weapons::GRENADES,
            grenade_cooldown: 0.0,
            dead: Some(RESPAWN_TIME),
            downed: false,
            revive: 0.0,
            revive_tick: 0,
            protect: 0.0,
            seat: None,
            turret: None,
            beaming: false,
            firing: false,
            history: VecDeque::new(),
            kills: 0,
            deaths: 0,
            assists: 0,
            score: 0,
            lock: None,
            hurt_by: Vec::new(),
            heal_xp: 0.0,
            repair_xp: 0.0,
            deliver_xp: 0.0,
            cloak: 1.0,
            cloaked: false,
            hacking: None,
            kit_left: 0,
            spot_ready: 0,
            carrying: None,
            flashed: 0.0,
            vehicle_cooldown: 0.0,
            shots: 0,
        }
    }

    pub fn alive(&self) -> bool {
        self.dead.is_none()
    }

    /// Dead where it fell on foot, and a medic may still revive it.
    pub fn revivable(&self) -> bool {
        self.downed && self.dead.is_some_and(|t| t < REVIVE_WINDOW)
    }

    pub fn feet(&self) -> Vec3 {
        self.body.feet
    }

    pub fn eye(&self) -> Vec3 {
        self.body.feet + Vec3::Y * eye_height(self.cmd.crouch)
    }

    /// On foot and alive (hit by shots, runs, walks).
    pub fn afoot(&self) -> bool {
        self.alive() && self.seat.is_none() && self.turret.is_none()
    }

    pub fn slot(&self) -> usize {
        if self.loadout.usable(self.cmd.slot) {
            self.cmd.slot as usize
        } else {
            0
        }
    }

    pub fn weapon(&self) -> Weapon {
        self.guns[self.slot()].weapon
    }

    /// Re-arms: the loadout's guns full, armor full.
    pub fn refill(&mut self) {
        self.guns = self.loadout.weapons().map(Gun::new);
        // (A MAX throws no grenades.)
        self.grenades = if self.loadout.max() {
            0
        } else {
            weapons::GRENADES
        };
        self.armor = self.loadout.armor.max();
        self.kit_left = self.loadout.kit().count();
        self.cloak = 1.0;
    }

    /// Where the player was `ticks` ago (lag compensation): feet and crouching.
    fn at_tick(&self, tick: u64) -> (Vec3, bool) {
        for &(t, f, c) in self.history.iter().rev() {
            if t <= tick {
                return (f, c);
            }
        }
        (self.body.feet, self.cmd.crouch)
    }

    fn snap(&self, near: bool) -> PlayerSnap {
        let mut f = 0u16;
        if self.cmd.crouch {
            f |= flags::CROUCH;
        }
        if self.firing {
            f |= flags::FIRING;
        }
        if !self.alive() {
            f |= flags::DEAD;
        }
        if self.beaming {
            f |= flags::BEAM;
        }
        if player::sprinting(&self.cmd) {
            f |= flags::SPRINT;
        }
        if self.guns[self.slot()].reload > 0.0 {
            f |= flags::RELOAD;
        }
        if !self.body.on_ground() {
            f |= flags::AIR;
        }
        if self.protect > 0.0 {
            f |= flags::FRESH;
        }
        if self.revivable() {
            f |= flags::DOWNED;
        }
        match self.loadout.armor {
            weapons::ArmorKind::Max => f |= flags::MAX,
            weapons::ArmorKind::Stealth => f |= flags::STEALTH,
            _ => {}
        }
        if self.cloaked {
            f |= flags::CLOAKED;
        }
        if self.hacking.is_some() {
            f |= flags::HACKING;
        }
        if self.body.jetting {
            f |= flags::JET;
        }
        if self.body.chute {
            f |= flags::CHUTE;
        }
        PlayerSnap {
            id: self.id,
            team: self.team,
            feet: self.body.feet,
            yaw: self.cmd.yaw,
            pitch: self.cmd.pitch,
            flags: f,
            weapon: self.weapon(),
            health: self.health,
            seat: self.seat,
            vr: self.cmd.vr.filter(|_| near).map(|v| VrPose {
                head: QPose {
                    pos: v.head.pos,
                    rot: v.head.rot,
                },
                hands: [
                    QPose {
                        pos: v.hands[0].pos,
                        rot: v.hands[0].rot,
                    },
                    QPose {
                        pos: v.hands[1].pos,
                        rot: v.hands[1].rot,
                    },
                ],
                gun_hand: v.gun_hand,
            }),
        }
    }

    fn me(&self) -> MeSnap {
        MeSnap {
            feet: self.body.feet,
            vy: self.body.vy,
            health: self.health,
            armor: self.armor,
            loadout: self.loadout,
            slot: self.slot() as u8,
            guns: self.guns.map(|g| g.snap()),
            grenades: self.grenades,
            dead: self.dead.map(|t| (RESPAWN_TIME - t).max(0.0)),
            downed: self.revivable().then(|| {
                (
                    REVIVE_WINDOW - self.dead.unwrap_or(0.0),
                    (self.revive / REVIVE_TIME).clamp(0.0, 1.0),
                )
            }),
            seat: self.seat,
            seat_ammo: None,
            alt_ammo: None,
            turret: self.turret.map(|t| t as u16),
            kills: self.kills,
            deaths: self.deaths,
            assists: self.assists,
            score: self.score,
            lock: self
                .lock
                .map(|(v, held)| (v, (held / weapons::LOCK_TIME).min(1.0))),
            warning: 0,
            flare_wait: 0.0,
            cloak: self.cloak,
            hack: self
                .hacking
                .map(|(_, t)| (t / weapons::HACK_TIME).clamp(0.0, 1.0)),
            kit_left: self.kit_left,
            carrying: self.carrying,
            fuel: self.body.fuel,
            chute: self.body.chute,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VGun {
    pub ammo: u16,
    pub cooldown: f32,
    /// Aim relative to the hull: yaw and pitch.
    pub yaw: f32,
    pub pitch: f32,
    pub barrel: u8,
}

#[derive(Clone, Debug)]
pub struct Vehicle {
    pub id: VehicleId,
    pub kind: VehicleKind,
    pub team: Team,
    pub motion: Motion,
    pub health: f32,
    pub seats: Vec<Option<PlayerId>>,
    pub guns: Vec<VGun>,
    pub deployed: bool,
    pub empty_for: f32,
    /// The base whose pad it came from.
    pub from_base: Option<BaseIdx>,
    /// Who ordered it (spawns on a bus earn it XP).
    pub owner: Option<PlayerId>,
    pub history: VecDeque<(u64, Vec3, Quat)>,
    pub firing: bool,
    /// A harvester's load, and whether its beam is on.
    pub cargo: f32,
    pub harvesting: bool,
    /// Unloading into a silo this tick.
    pub unloading: bool,
    /// Aircraft: flares burning (seconds left; locks break, missiles go astray), and seconds
    /// until the next can go.
    pub flares: f32,
    /// Seconds before its defensive slot may be used again (flares among them).
    pub flare_wait: f32,
    /// Its main gun's choice and its defensive slot.
    pub loadout: vehicle::VehicleLoadout,
}

impl Vehicle {
    pub fn new(id: VehicleId, kind: VehicleKind, team: Team, at: Vec3, yaw: f32) -> Vehicle {
        let spec = kind.spec();
        Vehicle {
            id,
            kind,
            team,
            motion: Motion::at(at, yaw),
            health: 100.0,
            seats: vec![None; spec.seats.len()],
            guns: spec
                .guns
                .iter()
                .map(|g| VGun {
                    ammo: g.ammo_max,
                    cooldown: 0.0,
                    yaw: 0.0,
                    pitch: 0.0,
                    barrel: 0,
                })
                .collect(),
            deployed: false,
            empty_for: 0.0,
            from_base: None,
            owner: None,
            history: VecDeque::new(),
            firing: false,
            cargo: 0.0,
            harvesting: false,
            unloading: false,
            flares: 0.0,
            flare_wait: 0.0,
            loadout: kind.default_loadout(),
        }
    }

    /// Ordered with loadout `l`: its main gun's choice loaded.
    pub fn load_out(&mut self, l: vehicle::VehicleLoadout) {
        self.loadout = l;
        if let Some(g) = self.guns.first_mut() {
            g.ammo = self.kind.gun_spec(l, 0).ammo_max;
        }
    }

    /// Gun `g` as loaded out.
    pub fn gun_spec(&self, g: usize) -> vehicle::GunSpec {
        self.kind.gun_spec(self.loadout, g)
    }

    pub fn flying(&self) -> bool {
        self.kind.aircraft() && !self.motion.grounded
    }

    /// Where lock-ons and missiles aim: the hull's middle.
    pub fn center(&self) -> Vec3 {
        self.motion.to_world(Vec3::Y * self.kind.spec().half.y)
    }

    pub fn driver(&self) -> Option<PlayerId> {
        self.seats.first().copied().flatten()
    }

    /// Where seat `seat`'s occupant's eyes are.
    pub fn seat_eye(&self, seat: usize) -> Vec3 {
        let spec = self.kind.spec();
        self.motion.to_world(spec.seats[seat].eye)
    }

    fn at_tick(&self, tick: u64) -> Motion {
        for &(t, p, r) in self.history.iter().rev() {
            if t <= tick {
                return Motion {
                    pos: p,
                    rot: r,
                    ..self.motion
                };
            }
        }
        self.motion
    }

    /// A rough axis-aligned box around the hull (bodies bump into it, stand on it).
    pub fn bounds(&self) -> Collider {
        let half = self.kind.spec().half;
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for x in [-half.x, half.x] {
            for y in [0.0, half.y * 2.0] {
                for z in [-half.z, half.z] {
                    let p = self.motion.to_world(Vec3::new(x, y, z));
                    min = min.min(p);
                    max = max.max(p);
                }
            }
        }
        // (A little above the ground, so bodies walk under nothing and the hull's tilt doesn't
        // make a step of its footprint's corners.)
        min.y += 0.4;
        Collider::Box { min, max }
    }

    fn snap(&self, cloaked: bool) -> VehicleSnap {
        let mut f = 0;
        if self.deployed {
            f |= vflags::DEPLOYED;
        }
        if cloaked {
            f |= vflags::CLOAKED;
        }
        if self.firing {
            f |= vflags::FIRING;
        }
        if self.health < 25.0 {
            f |= vflags::BURNING;
        }
        if self.harvesting {
            f |= vflags::HARVESTING;
        }
        if self.unloading {
            f |= vflags::UNLOADING;
        }
        if self.flares > 0.0 {
            f |= vflags::FLARES;
        }
        VehicleSnap {
            id: self.id,
            kind: self.kind,
            team: self.team,
            pos: self.motion.pos,
            rot: self.motion.rot,
            vel: self.motion.vel,
            aim: self
                .guns
                .iter()
                .map(|g| {
                    (
                        (wrap_angle(g.yaw) * 10000.0) as i16,
                        (g.pitch * 10000.0) as i16,
                    )
                })
                .collect(),
            health: self.health,
            seats: self.seats.clone(),
            flags: f,
            cargo: self.cargo.round() as u8,
            loadout: self.loadout,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Turret {
    pub base: BaseIdx,
    pub index: u8,
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub health: f32,
    pub cooldown: f32,
    pub down: f32,
    pub target: Option<Target>,
    pub think: f32,
    pub occupant: Option<PlayerId>,
    /// Which barrel fires next.
    pub barrel: bool,
    /// Hacked: whose it is now, and for how long more (s).
    pub hacked: Option<(Team, f32)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Player(PlayerId),
    Vehicle(VehicleId),
}

#[derive(Clone, Copy, Debug)]
pub struct Projectile {
    pub id: u32,
    pub ammo: Ammo,
    pub owner: PlayerId,
    pub team: Team,
    pub pos: Vec3,
    pub vel: Vec3,
    pub age: f32,
    pub dist: f32,
    pub lag: u64,
    /// The vehicle it was fired from (not hit by it).
    pub from_vehicle: Option<VehicleId>,
    /// A missile's lock: it homes on this vehicle (until flares lead it astray).
    pub homing: Option<VehicleId>,
}

#[derive(Clone, Debug)]
pub struct BaseRt {
    pub owner: Option<Team>,
    pub hack: Option<Hack>,
    /// Which factions may hack it now (told by the server).
    pub hackable: [bool; 3],
    pub puzzle_seed: u32,
    pub puzzle: Option<Arc<Puzzle>>,
    pub at: Vec<u8>,
    /// The generator's health (0: down, the base unpowered).
    pub generator: f32,
    /// The silo's resources (this continent's word on them; told to the server).
    pub resources: f32,
    resources_known: bool,
    /// Its terminals hacked (by index): whose they are now, and for how long more (s).
    pub hacked: Vec<Option<(Team, f32)>>,
    /// Its capture points (A, B, C; none where the console is hacked).
    pub points: Vec<PointRt>,
    /// Who held two points the last time the server was told (`None`: never told).
    reported: Option<Option<Team>>,
}

impl BaseRt {
    pub fn powered(&self) -> bool {
        self.generator > 0.0
    }
}

/// A resource node: what's left to mine.
#[derive(Clone, Copy, Debug)]
pub struct ResourceNode {
    pub id: u32,
    pub pos: Vec3,
    pub amount: f32,
}

/// Something that went into a warp gate's core.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Warper {
    Player(PlayerId),
    Vehicle(VehicleId),
}

pub struct ContinentSim {
    pub id: ContinentId,
    pub map: Arc<ContinentMap>,
    pub world: World,
    pub tick: u64,
    pub players: Vec<Player>,
    pub vehicles: Vec<Vehicle>,
    pub turrets: Vec<Turret>,
    pub projectiles: Vec<Projectile>,
    pub bases: Vec<BaseRt>,
    /// The owner of the continent each warp gate leads to.
    pub gate_owners: Vec<Option<Team>>,
    /// Whether each faction has something to take beyond each gate (bots head there).
    pub gate_fronts: Vec<[bool; 3]>,
    /// Effects of the last few ticks (tick, effect).
    pub fx: VecDeque<(u64, Fx)>,
    pub out: Vec<Outgoing>,
    /// Consoles solved since the server last took them: base, faction, player.
    pub solved: Vec<(BaseIdx, Team, PlayerId)>,
    /// Capture point bases whose majority changed since the server last took them: the
    /// faction (not the owner) holding two points that may take it, if any.
    pub majority: Vec<(BaseIdx, Option<Team>)>,
    /// Gate cores entered since the server last took them: who and which gate.
    pub warps: Vec<(Warper, u8)>,
    /// Bases whose silo ran dry since the server last took them.
    pub drained: Vec<BaseIdx>,
    /// XP given since the server last took it: to whom, what for, how much.
    pub awards: Vec<(PlayerId, Award, u32)>,
    /// Human players' stats since the server last took them: whose, which counter, how much.
    pub stats: Vec<(PlayerId, String, i64)>,
    pub nodes: Vec<ResourceNode>,
    next_node: u32,
    /// Seconds until each mined out node grows back, and whether the first ones are out.
    regrow: Vec<f32>,
    nodes_seeded: bool,
    powered: Vec<bool>,
    next_projectile: u32,
    next_vehicle: VehicleId,
    /// Engineers' deployables.
    pub deployables: Vec<Deployable>,
    next_deployable: u32,
    /// Built structures.
    pub structures: Vec<Structure>,
    next_structure: u32,
    /// Enemies spotted, by faction.
    pub spots: Vec<spot::Spot>,
    /// Smoke clouds: where, and how long more they hang (s).
    pub smokes: Vec<(Vec3, f32)>,
    /// What vehicles, MAXes and structures cost each faction here (its continent locks'
    /// discount; told by the server).
    pub cost_factor: [f32; 3],
    /// Testing (`PS_DEV_TARGET`): an enemy aircraft held hovering where it is (flares popped
    /// yet), and the aircraft ordered, held at its height.
    dev_target: Option<(VehicleId, Vec3, bool, VehicleId, f32)>,
    rng: Seq,
}

impl ContinentSim {
    pub fn new(id: ContinentId, map: Arc<ContinentMap>) -> ContinentSim {
        let mut turrets = Vec::new();
        for (bi, b) in map.bases.iter().enumerate() {
            for (ti, &t) in b.turrets.iter().enumerate() {
                turrets.push(Turret {
                    base: bi as BaseIdx,
                    index: ti as u8,
                    pos: t,
                    yaw: 0.0,
                    pitch: 0.0,
                    health: 100.0,
                    cooldown: 0.0,
                    down: 0.0,
                    target: None,
                    think: (ti as f32) * 0.07,
                    occupant: None,
                    barrel: false,
                    hacked: None,
                });
            }
        }
        let bases = map
            .bases
            .iter()
            .enumerate()
            .map(|(i, l)| BaseRt {
                owner: None,
                hack: None,
                hackable: [false; 3],
                puzzle_seed: noise::hash2(id as i32, i as i32, map.def.terrain_seed),
                puzzle: None,
                at: Vec::new(),
                generator: 100.0,
                resources: crate::state::START_RESOURCES,
                resources_known: false,
                hacked: vec![None; l.terminals.len()],
                points: vec![PointRt::default(); l.points.len()],
                reported: None,
            })
            .collect();
        let world = map.world.clone();
        let mut s = ContinentSim {
            id,
            world,
            tick: 0,
            players: Vec::new(),
            vehicles: Vec::new(),
            turrets,
            projectiles: Vec::new(),
            bases,
            gate_owners: vec![None; map.gates.len()],
            gate_fronts: vec![[false; 3]; map.gates.len()],
            fx: VecDeque::new(),
            out: Vec::new(),
            solved: Vec::new(),
            majority: Vec::new(),
            warps: Vec::new(),
            drained: Vec::new(),
            awards: Vec::new(),
            stats: Vec::new(),
            nodes: Vec::new(),
            next_node: 1,
            regrow: Vec::new(),
            nodes_seeded: false,
            powered: Vec::new(),
            next_projectile: 1,
            next_vehicle: 1,
            dev_target: None,
            deployables: Vec::new(),
            next_deployable: 1,
            structures: Vec::new(),
            next_structure: 1,
            spots: Vec::new(),
            smokes: Vec::new(),
            cost_factor: [1.0; 3],
            rng: Seq::new(map.def.terrain_seed as u64, 0x51),
            map,
        };
        for b in 0..s.bases.len() {
            s.new_puzzle(b);
        }
        s.update_shields();
        s
    }

    fn new_puzzle(&mut self, b: usize) {
        let rt = &mut self.bases[b];
        rt.puzzle_seed = noise::hash2(rt.puzzle_seed as i32, 77, 0x9e37);
        let p = Puzzle::generate(rt.puzzle_seed);
        rt.at = p.start();
        rt.puzzle = Some(Arc::new(p));
    }

    /// The server's word on a base: its owner, hack and who may hack it.
    pub fn set_base(
        &mut self,
        b: BaseIdx,
        owner: Option<Team>,
        hack: Option<Hack>,
        hackable: [bool; 3],
    ) {
        let Some(rt) = self.bases.get_mut(b as usize) else {
            return;
        };
        let changed = rt.owner != owner;
        rt.owner = owner;
        rt.hack = hack;
        rt.hackable = hackable;
        if changed {
            for h in rt.hacked.iter_mut() {
                *h = None;
            }
            for pt in rt.points.iter_mut() {
                *pt = PointRt::held_by(owner);
            }
            self.update_shields();
            for t in self.turrets.iter_mut().filter(|t| t.base == b) {
                t.health = 100.0;
                t.down = 0.0;
                t.target = None;
                t.hacked = None;
            }
        }
    }

    /// The server's word on a base's silo: taken the first time (this continent keeps count
    /// after that).
    pub fn set_resources(&mut self, b: BaseIdx, r: f32) {
        if let Some(rt) = self.bases.get_mut(b as usize)
            && !rt.resources_known
        {
            rt.resources = r;
            rt.resources_known = true;
        }
    }

    /// The spawn rooms' forcefields let their base's owners through (while it has power).
    fn update_shields(&mut self) {
        self.world.shields.clear();
        self.powered = self.bases.iter().map(|b| b.powered()).collect();
        for (i, b) in self.map.bases.iter().enumerate() {
            if !self.bases[i].powered() {
                continue;
            }
            let owner = self.bases[i].owner;
            for s in &b.shields {
                self.world.shields.push((s.collider, owner));
            }
        }
    }

    pub fn player(&self, id: PlayerId) -> Option<&Player> {
        self.players.iter().find(|p| p.id == id)
    }

    pub fn player_mut(&mut self, id: PlayerId) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.id == id)
    }

    fn pidx(&self, id: PlayerId) -> Option<usize> {
        self.players.iter().position(|p| p.id == id)
    }

    pub fn vehicle(&self, id: VehicleId) -> Option<&Vehicle> {
        self.vehicles.iter().find(|v| v.id == id)
    }

    fn vidx(&self, id: VehicleId) -> Option<usize> {
        self.vehicles.iter().position(|v| v.id == id)
    }

    /// A new player (dead, choosing where to spawn).
    pub fn add_player(&mut self, id: PlayerId, name: &str, team: Team, bot: bool) {
        self.players.push(Player::new(id, name, team, bot));
    }

    /// A player arriving from elsewhere (a warp, a spawn on this continent).
    pub fn insert_player(&mut self, p: Player) {
        self.players.retain(|q| q.id != p.id);
        self.players.push(p);
    }

    /// Takes a player off this continent (leaving, warping), out of any seat.
    pub fn remove_player(&mut self, id: PlayerId) -> Option<Player> {
        let i = self.pidx(id)?;
        self.unseat(i);
        if let Some(t) = self.players[i].turret.take()
            && let Some(tr) = self.turrets.get_mut(t)
        {
            tr.occupant = None;
        }
        self.drop_deployables(id);
        Some(self.players.remove(i))
    }

    /// Takes a vehicle off this continent with its occupants (warping).
    pub fn remove_vehicle(&mut self, id: VehicleId) -> Option<(Vehicle, Vec<Player>)> {
        let i = self.vidx(id)?;
        let v = self.vehicles.remove(i);
        let mut riders = Vec::new();
        for pid in v.seats.iter().flatten() {
            if let Some(pi) = self.pidx(*pid) {
                riders.push(self.players.remove(pi));
            }
        }
        Some((v, riders))
    }

    /// A vehicle arriving with its occupants (a warp): placed at `at` facing `yaw`.
    pub fn insert_vehicle(&mut self, mut v: Vehicle, riders: Vec<Player>, at: Vec3, yaw: f32) {
        let id = self.next_vehicle;
        self.next_vehicle = self.next_vehicle.wrapping_add(1).max(1);
        v.id = id;
        let fwd = look_dir(yaw, 0.0);
        let speed = v.motion.vel.length().min(10.0);
        v.motion = Motion {
            vel: fwd * speed,
            ..Motion::at(at, yaw)
        };
        if v.kind.aircraft() {
            v.motion.grounded = false;
            v.motion.pos.y += 15.0;
            v.motion.throttle = speed;
        }
        v.history.clear();
        for mut p in riders {
            if let Some((_, seat)) = p.seat {
                p.seat = Some((id, seat));
            }
            self.insert_player(p);
        }
        self.vehicles.push(v);
    }

    pub fn set_cmd(&mut self, id: PlayerId, cmd: PlayerCmd) {
        if let Some(p) = self.player_mut(id) {
            p.cmd = cmd;
        }
    }

    fn say(&mut self, to: Audience, ev: SimEvent) {
        self.out.push(Outgoing { to, ev });
    }

    fn deny(&mut self, id: PlayerId, d: Deny) {
        self.say(Audience::Player(id), SimEvent::Denied(d));
    }

    fn fx(&mut self, f: Fx) {
        self.fx.push_back((self.tick, f));
    }

    /// Gives player `id` XP (`progress`): its score, the server's tally, a word to it.
    /// Adds `n` to human player `id`'s stats counter `key` (bots keep none).
    pub fn stat(&mut self, id: PlayerId, key: &str, n: i64) {
        if self.player(id).is_some_and(|p| !p.bot) {
            self.stats.push((id, key.to_string(), n));
        }
    }

    pub fn award(&mut self, id: PlayerId, reason: Award, xp: u32) {
        if xp == 0 || id == crate::types::HAZARD {
            return;
        }
        let Some(pi) = self.pidx(id) else { return };
        let p = &mut self.players[pi];
        p.score = p.score.saturating_add(xp);
        self.awards.push((id, reason, xp));
        self.say(Audience::Player(id), SimEvent::Xp { reason, xp });
    }

    /// Adds `amount` (scaled by `per`) to one of player `pi`'s lumps of support XP, giving it
    /// when it reaches `progress::LUMP`.
    fn support_xp(&mut self, pi: usize, reason: Award, amount: f32, per: f32) {
        let p = &mut self.players[pi];
        let lump = match reason {
            Award::Heal => &mut p.heal_xp,
            Award::Repair => &mut p.repair_xp,
            _ => &mut p.deliver_xp,
        };
        *lump += amount.max(0.0) * per;
        if *lump >= progress::LUMP {
            let xp = *lump as u32;
            *lump -= xp as f32;
            let id = p.id;
            self.award(id, reason, xp);
        }
    }

    /// Puts a (dead) player at `feet` facing `yaw`, alive and armed.
    pub fn spawn_at(&mut self, id: PlayerId, feet: Vec3, yaw: f32) {
        let Some(p) = self.player_mut(id) else {
            return;
        };
        p.body = Body::at(feet);
        p.cmd.yaw = yaw;
        p.cmd.pitch = 0.0;
        p.cmd.feet = None;
        p.health = 100.0;
        p.refill();
        p.dead = None;
        p.downed = false;
        p.revive = 0.0;
        p.hurt_by.clear();
        p.protect = SPAWN_PROTECT;
        p.seat = None;
        p.history.clear();
        self.say(
            Audience::Player(id),
            SimEvent::Spawned {
                player: id,
                pos: feet,
                yaw,
            },
        );
    }

    /// A free spawn point at base `b` (in its spawn room).
    pub fn base_spawn(&mut self, b: usize) -> Option<(Vec3, f32)> {
        let layout = self.map.bases.get(b)?;
        let n = layout.spawns.len();
        if n == 0 {
            return Some((layout.center, 0.0));
        }
        let start = self.rng.int(0, n as i32) as usize;
        for k in 0..n {
            let (p, yaw) = layout.spawns[(start + k) % n];
            if !self
                .players
                .iter()
                .any(|q| q.afoot() && (q.feet() - p).length() < 0.8)
            {
                return Some((p, yaw));
            }
        }
        Some(layout.spawns[start])
    }

    /// Whether `id` may spawn now (dead long enough).
    pub fn may_spawn(&self, id: PlayerId) -> bool {
        self.player(id)
            .is_some_and(|p| p.dead.is_some_and(|t| t >= self.respawn_time(p.team)))
    }

    /// A request from a player; spawns at bases on other continents are the server's.
    pub fn request(&mut self, id: PlayerId, r: Request) {
        let Some(pi) = self.pidx(id) else { return };
        let team = self.players[pi].team;
        match r {
            Request::Spawn(SpawnPoint::Base { continent, base }) => {
                if continent != self.id {
                    return;
                }
                if !self.may_spawn(id) {
                    return self.deny(id, Deny::NotReady);
                }
                if self.bases.get(base as usize).and_then(|b| b.owner) != Some(team) {
                    return self.deny(id, Deny::NotYours);
                }
                if !self.can_spawn_at(base as usize) {
                    return self.deny(id, Deny::NoResources);
                }
                if let Some((p, yaw)) = self.base_spawn(base as usize) {
                    self.spawn_at(id, p, yaw);
                }
            }
            Request::Spawn(SpawnPoint::Bus(v)) => {
                if !self.may_spawn(id) {
                    return self.deny(id, Deny::NotReady);
                }
                let Some(bus) = self.vehicle(v) else {
                    return self.deny(id, Deny::NotReady);
                };
                if bus.team != team || !bus.deployed {
                    return self.deny(id, Deny::NotYours);
                }
                let m = bus.motion;
                let owner = bus.owner.filter(|&o| o != id);
                let a = self.rng.range(0.0, std::f32::consts::TAU);
                let r = self.rng.range(4.5, 7.0);
                let mut at = m.pos + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
                at.y = self.world.ground(at.x, at.z, at.y + 3.0).unwrap_or(at.y);
                let yaw = yaw_of(at - m.pos);
                self.spawn_at(id, at, yaw);
                if let Some(o) = owner {
                    self.award(o, Award::Spawn, progress::SPAWN_XP);
                }
            }
            Request::Spawn(SpawnPoint::Squad(leader)) => {
                if !self.may_spawn(id) {
                    return self.deny(id, Deny::NotReady);
                }
                match self.squad_spawn(pi, leader) {
                    Ok(()) => self.award(leader, Award::Spawn, progress::SPAWN_XP),
                    Err(d) => self.deny(id, d),
                }
            }
            Request::Loadout(l) => {
                if !l.valid(team) {
                    return;
                }
                // (Testing: PS_CHEAT=1 takes one anywhere, free.)
                let term = self.terminal_near(pi, TerminalKind::Equipment);
                if term.is_none() && !(cheats() && self.players[pi].afoot()) {
                    return self.deny(id, Deny::TooFar);
                }
                // A MAX costs the base resources (not at a sanctuary), each time one is taken.
                if let Some((b, _)) = term
                    && l.max()
                    && self.map.bases[b].kind != BaseKind::Home
                {
                    let cost = weapons::MAX_COST * self.cost_factor[team.index()];
                    if self.bases[b].resources < cost {
                        return self.deny(id, Deny::NoResources);
                    }
                    self.drain(b, cost);
                }
                let p = &mut self.players[pi];
                p.loadout = l;
                p.refill();
                self.say(Audience::Player(id), SimEvent::LoadoutTaken { player: id });
            }
            Request::Heal => {
                if self.terminal_near(pi, TerminalKind::Medical).is_none() {
                    return self.deny(id, Deny::TooFar);
                }
                let p = &mut self.players[pi];
                p.health = 100.0;
                self.say(Audience::Player(id), SimEvent::Healed { player: id });
            }
            Request::EnterVehicle(v) => self.enter(pi, v),
            Request::EnterTurret(t) => self.enter_turret(pi, t as usize),
            Request::ExitVehicle => {
                if self.players[pi].turret.is_some() {
                    self.leave_turret(pi);
                } else {
                    self.exit(pi);
                }
            }
            Request::SpawnVehicle {
                base,
                terminal,
                kind,
                loadout,
            } => {
                let loadout = if kind.loadout_valid(loadout) {
                    loadout
                } else {
                    kind.default_loadout()
                };
                self.spawn_vehicle(pi, base as usize, terminal as usize, kind, loadout)
            }
            Request::ViewConsole(b) => {
                if let Some(rt) = self.bases.get(b as usize) {
                    let ev = SimEvent::Puzzle {
                        base: b,
                        seed: rt.puzzle_seed,
                        at: rt.at.clone(),
                    };
                    self.say(Audience::Player(id), ev);
                }
            }
            Request::Poke { base, piece } => self.poke(pi, base as usize, piece as usize),
            Request::Suicide => {
                if self.players[pi].alive() {
                    self.kill(pi, id, Cause::Suicide, false);
                }
            }
            Request::Hack(what) => self.start_hack(pi, what),
            Request::Spot => self.spot(pi),
            Request::Build { kind, at, yaw } => self.build(pi, kind, at, yaw),
            Request::Pull { harvester, kind } => self.pull(pi, harvester, kind),
            Request::Spawn(SpawnPoint::Tube(t)) => {
                if !self.may_spawn(id) {
                    return self.deny(id, Deny::NotReady);
                }
                if let Err(d) = self.tube_spawn(pi, t) {
                    self.deny(id, d);
                }
            }
        }
    }

    /// Spawns (dead) player `pi` on its squad leader `leader` (the server has checked the
    /// squad): in a free seat of the leader's vehicle, beside it (not in the air), or next to
    /// the leader on foot where nothing stands between them.
    fn squad_spawn(&mut self, pi: usize, leader: PlayerId) -> Result<(), Deny> {
        let (id, team) = (self.players[pi].id, self.players[pi].team);
        let Some(li) = self.pidx(leader).filter(|&li| li != pi) else {
            return Err(Deny::LeaderUnavailable);
        };
        let l = &self.players[li];
        if l.team != team || !l.alive() || l.turret.is_some() {
            return Err(Deny::LeaderUnavailable);
        }
        let (feet, yaw) = (l.feet(), l.cmd.yaw);
        if let Some((vid, _)) = l.seat {
            let vi = self.vidx(vid).ok_or(Deny::LeaderUnavailable)?;
            let v = &self.vehicles[vi];
            let (pos, flying) = (v.motion.pos, v.flying());
            match v.seats.iter().position(|s| s.is_none()) {
                Some(seat) => {
                    self.spawn_at(id, pos, yaw);
                    self.seat(pi, vi, seat);
                }
                None if flying => return Err(Deny::LeaderUnavailable),
                None => {
                    let half = v.kind.spec().half;
                    let r = half.x.max(half.z) + 2.5;
                    let a = self.rng.range(0.0, std::f32::consts::TAU);
                    let mut at = pos + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
                    at.y = self.world.ground(at.x, at.z, at.y + 3.0).unwrap_or(at.y);
                    self.spawn_at(id, at, yaw_of(at - pos));
                }
            }
            return Ok(());
        }
        // On foot: a free spot round the leader on about its level, in its sight.
        let start = self.rng.int(0, 8);
        let chest = Vec3::Y * 1.0;
        let mut spot = feet;
        for k in 0..8 {
            let a = (start + k) as f32 / 8.0 * std::f32::consts::TAU;
            let mut at = feet + Vec3::new(a.cos(), 0.0, a.sin()) * 2.0;
            let Some(g) = self.world.ground(at.x, at.z, feet.y + 1.2) else {
                continue;
            };
            if (g - feet.y).abs() > 0.8 {
                continue;
            }
            at.y = g;
            let crowded = self
                .players
                .iter()
                .any(|q| q.afoot() && (q.feet() - at).length() < 0.8);
            if !crowded && self.world.clear(feet + chest, at + chest) {
                spot = at;
                break;
            }
        }
        self.spawn_at(id, spot, yaw);
        Ok(())
    }

    /// Brings downed player `pi` back where it fell (revived by `by`): a little health, no
    /// armor, its guns as they were.
    fn revive_player(&mut self, pi: usize, by: PlayerId) {
        let p = &mut self.players[pi];
        if !p.revivable() {
            return;
        }
        self.stat(by, "revives", 1);
        let p = &mut self.players[pi];
        let feet = p.body.feet;
        let yaw = p.cmd.yaw;
        p.body = Body::at(feet);
        p.cmd.feet = None;
        p.health = REVIVE_HEALTH;
        p.armor = 0.0;
        p.dead = None;
        p.downed = false;
        p.revive = 0.0;
        p.hurt_by.clear();
        p.protect = 1.0;
        p.history.clear();
        let id = p.id;
        self.say(
            Audience::Player(id),
            SimEvent::Spawned {
                player: id,
                pos: feet,
                yaw,
            },
        );
        self.say(Audience::All, SimEvent::Revived { player: id, by });
        self.award(by, Award::Revive, progress::REVIVE_XP);
    }

    /// A dead bot waits rather than spawning while a medic of its faction is reviving it or
    /// is near enough to (for the first half of the window).
    pub fn awaiting_revive(&self, id: PlayerId) -> bool {
        let Some(p) = self.player(id).filter(|p| p.revivable()) else {
            return false;
        };
        if p.revive > 0.0 {
            return true;
        }
        p.dead.is_some_and(|t| t < REVIVE_WINDOW * 0.5)
            && self.players.iter().any(|m| {
                m.team == p.team
                    && m.afoot()
                    && m.loadout.special == Weapon::Medgun
                    && (m.feet() - p.feet()).length() < REVIVE_WAIT_RANGE
            })
    }

    /// Whether player `pi` stands at a terminal of `kind` of a base its faction holds (any
    /// terminal in a sanctuary).
    fn poke(&mut self, pi: usize, b: usize, piece: usize) {
        let (id, team, feet) = {
            let p = &self.players[pi];
            (p.id, p.team, p.feet())
        };
        let Some((at, _)) = self.map.bases.get(b).and_then(|l| l.console) else {
            return;
        };
        if !self.players[pi].afoot() || (at - feet).length() > USE_REACH + 0.6 {
            return self.deny(id, Deny::TooFar);
        }
        if self.players[pi].max_suit() {
            return self.deny(id, Deny::Max);
        }
        let rt = &self.bases[b];
        let ours = rt.owner == Some(team);
        if ours && rt.hack.is_none() {
            return self.deny(id, Deny::NoHack);
        }
        if !ours && !rt.hackable[team.index()] && rt.hack.is_none_or(|h| h.team != team) {
            return self.deny(id, Deny::NotLinked);
        }
        if !ours && rt.hack.is_some_and(|h| h.team == team) {
            return self.deny(id, Deny::NotReady);
        }
        self.poke_piece(b, piece, id, team);
    }

    /// Steps the capture points: who stands in each, who takes them (XP to those there), and
    /// who holds two (told to the server when it changes).
    fn step_points(&mut self, dt: f32) {
        for b in 0..self.bases.len() {
            if self.bases[b].points.is_empty() {
                continue;
            }
            let (owner, hackable) = (self.bases[b].owner, self.bases[b].hackable);
            let eligible = Team::ALL.map(|t| owner == Some(t) || hackable[t.index()]);
            for i in 0..self.bases[b].points.len() {
                let pt = &self.map.bases[b].points[i];
                let (at, r2) = (pt.pos, pt.radius * pt.radius);
                let mut present = [0u8; 3];
                let mut who = Vec::new();
                for p in &self.players {
                    let f = p.body.feet;
                    if p.afoot()
                        && (f.y - at.y).abs() < crate::capture::POINT_DY
                        && Vec2::new(f.x - at.x, f.z - at.z).length_squared() < r2
                    {
                        let k = &mut present[p.team.index()];
                        *k = k.saturating_add(1);
                        who.push((p.id, p.team));
                    }
                }
                let Some(took) =
                    crate::capture::step_point(&mut self.bases[b].points[i], present, eligible, dt)
                else {
                    continue;
                };
                let defend = Some(took) == owner && self.bases[b].hack.is_some();
                for (id, t) in who {
                    if t != took {
                        continue;
                    }
                    if defend {
                        self.award(id, Award::Defend, progress::CAPTURE_XP);
                        self.stat(id, "defends", 1);
                    } else {
                        self.award(id, Award::Capture, progress::CAPTURE_XP);
                        self.stat(id, "points", 1);
                    }
                }
            }
            let rt = &mut self.bases[b];
            let want = crate::capture::majority(&rt.points)
                .filter(|&t| Some(t) != rt.owner && rt.hackable[t.index()]);
            if rt.reported != Some(want) {
                rt.reported = Some(want);
                self.majority.push((b as BaseIdx, want));
            }
        }
    }

    /// Pokes a console's piece (players through `poke`, bots straight).
    pub fn poke_piece(&mut self, b: usize, piece: usize, by: PlayerId, team: Team) {
        let rt = &mut self.bases[b];
        let Some(puzzle) = rt.puzzle.clone() else {
            return;
        };
        puzzle.poke(&mut rt.at, piece);
        if puzzle.solved(&rt.at) {
            // (Only what changes something: a hack started, or one stopped.)
            let (defend, hack) = (rt.owner == Some(team), rt.hack.is_some());
            let counts = if defend {
                hack
            } else {
                rt.hackable[team.index()] && !hack
            };
            self.solved.push((b as BaseIdx, team, by));
            if counts {
                let why = if defend { Award::Defend } else { Award::Hack };
                self.award(by, why, progress::HACK_XP);
                self.stat(by, if defend { "defends" } else { "hacks" }, 1);
            }
            self.say(
                Audience::Near(self.map.bases[b].center, 200.0),
                SimEvent::ConsoleSolved {
                    base: b as BaseIdx,
                    by,
                    team,
                },
            );
            self.new_puzzle(b);
        }
        let rt = &self.bases[b];
        let ev = SimEvent::Puzzle {
            base: b as BaseIdx,
            seed: rt.puzzle_seed,
            at: rt.at.clone(),
        };
        let console = self.map.bases[b].console.map_or(Vec3::ZERO, |c| c.0);
        self.say(Audience::Near(console, 12.0), ev);
    }

    fn enter(&mut self, pi: usize, v: VehicleId) {
        let p = &self.players[pi];
        let id = p.id;
        if !p.afoot() {
            return;
        }
        let Some(vi) = self.vidx(v) else { return };
        let veh = &self.vehicles[vi];
        let spec = veh.kind.spec();
        if veh.team != p.team {
            return self.deny(id, Deny::NotYours);
        }
        if vehicle::hull_distance(&veh.motion, spec.half, p.eye()) > 3.5 {
            return self.deny(id, Deny::TooFar);
        }
        // (A MAX rides only where there's neither the controls nor a gun.)
        let max = self.players[pi].max_suit();
        let Some(seat) = (0..veh.seats.len()).find(|&s| {
            veh.seats[s].is_none()
                && (!max || (!spec.seats[s].driver && spec.seats[s].gun.is_none()))
        }) else {
            return self.deny(id, if max { Deny::Max } else { Deny::Full });
        };
        self.seat(pi, vi, seat);
    }

    fn seat(&mut self, pi: usize, vi: usize, seat: usize) {
        let id = self.players[pi].id;
        let vid = self.vehicles[vi].id;
        self.vehicles[vi].seats[seat] = Some(id);
        self.vehicles[vi].empty_for = 0.0;
        let p = &mut self.players[pi];
        p.seat = Some((vid, seat as u8));
        p.body.vel = Vec2::ZERO;
        p.body.vy = 0.0;
        p.cmd.feet = None;
        let eye = self.vehicles[vi].seat_eye(seat);
        self.players[pi].body.feet = eye - Vec3::Y * player::EYE;
    }

    /// Takes player `pi` out of its seat (nowhere in particular).
    fn unseat(&mut self, pi: usize) {
        if let Some((v, s)) = self.players[pi].seat.take()
            && let Some(vi) = self.vidx(v)
        {
            self.vehicles[vi].seats[s as usize] = None;
        }
    }

    fn exit(&mut self, pi: usize) {
        let Some((v, _)) = self.players[pi].seat else {
            return;
        };
        let Some(vi) = self.vidx(v) else {
            self.unseat(pi);
            return;
        };
        let m = self.vehicles[vi].motion;
        let half = self.vehicles[vi].kind.spec().half;
        if self.vehicles[vi].flying() && m.vel.length() > 3.0 {
            // (Bailing out of a flying VTOL: fine, a long way down.)
        }
        let right = m.rot * Vec3::X;
        let fwd = m.rot * Vec3::NEG_Z;
        let tries = [
            -right * (half.x + 1.2),
            right * (half.x + 1.2),
            fwd * -(half.z + 1.5),
            fwd * (half.z + 1.5),
        ];
        let mut at = m.pos + tries[0];
        for t in tries {
            let mut q = m.pos + t;
            let ground = self.world.ground(q.x, q.z, q.y + 2.5).unwrap_or(q.y);
            q.y = q.y.max(ground);
            let mut probe = q;
            let blocked = self
                .world
                .near_colliders(
                    Vec2::new(q.x - 1.0, q.z - 1.0),
                    Vec2::new(q.x + 1.0, q.z + 1.0),
                )
                .iter()
                .any(|c| crate::collide::push_out(c, &mut probe));
            if !blocked {
                at = q;
                break;
            }
        }
        self.unseat(pi);
        let flying = self.vehicles[vi].flying();
        let p = &mut self.players[pi];
        p.body = Body::at(at);
        p.body.vy = if flying { -0.1 } else { 0.0 };
        // Bailing out of an aircraft in the air: under a chute.
        p.body.chute = flying && at.y - self.world.ground(at.x, at.z, at.y).unwrap_or(at.y) > 4.0;
        p.body.vel = Vec2::new(m.vel.x, m.vel.z) * 0.5;
        p.cmd.feet = None;
        let (id, yaw) = (p.id, p.cmd.yaw);
        self.say(
            Audience::Player(id),
            SimEvent::Spawned {
                player: id,
                pos: at,
                yaw,
            },
        );
    }

    fn spawn_vehicle(
        &mut self,
        pi: usize,
        b: usize,
        t: usize,
        kind: VehicleKind,
        loadout: vehicle::VehicleLoadout,
    ) {
        let (id, team, feet, cooldown) = {
            let p = &self.players[pi];
            (p.id, p.team, p.feet(), p.vehicle_cooldown)
        };
        let Some(layout) = self.map.bases.get(b) else {
            return;
        };
        let Some(term) = layout.terminals.get(t).copied() else {
            return;
        };
        if term.kind != TerminalKind::Vehicle {
            return;
        }
        if !self.players[pi].afoot() || ((term.pos - feet).length() > USE_REACH + 1.0 && !cheats())
        {
            return self.deny(id, Deny::TooFar);
        }
        if self.players[pi].max_suit() {
            return self.deny(id, Deny::Max);
        }
        if self.terminal_team(b, t) != Some(team) {
            return self.deny(id, Deny::NotYours);
        }
        if kind.advanced() && !self.benefit(team, Facility::TechPlant) {
            return self.deny(id, Deny::NeedsFacility(Facility::TechPlant));
        }
        if cooldown > 0.0 && !cheats() {
            return self.deny(id, Deny::Cooldown(cooldown.ceil() as u16));
        }
        if !self.bases[b].powered() {
            return self.deny(id, Deny::Unpowered);
        }
        let cost = kind.cost() * self.cost_factor[team.index()];
        if self.bases[b].resources < cost && layout.kind != BaseKind::Home {
            return self.deny(id, Deny::NoResources);
        }
        let (pad, yaw) = layout.pads[term.pad.unwrap_or(0)];
        if self
            .vehicles
            .iter()
            .any(|v| (v.motion.pos - pad).length() < PAD_RADIUS + 2.0)
        {
            return self.deny(id, Deny::PadBusy);
        }
        if self
            .vehicles
            .iter()
            .filter(|v| v.from_base == Some(b as BaseIdx))
            .count()
            >= VEHICLES_PER_BASE
        {
            return self.deny(id, Deny::TooMany);
        }
        let vid = self.add_vehicle(kind, team, pad + Vec3::Y * 0.2, yaw);
        if let Some(vi) = self.vidx(vid) {
            self.vehicles[vi].load_out(loadout);
            self.vehicles[vi].from_base = Some(b as BaseIdx);
            self.vehicles[vi].owner = Some(id);
            self.players[pi].vehicle_cooldown = kind.spec().cooldown;
            self.seat(pi, vi, 0);
        }
        self.drain(b, cost);
        // Testing: PS_DEV_TARGET=1 (with PS_CHEAT=1) puts an enemy VTOL hovering 220 m ahead of
        // an aircraft ordered (lifted 80 m), to lock on to (it pops flares at the first missile that comes for it).
        if cheats() && kind.aircraft() && std::env::var("PS_DEV_TARGET").is_ok_and(|v| v == "1") {
            // (Both up in the open air: over the base's walls.)
            let mut height = pad.y + 80.0;
            if let Some(vi) = self.vidx(vid) {
                let m = &mut self.vehicles[vi].motion;
                m.pos.y += 80.0;
                m.grounded = false;
                height = m.pos.y;
            }
            let ahead = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
            let at = pad + ahead * 220.0 + Vec3::Y * 84.0;
            let enemy = Team::ALL.into_iter().find(|&t| t != team).unwrap_or(team);
            let t = self.add_vehicle(VehicleKind::Vtol, enemy, at, yaw);
            self.dev_target = Some((t, at, false, vid, height));
        }
        self.say(
            Audience::Near(pad, 300.0),
            SimEvent::VehicleSpawned {
                id: vid,
                kind,
                by: id,
            },
        );
    }

    /// A bot takes a vehicle: from base `base`'s first free pad (the base its faction's), or
    /// at `at` (yaw); it sits in the driver's seat. False if none could be had.
    pub fn bot_vehicle(
        &mut self,
        id: PlayerId,
        kind: VehicleKind,
        base: Option<usize>,
        at: Option<(Vec3, f32)>,
    ) -> bool {
        let Some(pi) = self.pidx(id) else {
            return false;
        };
        if !self.players[pi].afoot() || self.players[pi].max_suit() {
            return false;
        }
        let team = self.players[pi].team;
        if kind.advanced() && !self.benefit(team, Facility::TechPlant) {
            return false;
        }
        let place = match (base, at) {
            (Some(b), _) => {
                if self.bases.get(b).and_then(|r| r.owner) != Some(team)
                    || !self.bases[b].powered()
                    || (self.bases[b].resources < kind.cost() * self.cost_factor[team.index()]
                        && self.map.bases[b].kind != BaseKind::Home)
                    || self
                        .vehicles
                        .iter()
                        .filter(|v| v.from_base == Some(b as BaseIdx))
                        .count()
                        >= VEHICLES_PER_BASE
                {
                    return false;
                }
                let pads = &self.map.bases[b].pads;
                pads.iter()
                    .find(|(p, _)| {
                        !self
                            .vehicles
                            .iter()
                            .any(|v| (v.motion.pos - *p).length() < PAD_RADIUS + 2.0)
                    })
                    .copied()
            }
            (None, Some(a)) => Some(a),
            _ => None,
        };
        let Some((p, yaw)) = place else { return false };
        let vid = self.add_vehicle(kind, team, p + Vec3::Y * 0.2, yaw);
        if let Some(b) = base {
            self.drain(b, kind.cost() * self.cost_factor[team.index()]);
        }
        if let Some(vi) = self.vidx(vid) {
            self.vehicles[vi].from_base = base.map(|b| b as BaseIdx);
            self.vehicles[vi].owner = Some(id);
            self.seat(pi, vi, 0);
            return true;
        }
        false
    }

    /// Puts bot `id` (on foot, just spawned) straight into seat `seat` of its faction's
    /// vehicle `v` (crewing it from the base nearby).
    pub fn bot_board(&mut self, id: PlayerId, v: VehicleId, seat: usize) -> bool {
        let (Some(pi), Some(vi)) = (self.pidx(id), self.vidx(v)) else {
            return false;
        };
        let spec = self.vehicles[vi].kind.spec();
        if !self.players[pi].afoot()
            || self.vehicles[vi].team != self.players[pi].team
            || self.vehicles[vi].seats.get(seat) != Some(&None)
            || (self.players[pi].max_suit()
                && (spec.seats[seat].driver || spec.seats[seat].gun.is_some()))
        {
            return false;
        }
        self.seat(pi, vi, seat);
        true
    }

    /// Puts a new vehicle on the continent.
    pub fn add_vehicle(&mut self, kind: VehicleKind, team: Team, at: Vec3, yaw: f32) -> VehicleId {
        let id = self.next_vehicle;
        self.next_vehicle = self.next_vehicle.wrapping_add(1).max(1);
        self.vehicles.push(Vehicle::new(id, kind, team, at, yaw));
        id
    }

    pub fn step(&mut self) {
        self.tick += 1;
        let dt = TICK;
        // (`solved` and `warps` pile up until the server takes them.)
        while self.fx.front().is_some_and(|(t, _)| *t + 12 < self.tick) {
            self.fx.pop_front();
        }
        self.step_nodes(dt);
        // Generators going down or coming back: the forcefields follow.
        let powered: Vec<bool> = self.bases.iter().map(|b| b.powered()).collect();
        if powered != self.powered {
            let was_powered = self.powered.clone();
            for (b, (&now, &was)) in powered.iter().zip(was_powered.iter()).enumerate() {
                if now != was {
                    let at = self.map.bases[b].center;
                    self.say(
                        Audience::Near(at, 600.0),
                        SimEvent::Generator {
                            base: b as BaseIdx,
                            up: now,
                        },
                    );
                }
            }
            self.update_shields();
        }
        self.world.set_dynamic(Vec::new());
        self.step_vehicles(dt);
        if let Some((t, at, flared, mine, height)) = self.dev_target {
            if let Some(vi) = self.vidx(mine) {
                let m = &mut self.vehicles[vi].motion;
                m.pos.y = height;
                m.vel.y = 0.0;
            }
            match self.vidx(t) {
                Some(vi) => {
                    let m = &mut self.vehicles[vi].motion;
                    m.pos = at;
                    m.vel = Vec3::ZERO;
                    m.grounded = false;
                    // (Once: the next missile gets it.)
                    if !flared && self.projectiles.iter().any(|p| p.homing == Some(t)) {
                        self.flare(vi);
                        self.dev_target = Some((t, at, true, mine, height));
                    }
                }
                None => self.dev_target = None,
            }
        }
        let dynamic: Vec<Collider> = self
            .vehicles
            .iter()
            .filter(|v| !v.flying())
            .map(|v| v.bounds())
            .chain(self.structure_colliders())
            .collect();
        self.world.set_dynamic(dynamic);
        self.step_players(dt);
        self.step_points(dt);
        // (Shots and sight lines: the structures stand in the way.)
        let built: Vec<Collider> = self.structure_colliders().collect();
        self.world.set_dynamic(built);
        self.step_turrets(dt);
        self.step_kit(dt);
        self.step_structures(dt);
        self.step_spots();
        for s in &mut self.smokes {
            s.1 -= dt;
        }
        self.smokes.retain(|s| s.1 > 0.0);
        for p in &mut self.players {
            p.flashed = (p.flashed - dt).max(0.0);
        }
        self.step_projectiles(dt);
        self.check_gates();
        // History for lag compensation.
        let tick = self.tick;
        for p in &mut self.players {
            p.history.push_back((tick, p.body.feet, p.cmd.crouch));
            while p.history.len() > HISTORY {
                p.history.pop_front();
            }
        }
        for v in &mut self.vehicles {
            v.history.push_back((tick, v.motion.pos, v.motion.rot));
            while v.history.len() > HISTORY {
                v.history.pop_front();
            }
        }
    }

    fn step_players(&mut self, dt: f32) {
        for pi in 0..self.players.len() {
            {
                let p = &mut self.players[pi];
                p.vehicle_cooldown = (p.vehicle_cooldown - dt).max(0.0);
                if let Some(t) = &mut p.dead {
                    *t += dt;
                    // (A revive under way stops when the beam leaves the body for a moment.)
                    if self.tick > p.revive_tick + 30 {
                        p.revive = 0.0;
                    }
                    continue;
                }
                p.protect = (p.protect - dt).max(0.0);
            }
            self.step_lock(pi, dt);
            if self.players[pi].seat.is_some() {
                self.step_seated(pi);
                continue;
            }
            if let Some(t) = self.players[pi].turret {
                // In the turret's seat, turning with it.
                let tr = &self.turrets[t];
                let eye = tr.pos
                    + Vec3::Y * TURRET_PIVOT
                    + glam::Quat::from_rotation_y(tr.yaw) * TURRET_EYE;
                let p = &mut self.players[pi];
                p.body.feet = eye - Vec3::Y * player::EYE;
                p.firing = tr.cooldown > TURRET_INTERVAL * 0.5;
                continue;
            }
            self.move_player(pi, dt);
            self.weapons(pi, dt);
        }
    }

    fn step_seated(&mut self, pi: usize) {
        let Some((v, s)) = self.players[pi].seat else {
            return;
        };
        let Some(vi) = self.vidx(v) else {
            self.players[pi].seat = None;
            return;
        };
        let eye = self.vehicles[vi].seat_eye(s as usize);
        let p = &mut self.players[pi];
        p.body.feet = eye - Vec3::Y * player::EYE;
        p.firing = false;
        p.beaming = false;
        // Switching seats.
        let cmd = p.cmd;
        if let Some(seen) = p.switches_seen
            && seen != cmd.seat_switches
        {
            let to = cmd.seat as usize;
            // (A MAX only into a seat with neither the controls nor a gun.)
            let spec = self.vehicles[vi].kind.spec();
            let max_ok = !p.max_suit()
                || spec
                    .seats
                    .get(to)
                    .is_some_and(|s| !s.driver && s.gun.is_none());
            if to < self.vehicles[vi].seats.len() && self.vehicles[vi].seats[to].is_none() && max_ok
            {
                self.vehicles[vi].seats[s as usize] = None;
                self.vehicles[vi].seats[to] = Some(p.id);
                p.seat = Some((v, to as u8));
            }
        }
        self.players[pi].switches_seen = Some(cmd.seat_switches);
    }

    fn move_player(&mut self, pi: usize, dt: f32) {
        let p = &mut self.players[pi];
        // (Light armour has jump jets.)
        p.body.can_jet = p.loadout.armor == weapons::ArmorKind::Light;
        let mut cmd = p.cmd;
        // (A MAX doesn't run.)
        if p.max_suit() {
            cmd.sprint = false;
        }
        let speed = player::move_speed(&cmd, p.loadout.armor.speed());
        let landed;
        if let Some(target) = cmd.feet.filter(|f| f.is_finite()) {
            // The client's prediction: followed as far as the speed limit and the walls allow.
            let from = p.body.feet;
            let mut d = Vec2::new(target.x - from.x, target.z - from.z);
            let max = speed * dt * 1.35 + 0.02;
            if d.length() > max {
                d = d.normalize() * max;
            }
            let vy = if (target.y - from.y) > 0.5 && p.body.on_ground() {
                // A jump the client made.
                p.body.vy.max(0.0)
            } else {
                p.body.vy
            };
            let mut stepped = p.body;
            stepped.vel = d / dt;
            // Jumps happen here too (the same jump count).
            let (feet, vy2) = {
                // Jump bookkeeping as the body does it.
                if let Some(seen) = stepped.jumps_seen
                    && cmd.jumps != seen
                {
                    stepped.jump_wait = player::JUMP_BUFFER;
                }
                stepped.jumps_seen = Some(cmd.jumps);
                let mut vy = vy;
                if stepped.jump_wait > 0.0 {
                    stepped.jump_wait -= dt;
                    if !stepped.jumped && stepped.off_ground <= player::COYOTE {
                        stepped.jump_wait = 0.0;
                        stepped.jumped = true;
                        vy = player::JUMP_SPEED;
                    }
                }
                if stepped.on_ground() {
                    stepped.off_ground = 0.0;
                } else {
                    stepped.off_ground += dt;
                }
                if vy == 0.0 && stepped.off_ground == 0.0 {
                    stepped.jumped = false;
                }
                let vy = stepped.air(&cmd, vy, dt);
                self.world.move_body(from, d, vy, dt)
            };
            let was_falling = p.body.vy != 0.0;
            if vy2 < 0.0 {
                stepped.fall = stepped.fall.max(-vy2);
            }
            landed = (was_falling && vy2 == 0.0).then(|| std::mem::take(&mut stepped.fall));
            if landed.is_some() {
                stepped.chute = false;
            }
            if vy2 == 0.0 && !was_falling {
                stepped.fall = 0.0;
            }
            stepped.feet = ContinentMap::clamp(feet);
            stepped.vy = vy2;
            p.body = stepped;
        } else {
            landed = p.body.step(&self.world, &cmd, speed, dt);
        }
        // Spawn rooms keep enemies out.
        let team = p.team;
        let mut feet = p.body.feet;
        for (c, allowed) in &self.world.shields {
            if *allowed != Some(team) {
                crate::collide::push_out(c, &mut feet);
            }
        }
        self.players[pi].body.feet = feet;
        if let Some(fall) = landed
            && fall > weapons::FALL_SAFE
        {
            let dmg = (fall - weapons::FALL_SAFE) * weapons::FALL_DAMAGE;
            let id = self.players[pi].id;
            self.damage_player(pi, dmg, id, Cause::Fall, feet, false, true);
        }
    }

    /// Firing, reloading, switching and throwing.
    fn weapons(&mut self, pi: usize, dt: f32) {
        if !self.players[pi].alive() {
            return;
        }
        let p = &mut self.players[pi];
        let cmd = p.cmd;
        let slot = p.slot();
        p.grenade_cooldown = (p.grenade_cooldown - dt).max(0.0);
        for (i, g) in p.guns.iter_mut().enumerate() {
            g.cooldown = (g.cooldown - dt).max(0.0);
            if g.reload > 0.0 && i == slot {
                g.reload -= dt;
                if g.reload <= 0.0 {
                    g.reload = 0.0;
                    let s = g.weapon.stats();
                    let take = (s.mag - g.mag).min(g.reserve);
                    g.mag += take;
                    g.reserve -= take;
                }
            } else if i != slot {
                g.reload = 0.0;
            }
        }
        if let Some(seen) = p.reloads_seen
            && seen != cmd.reloads
        {
            let g = &mut p.guns[slot];
            let s = g.weapon.stats();
            if g.reload == 0.0 && g.mag < s.mag && g.reserve > 0 {
                g.reload = s.reload;
            }
        }
        p.reloads_seen = Some(cmd.reloads);
        let throw = p.throws_seen.is_some_and(|s| s != cmd.throws);
        p.throws_seen = Some(cmd.throws);
        // (VR: only with a gun drawn from a holster.)
        let fire = cmd.fire && !cmd.vr.is_some_and(|v| v.gun_hand.is_none());
        let edge = fire && !p.prev_fire;
        p.prev_fire = fire;
        p.firing = false;
        p.beaming = false;
        // (Not inside a warp gate's bubble.)
        let feet = p.body.feet;
        if self.in_bubble(feet + Vec3::Y) {
            return;
        }
        let p = &mut self.players[pi];
        if throw && p.grenades > 0 && p.grenade_cooldown <= 0.0 {
            p.grenades -= 1;
            p.cloaked = false;
            p.grenade_cooldown = 0.8;
            let eye = p.eye();
            let dir = look_dir(cmd.yaw, cmd.pitch + 0.15);
            let vel = dir * weapons::GRENADE_SPEED + Vec3::new(p.body.vel.x, 0.0, p.body.vel.y);
            let (id, team) = (p.id, p.team);
            let ammo = self.players[pi].loadout.grenade.ammo();
            self.launch(ammo, id, team, eye + dir * 0.5, vel, 0, None);
        }
        let p = &mut self.players[pi];
        let g = p.guns[slot];
        let s = g.weapon.stats();
        if g.reload > 0.0 || !fire {
            return;
        }
        if g.mag == 0 {
            if g.reserve > 0 {
                p.guns[slot].reload = s.reload;
            }
            return;
        }
        if g.cooldown > 0.0 || (!s.automatic && !edge) {
            return;
        }
        p.protect = 0.0;
        p.firing = true;
        p.cloaked = false;
        let (id, team, eye) = (p.id, p.team, p.eye());
        let (from, dir) = if let Some(vr) = &cmd.vr {
            (p.body.feet + vr.muzzle.pos, vr.muzzle.forward())
        } else {
            (eye, cmd.look())
        };
        match s.shot {
            ShotKind::Beam => {
                p.guns[slot].cooldown = s.interval;
                p.guns[slot].mag = g.mag.saturating_sub(1);
                p.beaming = true;
                if g.weapon == Weapon::RepairGun {
                    self.repair_beam(pi, from, dir, s.vs_vehicle * s.interval);
                } else {
                    self.heal_beam(pi, from, dir, s.damage * s.interval, s.interval);
                }
            }
            ShotKind::Bolt => {
                if !p.bot {
                    self.stats.push((id, "shots".into(), 1));
                }
                let p = &mut self.players[pi];
                p.guns[slot].cooldown = s.interval;
                p.guns[slot].mag -= 1;
                p.shots = p.shots.wrapping_add(1);
                let seed = (id as u64) << 32 | p.shots as u64;
                let lag = if cmd.view_tick > 0 {
                    self.tick.saturating_sub(cmd.view_tick).min(MAX_LAG)
                } else {
                    0
                };
                let moving = p.body.vel.length() / player::WALK_SPEED;
                let spread = g
                    .weapon
                    .spread_for(cmd.alt, cmd.crouch, moving, !p.body.on_ground());
                let mut rng = Seq::new(seed, 0x5407);
                for _ in 0..s.pellets {
                    let d = jitter(dir, spread, &mut rng);
                    self.launch_locked(Ammo::Gun(g.weapon), id, team, from, d * s.speed, lag, None);
                }
            }
        }
    }

    /// The medgun's beam: heals the nearest hurt team mate along it, or revives a downed one
    /// (its body lying where it fell) after `REVIVE_TIME` on it.
    fn heal_beam(&mut self, pi: usize, from: Vec3, dir: Vec3, heal: f32, interval: f32) {
        let (me, team) = (self.players[pi].id, self.players[pi].team);
        let reach = Weapon::Medgun.stats().near;
        let to = from + dir * reach;
        let wall = self.world.ray(from, to).map_or(1.0, |h| h.0);
        let mut best: Option<(f32, usize)> = None;
        for (i, q) in self.players.iter().enumerate() {
            if q.id == me || q.team != team || q.seat.is_some() {
                continue;
            }
            let (c, radius) = if q.revivable() {
                (q.feet() + Vec3::Y * 0.3, 1.2)
            } else if q.alive() && q.health < 100.0 && !q.max_suit() {
                (q.feet() + Vec3::Y * 1.0, 0.9)
            } else {
                continue;
            };
            let t = (c - from).dot(dir).clamp(0.0, reach);
            // (A body on the floor: the beam may end on the floor just short of it.)
            if t / reach > wall + 1.0 / reach {
                continue;
            }
            let off = (from + dir * t - c).length();
            if off < radius && best.is_none_or(|b| t < b.0) {
                best = Some((t, i));
            }
        }
        let Some((_, i)) = best else { return };
        let tick = self.tick;
        let q = &mut self.players[i];
        if q.alive() {
            let before = q.health;
            q.health = (q.health + heal).min(100.0);
            let healed = q.health - before;
            self.support_xp(pi, Award::Heal, healed, progress::HEAL_XP);
            return;
        }
        q.revive += interval;
        q.revive_tick = tick;
        if q.revive >= REVIVE_TIME {
            self.revive_player(i, me);
        }
    }

    /// The repair gun's beam: repairs the faction's vehicle, turret or generator it meets
    /// (turrets and generators of bases nobody holds too).
    fn repair_beam(&mut self, pi: usize, from: Vec3, dir: Vec3, repair: f32) {
        let team = self.players[pi].team;
        let reach = Weapon::RepairGun.stats().near;
        // Not through walls (a little past the first thing hit: the target's own shell).
        let wall = self
            .world
            .ray(from, from + dir * reach)
            .map_or(1.0, |h| h.0);
        let reach = (reach * wall + 1.5).min(reach);
        let to = from + dir * reach;
        // A MAX of the faction's: its armour, then its health.
        let me = self.players[pi].id;
        let max = self.players.iter().position(|q| {
            q.id != me
                && q.team == team
                && q.afoot()
                && q.max_suit()
                && (q.armor < q.loadout.armor.max() || q.health < 100.0)
                && {
                    let c = q.feet() + Vec3::Y * 1.0;
                    let k = (c - from).dot(dir).clamp(0.0, reach);
                    (from + dir * k - c).length() < 1.1
                }
        });
        if let Some(qi) = max {
            // (Armour mends at three times the vehicle percent rate.)
            let q = &mut self.players[qi];
            let before = q.armor + q.health;
            let max_armor = q.loadout.armor.max();
            let mut left = repair * 3.0;
            let a = (max_armor - q.armor).min(left);
            q.armor += a;
            left -= a;
            q.health = (q.health + left).min(100.0);
            let done = q.armor + q.health - before;
            self.support_xp(pi, Award::Repair, done / 3.0, progress::REPAIR_XP);
            return;
        }
        if let Some(done) = self.repair_deployable(team, from, dir, reach, repair * 2.0) {
            self.support_xp(pi, Award::Repair, done / 2.0, progress::REPAIR_XP);
            return;
        }
        if let Some(done) = self.repair_structure(team, from, to, repair) {
            self.support_xp(pi, Award::Repair, done, progress::REPAIR_XP);
            return;
        }
        for v in &mut self.vehicles {
            if v.team != team || v.health >= 100.0 {
                continue;
            }
            let half = v.kind.spec().half;
            if vehicle::hit_hull(&v.motion, half, from, to).is_some() {
                let before = v.health;
                v.health = (v.health + repair).min(100.0);
                let done = v.health - before;
                self.support_xp(pi, Award::Repair, done, progress::REPAIR_XP);
                return;
            }
        }
        for ti in 0..self.turrets.len() {
            let owner = self.turret_team(ti);
            let t = &mut self.turrets[ti];
            if (owner.is_some() && owner != Some(team)) || t.health >= 100.0 {
                continue;
            }
            let c = t.pos + Vec3::Y * TURRET_PIVOT;
            let k = (c - from).dot(dir).clamp(0.0, reach);
            if (from + dir * k - c).length() < 2.0 {
                let before = t.health;
                t.health = (t.health + repair).min(100.0);
                if t.health >= 100.0 {
                    t.down = 0.0;
                }
                let done = t.health - before.max(0.0);
                self.support_xp(pi, Award::Repair, done, progress::REPAIR_XP);
                return;
            }
        }
        for (b, l) in self.map.bases.iter().enumerate() {
            let Some(g) = l.generator else { continue };
            let owner = self.bases[b].owner;
            if (owner.is_some() && owner != Some(team)) || self.bases[b].generator >= 100.0 {
                continue;
            }
            let c = g + Vec3::Y * GENERATOR_HALF.y;
            let k = (c - from).dot(dir).clamp(0.0, reach);
            if (from + dir * k - c).length() < 2.8 {
                let before = self.bases[b].generator;
                self.bases[b].generator = (before + repair).min(100.0);
                let done = self.bases[b].generator - before;
                self.support_xp(pi, Award::Repair, done, progress::REPAIR_XP);
                return;
            }
        }
    }

    /// Whether `p` is inside a warp gate's bubble (where nobody fires).
    pub fn in_bubble(&self, p: Vec3) -> bool {
        self.map.gates.iter().any(|g| in_gate_bubble(*g, p))
    }

    /// Whether players may spawn at base `b` (a tower always; a base with resources left).
    pub fn can_spawn_at(&self, b: usize) -> bool {
        matches!(self.map.bases[b].kind, BaseKind::Tower | BaseKind::Home)
            || self.bases.get(b).is_some_and(|r| r.resources > 0.0)
    }

    fn enter_turret(&mut self, pi: usize, t: usize) {
        let p = &self.players[pi];
        let id = p.id;
        if !p.afoot() {
            return;
        }
        let Some(tr) = self.turrets.get(t) else {
            return;
        };
        if p.max_suit() {
            return self.deny(id, Deny::Max);
        }
        if self.turret_team(t) != Some(p.team) {
            return self.deny(id, Deny::NotYours);
        }
        if tr.health <= 0.0 || tr.occupant.is_some() {
            return self.deny(id, Deny::Full);
        }
        if (tr.pos + Vec3::Y * 1.0 - p.eye()).length() > 6.5 {
            return self.deny(id, Deny::TooFar);
        }
        self.turrets[t].occupant = Some(id);
        let p = &mut self.players[pi];
        p.turret = Some(t);
        p.cmd.feet = None;
        p.body.vel = Vec2::ZERO;
        p.body.vy = 0.0;
    }

    /// Out of the turret's seat, onto the wall behind it.
    fn leave_turret(&mut self, pi: usize) {
        let Some(t) = self.players[pi].turret.take() else {
            return;
        };
        let tr = &mut self.turrets[t];
        if tr.occupant == Some(self.players[pi].id) {
            tr.occupant = None;
        }
        let back = glam::Quat::from_rotation_y(tr.yaw) * Vec3::Z * 2.4;
        let mut at = tr.pos + back;
        at.y = self.world.ground(at.x, at.z, at.y + 1.5).unwrap_or(at.y);
        let p = &mut self.players[pi];
        p.body = Body::at(at);
        p.cmd.feet = None;
        let (id, yaw) = (p.id, p.cmd.yaw);
        self.say(
            Audience::Player(id),
            SimEvent::Spawned {
                player: id,
                pos: at,
                yaw,
            },
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn launch(
        &mut self,
        ammo: Ammo,
        owner: PlayerId,
        team: Team,
        from: Vec3,
        vel: Vec3,
        lag: u64,
        from_vehicle: Option<VehicleId>,
    ) {
        let id = self.next_projectile;
        self.next_projectile = self.next_projectile.wrapping_add(1).max(1);
        self.projectiles.push(Projectile {
            id,
            ammo,
            owner,
            team,
            pos: from,
            vel,
            age: 0.0,
            dist: 0.0,
            lag,
            from_vehicle,
            homing: None,
        });
        self.fx(Fx::Shot {
            id,
            owner,
            ammo,
            from,
            vel,
            target: None,
        });
    }

    /// Launches as `launch`, homing on `target` if the shooter (`owner`) has it locked.
    #[allow(clippy::too_many_arguments)]
    fn launch_locked(
        &mut self,
        ammo: Ammo,
        owner: PlayerId,
        team: Team,
        from: Vec3,
        vel: Vec3,
        lag: u64,
        from_vehicle: Option<VehicleId>,
    ) {
        let target = self
            .player(owner)
            .and_then(|p| p.lock)
            .filter(|l| l.1 >= weapons::LOCK_TIME && ammo.lock_on().is_some())
            .map(|l| l.0);
        self.launch(ammo, owner, team, from, vel, lag, from_vehicle);
        if let Some(t) = target {
            if let Some(pr) = self.projectiles.last_mut() {
                pr.homing = Some(t);
            }
            if let Some((_, Fx::Shot { target: tg, .. })) = self.fx.back_mut() {
                *tg = Some(t);
            }
        }
    }

    /// Aircraft `vi` lets its flares go (if they're ready): locks on it break, and missiles
    /// homing on it go astray.
    /// Vehicle `vi`'s driver uses its defensive slot (when it's ready).
    pub fn defend(&mut self, vi: usize) {
        let v = &self.vehicles[vi];
        let d = v.loadout.defense;
        if v.flare_wait > 0.0 {
            return;
        }
        match d {
            vehicle::Defense::Flares => self.flare(vi),
            vehicle::Defense::Smoke => {
                let (at, id) = (v.center(), v.id);
                self.smokes.push((at, weapons::SMOKE_TIME * 0.75));
                self.fx(Fx::Impact {
                    id: 0,
                    ammo: Ammo::Utility(weapons::GrenadeKind::Smoke),
                    at,
                    what: ImpactOn::Ground,
                });
                // (Locks on it break.)
                for p in &mut self.players {
                    if p.lock.is_some_and(|l| l.0 == id) {
                        p.lock = None;
                    }
                }
                self.vehicles[vi].flare_wait = d.cooldown();
            }
            vehicle::Defense::FireSuppression => {
                let v = &mut self.vehicles[vi];
                if v.health >= 100.0 {
                    return;
                }
                v.health = (v.health + vehicle::FIRE_SUPPRESSION).min(100.0);
                v.flare_wait = d.cooldown();
            }
            vehicle::Defense::None | vehicle::Defense::Plating => {}
        }
    }

    pub fn flare(&mut self, vi: usize) {
        let v = &mut self.vehicles[vi];
        if !v.kind.aircraft() || v.flare_wait > 0.0 {
            return;
        }
        v.flares = FLARE_TIME;
        v.flare_wait = FLARE_WAIT;
        let id = v.id;
        for p in &mut self.players {
            if p.lock.is_some_and(|l| l.0 == id) {
                p.lock = None;
            }
        }
        for pr in &mut self.projectiles {
            if pr.homing == Some(id) {
                pr.homing = None;
            }
        }
    }

    /// Player `pi`'s lock-on: with a lock-on launcher in hand (the Striker aimed down its
    /// sights), or in a seat with missiles, the enemy vehicle of the right kind (aircraft, or
    /// on the ground for the Striker) nearest its aim within `LOCK_CONE` and reach, in sight
    /// and without flares, held there for `LOCK_TIME`.
    fn step_lock(&mut self, pi: usize, dt: f32) {
        let p = &self.players[pi];
        let ammo = if !p.alive() {
            None
        } else if let Some((vid, seat)) = p.seat {
            self.vehicle(vid).and_then(|v| {
                let gi = v.kind.spec().seats.get(seat as usize)?.alt_gun?;
                Some(v.kind.spec().guns[gi].ammo)
            })
        } else if p.turret.is_none() {
            let w = p.weapon();
            match w {
                Weapon::Skyhook => Some(Ammo::Gun(w)),
                Weapon::Striker if p.cmd.alt => Some(Ammo::Gun(w)),
                _ => None,
            }
        } else {
            None
        };
        let Some(air) = ammo.and_then(|a| a.lock_on()) else {
            self.players[pi].lock = None;
            return;
        };
        let (eye, look, team) = match p
            .seat
            .and_then(|(vid, s)| self.vehicle(vid).map(|v| (v, s)))
        {
            // (A craft's missiles lock where its nose points.)
            Some((v, s)) => (v.seat_eye(s as usize), v.motion.forward(), p.team),
            None => (p.eye(), p.cmd.look(), p.team),
        };
        let reach = if air {
            weapons::LOCK_RANGE_AIR
        } else {
            weapons::LOCK_RANGE_GROUND
        };
        let mine = p.seat.map(|s| s.0);
        let best = self
            .vehicles
            .iter()
            .filter(|v| v.team != team && Some(v.id) != mine && v.flares <= 0.0)
            // (Not through smoke.)
            .filter(|v| !kit::smoked_in(&self.smokes, eye, v.center()))
            .filter(|v| v.flying() == air)
            .filter_map(|v| {
                let c = v.center();
                let d = c - eye;
                let dist = d.length();
                let off = d.normalize_or(Vec3::Y).angle_between(look);
                (dist < reach && off < weapons::LOCK_CONE).then_some((v.id, off, c))
            })
            .filter(|(_, _, c)| {
                // (Not through walls or the land; the target's own hull may be in the way.)
                self.world
                    .ray(eye, *c)
                    .is_none_or(|h| (h.1 - *c).length() < 6.0)
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|t| t.0);
        let p = &mut self.players[pi];
        p.lock = match (best, p.lock) {
            (Some(t), Some((l, held))) if t == l => Some((t, held + dt)),
            (Some(t), _) => Some((t, 0.0)),
            (None, _) => None,
        };
    }

    fn step_vehicles(&mut self, dt: f32) {
        let mut wrecks = Vec::new();
        for vi in 0..self.vehicles.len() {
            let spec = self.vehicles[vi].kind.spec();
            let driver = self.vehicles[vi]
                .driver()
                .and_then(|d| self.player(d))
                .map(|p| p.cmd);
            let v = &mut self.vehicles[vi];
            v.firing = false;
            v.flares = (v.flares - dt).max(0.0);
            v.flare_wait = (v.flare_wait - dt).max(0.0);
            // (A deployed bus stays: it's a spawn point.)
            if v.seats.iter().all(|s| s.is_none()) && !v.deployed {
                v.empty_for += dt;
            } else {
                v.empty_for = 0.0;
            }
            let old_vel = v.motion.vel;
            let mut impact = None;
            match driver.and_then(|c| c.vehicle.map(|p| (c, p))) {
                Some((_, (pos, rot, vel)))
                    if pos.is_finite()
                        && rot.is_finite()
                        && vel.is_finite()
                        && (pos - v.motion.pos).length()
                            < (spec.max_speed + 5.0) * dt * 3.0 + 1.5
                        && !v.deployed =>
                {
                    // The driver's prediction.
                    v.motion.pos = ContinentMap::clamp(pos);
                    v.motion.rot = rot.normalize();
                    v.motion.vel = vel.clamp_length_max(spec.max_speed * 1.5 + 10.0);
                    let (yaw, _, _) = v.motion.rot.to_euler(glam::EulerRot::YXZ);
                    v.motion.yaw = yaw;
                    if spec.motion == MotionKind::Flying {
                        v.motion.throttle = vel.dot(v.motion.forward()).max(0.0);
                        let ground = self
                            .world
                            .ground(pos.x, pos.z, pos.y + 1.0)
                            .unwrap_or(-50.0);
                        v.motion.grounded = pos.y - ground < 0.3 && vel.length() < 0.5;
                    } else {
                        v.motion.grounded = true;
                        v.motion.throttle = vel.dot(v.motion.forward());
                    }
                    let dv = (v.motion.vel - old_vel).length();
                    if dv > 14.0 {
                        impact = Some(dv);
                    }
                }
                _ => {
                    let cmd = driver.map(|c| c.drive);
                    impact = v
                        .motion
                        .step(&spec, cmd.as_ref(), v.deployed, &self.world, dt);
                }
            }
            if let Some(speed) = impact {
                let dmg = ((speed - 10.0) * 2.5).max(0.0);
                if dmg > 0.0 {
                    v.health -= dmg;
                }
            }
            // Rearming and repairing.
            if v.motion.grounded || spec.motion != MotionKind::Flying {
                let near = self.map.bases.iter().enumerate().any(|(bi, b)| {
                    self.bases[bi].owner == Some(v.team)
                        && b.rearm.iter().any(|r| {
                            (Vec2::new(r.x - v.motion.pos.x, r.z - v.motion.pos.z)).length()
                                < REARM_RADIUS
                        })
                });
                if near {
                    v.health = (v.health + 10.0 * dt).min(100.0);
                    let (kind, l) = (v.kind, v.loadout);
                    for (gi, g) in v.guns.iter_mut().enumerate() {
                        let max = kind.gun_spec(l, gi).ammo_max;
                        let add = (max as f32 * 0.15 * dt).ceil() as u16;
                        g.ammo = (g.ammo + add).min(max);
                    }
                }
            }
            if v.health <= 0.0 {
                wrecks.push((v.id, None));
            }
        }
        // Gunners aim and fire; the bus deploys.
        for vi in 0..self.vehicles.len() {
            let spec = self.vehicles[vi].kind.spec();
            for (si, seat) in spec.seats.iter().enumerate() {
                let Some(pid) = self.vehicles[vi].seats[si] else {
                    continue;
                };
                let Some(cmd) = self.player(pid).map(|p| p.cmd) else {
                    continue;
                };
                if seat.driver && self.vehicles[vi].kind != VehicleKind::SpawnBus {
                    // The deploy key: the defensive slot.
                    let pi = self.pidx(pid).unwrap();
                    let pressed = self.players[pi]
                        .deploys_seen
                        .is_some_and(|s| s != cmd.deploys);
                    self.players[pi].deploys_seen = Some(cmd.deploys);
                    if pressed {
                        self.defend(vi);
                    }
                }
                if seat.driver && self.vehicles[vi].kind == VehicleKind::SpawnBus {
                    let pi = self.pidx(pid).unwrap();
                    let seen = self.players[pi].deploys_seen;
                    if let Some(s) = seen
                        && s != cmd.deploys
                        && self.vehicles[vi].motion.vel.length() < 1.5
                    {
                        let v = &mut self.vehicles[vi];
                        v.deployed = !v.deployed;
                    }
                    self.players[pi].deploys_seen = Some(cmd.deploys);
                }
                // The seat's gun on the trigger, its second (missiles) on the other.
                let triggers = [
                    seat.gun.map(|g| (g, cmd.fire)),
                    seat.alt_gun.map(|g| (g, cmd.alt)),
                ];
                for (gi, pulled) in triggers.into_iter().flatten() {
                    let gs = self.vehicles[vi].gun_spec(gi);
                    let m = self.vehicles[vi].motion;
                    // A harvester's beam turns to the nearest node in reach by itself.
                    let harvester = self.vehicles[vi].kind == VehicleKind::Harvester;
                    let node_aim = if harvester {
                        let from = m.to_world(gs.pivot);
                        self.nodes
                            .iter()
                            .filter(|n| n.amount > 0.0)
                            .map(|n| n.pos + Vec3::Y * 1.5)
                            .filter(|c| (*c - from).length() < HARVEST_REACH)
                            .min_by(|a, b| (*a - from).length().total_cmp(&(*b - from).length()))
                            .map(|c| (c - from).normalize_or(Vec3::NEG_Z))
                    } else {
                        None
                    };
                    let silenced = self.in_bubble(m.pos + Vec3::Y);
                    let v = &mut self.vehicles[vi];
                    // Aim: towards the look direction (a harvester: its node, else ahead), at the
                    // traverse rate.
                    let look = match (harvester, node_aim) {
                        (true, Some(d)) => d,
                        (true, None) => m.rot * Vec3::NEG_Z,
                        _ => look_dir(cmd.yaw, cmd.pitch),
                    };
                    let local = m.rot.inverse() * look;
                    let want_yaw = yaw_of(local);
                    let want_pitch = local.y.clamp(-1.0, 1.0).asin();
                    let g = &mut v.guns[gi];
                    if gs.fixed {
                        g.yaw = 0.0;
                        g.pitch = 0.0;
                    } else {
                        let dy = wrap_angle(want_yaw - g.yaw);
                        let step = gs.turn_rate * TICK;
                        g.yaw = wrap_angle(g.yaw + dy.clamp(-step, step));
                        let dp = want_pitch.clamp(gs.pitch_min, gs.pitch_max) - g.pitch;
                        g.pitch += dp.clamp(-step, step);
                    }
                    g.cooldown = (g.cooldown - TICK).max(0.0);
                    if v.kind == VehicleKind::Harvester {
                        // The beam mines the node it points at.
                        v.harvesting = false;
                        if !pulled || v.cargo >= CARGO {
                            continue;
                        }
                        let aim = Quat::from_rotation_y(g.yaw) * Quat::from_rotation_x(g.pitch);
                        let from = m.to_world(gs.pivot + aim * Vec3::new(0.0, 0.0, -gs.barrel));
                        let dir = m.rot * aim * Vec3::NEG_Z;
                        let node = self
                            .nodes
                            .iter_mut()
                            .filter(|n| n.amount > 0.0)
                            .map(|n| {
                                let c = n.pos + Vec3::Y * 1.5;
                                let k = (c - from).dot(dir).clamp(0.0, HARVEST_REACH);
                                ((from + dir * k - c).length(), n)
                            })
                            .filter(|(off, n)| {
                                *off < 4.0 && (n.pos - from).length() < HARVEST_REACH + 4.0
                            })
                            .min_by(|a, b| a.0.total_cmp(&b.0))
                            .map(|(_, n)| n);
                        if let Some(n) = node {
                            let take = (HARVEST_RATE * TICK).min(n.amount).min(CARGO - v.cargo);
                            n.amount -= take;
                            v.cargo += take;
                            v.harvesting = true;
                        }
                        continue;
                    }
                    if !pulled || g.cooldown > 0.0 || g.ammo == 0 || silenced {
                        continue;
                    }
                    g.cooldown = gs.interval;
                    g.ammo -= 1;
                    let barrel = g.barrel;
                    g.barrel = (g.barrel + 1) % gs.barrels.max(1);
                    let aim = Quat::from_rotation_y(g.yaw) * Quat::from_rotation_x(g.pitch);
                    let dir_local = aim * Vec3::NEG_Z;
                    let side = if gs.barrels > 1 {
                        (barrel as f32 - (gs.barrels - 1) as f32 * 0.5) * gs.barrel_gap
                    } else {
                        0.0
                    };
                    let muzzle_local = gs.pivot + aim * Vec3::new(side, 0.0, -gs.barrel);
                    let from = m.to_world(muzzle_local);
                    let mut rng = Seq::new(self.tick ^ (pid as u64) << 20, gi as u64);
                    let dir = jitter(m.rot * dir_local, gs.spread, &mut rng);
                    let speed = gs.ammo.ballistics().speed;
                    let vel = dir * speed + m.vel;
                    v.firing = true;
                    let (team, vid) = (v.team, v.id);
                    let lag = if cmd.view_tick > 0 {
                        self.tick.saturating_sub(cmd.view_tick).min(MAX_LAG)
                    } else {
                        0
                    };
                    self.launch_locked(gs.ammo, pid, team, from, vel, lag, Some(vid));
                }
            }
        }
        // Harvesters unload at their faction's silos; gateways' forcefields keep other
        // factions' vehicles out of powered bases.
        for vi in 0..self.vehicles.len() {
            self.vehicles[vi].unloading = false;
            let v = &self.vehicles[vi];
            let (team, pos, speed) = (v.team, v.motion.pos, v.motion.vel.length());
            if v.kind == VehicleKind::Harvester && v.cargo > 0.0 && speed < 2.0 {
                let silo = self.map.bases.iter().enumerate().find_map(|(b, l)| {
                    let s = l.silo?;
                    (self.bases[b].owner == Some(team)
                        && Vec2::new(s.x - pos.x, s.z - pos.z).length() < SILO_REACH)
                        .then_some(b)
                });
                // (Else a mobile silo of the faction's by it.)
                let mobile = silo.is_none().then(|| self.silo_near(team, pos)).flatten();
                let room = match (silo, mobile) {
                    (Some(b), _) => crate::state::SILO_CAP - self.bases[b].resources,
                    (None, Some(m)) => crate::construct::SILO_HOLDS - self.structures[m].resources,
                    _ => 0.0,
                };
                let give = (UNLOAD_RATE * dt)
                    .min(self.vehicles[vi].cargo)
                    .min(room.max(0.0));
                if give > 0.0 {
                    self.vehicles[vi].cargo -= give;
                    self.vehicles[vi].unloading = true;
                    match (silo, mobile) {
                        (Some(b), _) => self.bases[b].resources += give,
                        (None, Some(m)) => self.structures[m].resources += give,
                        _ => {}
                    }
                    if let Some(di) = self.vehicles[vi].driver().and_then(|d| self.pidx(d)) {
                        self.support_xp(di, Award::Delivery, give, progress::DELIVERY_XP);
                    }
                }
            }
            let half = self.vehicles[vi].kind.spec().half;
            for (b, l) in self.map.bases.iter().enumerate() {
                let rt = &self.bases[b];
                if rt.owner.is_none()
                    || rt.owner == Some(team)
                    || !rt.powered()
                    || l.gate_fields.is_empty()
                {
                    continue;
                }
                if (l.center - pos).length() > l.half * 1.6 {
                    continue;
                }
                let mut p = pos;
                for f in &l.gate_fields {
                    let (a, e) = f.bounds();
                    let r = half.x;
                    // Inside the field's box (grown by the hull's half width): pushed back
                    // out the way it came.
                    if p.x > a.x - r && p.x < e.x + r && p.z > a.z - r && p.z < e.z + r && p.y < e.y
                    {
                        let c = (a + e) / 2.0;
                        let out = l.center - c;
                        let away = if (e.x - a.x) > (e.z - a.z) {
                            // A field across x: out along z, away from the base.
                            Vec3::new(0.0, 0.0, -out.z.signum())
                        } else {
                            Vec3::new(-out.x.signum(), 0.0, 0.0)
                        };
                        let edge = if away.z != 0.0 {
                            if away.z > 0.0 {
                                e.z + r - p.z
                            } else {
                                a.z - r - p.z
                            }
                        } else if away.x > 0.0 {
                            e.x + r - p.x
                        } else {
                            a.x - r - p.x
                        };
                        p += away * edge.abs().min(2.0) * away.dot(away).signum();
                    }
                }
                if p != pos {
                    let v = &mut self.vehicles[vi];
                    v.motion.pos = p;
                    v.motion.vel *= -0.2;
                    v.motion.throttle = 0.0;
                }
            }
        }
        // Running infantry over, bumping into each other.
        for vi in 0..self.vehicles.len() {
            let v = &self.vehicles[vi];
            if v.flying() {
                continue;
            }
            let speed = v.motion.vel.length();
            let half = v.kind.spec().half;
            let (team, kind, m) = (v.team, v.kind, v.motion);
            let driver = v.driver();
            for pi in 0..self.players.len() {
                let p = &self.players[pi];
                if !p.afoot() {
                    continue;
                }
                let d = vehicle::hull_distance(&m, half, p.feet() + Vec3::Y * 0.9);
                if d > 0.45 {
                    continue;
                }
                if speed > ROADKILL_SPEED && p.team != team {
                    let by = driver.unwrap_or(crate::types::HAZARD);
                    let at = p.feet();
                    self.damage_player(pi, speed * 9.0, by, Cause::Roadkill(kind), at, false, true);
                } else {
                    // Pushed aside.
                    let away = (p.feet() - m.pos) * Vec3::new(1.0, 0.0, 1.0);
                    let push = away.normalize_or(Vec3::X) * 0.15;
                    self.players[pi].body.feet += push;
                }
            }
        }
        for a in 0..self.vehicles.len() {
            for b in a + 1..self.vehicles.len() {
                let (va, vb) = (&self.vehicles[a], &self.vehicles[b]);
                if va.flying() != vb.flying() {
                    continue;
                }
                let (ra, rb) = (va.kind.spec().half.x, vb.kind.spec().half.x);
                let (ha, hb) = (va.kind.spec().half.z, vb.kind.spec().half.z);
                let d = va.motion.pos - vb.motion.pos;
                let flat = Vec2::new(d.x, d.z);
                let min = (ra + rb).max((ha + hb) * 0.6);
                if flat.length() >= min || d.y.abs() > 4.0 {
                    continue;
                }
                let n = flat.normalize_or(Vec2::X);
                let push = (min - flat.length()) * 0.5;
                let closing = (vb.motion.vel - va.motion.vel).dot(Vec3::new(n.x, 0.0, n.y));
                self.vehicles[a].motion.pos += Vec3::new(n.x, 0.0, n.y) * push;
                self.vehicles[b].motion.pos -= Vec3::new(n.x, 0.0, n.y) * push;
                if closing > 8.0 {
                    let dmg = (closing - 8.0) * 3.0;
                    self.vehicles[a].health -= dmg;
                    self.vehicles[b].health -= dmg;
                }
            }
        }
        for v in &self.vehicles {
            if v.health <= 0.0 && !wrecks.iter().any(|w| w.0 == v.id) {
                wrecks.push((v.id, None));
            }
        }
        for (id, by) in wrecks {
            self.wreck(id, by.unwrap_or(crate::types::HAZARD));
        }
        // Abandoned vehicles go.
        self.vehicles.retain(|v| v.empty_for < ABANDON_TIME);
    }

    /// Vehicle `id` destroyed (by `by`): its occupants die, it bursts.
    fn wreck(&mut self, id: VehicleId, by: PlayerId) {
        let Some(vi) = self.vidx(id) else { return };
        let v = self.vehicles.remove(vi);
        let at = v.motion.pos + Vec3::Y * 1.5;
        for pid in v.seats.iter().flatten() {
            if let Some(pi) = self.pidx(*pid) {
                self.players[pi].seat = None;
                self.kill(pi, by, Cause::Wreck(v.kind), false);
            }
        }
        self.fx(Fx::Wreck { at, kind: v.kind });
        self.say(
            Audience::Near(at, 800.0),
            SimEvent::VehicleDestroyed {
                id,
                kind: v.kind,
                by,
            },
        );
        if by != crate::types::HAZARD
            && let Some(pi) = self.pidx(by)
            && self.players[pi].team != v.team
        {
            self.players[pi].kills += 1;
            self.award(by, Award::VehicleKill(v.kind), progress::vehicle_xp(v.kind));
            self.stat(by, "vehicle_kills", 1);
            self.spot_kill(crate::events::SpotTarget::Vehicle(v.id), by);
        }
        self.splash(at, 7.0, 60.0, 10.0, by, v.team, Cause::Wreck(v.kind), true);
    }

    /// Whether base `b`'s turrets have power: its generator up and (bases with a silo)
    /// resources left.
    pub fn turrets_powered(&self, b: usize) -> bool {
        let rt = &self.bases[b];
        let kind = self.map.bases[b].kind;
        rt.powered() && (kind == BaseKind::Tower || kind == BaseKind::Home || rt.resources > 0.0)
    }

    fn step_turrets(&mut self, dt: f32) {
        for ti in 0..self.turrets.len() {
            let base = self.turrets[ti].base as usize;
            let owner = self.turret_team(ti);
            let powered = self.turrets_powered(base);
            let amped = self.turret_amped(ti);
            let t = &mut self.turrets[ti];
            if t.health <= 0.0 {
                // Destroyed: down until repaired (or, at length, rebuilt).
                t.down += dt;
                t.target = None;
                if t.down >= TURRET_DOWN_TIME {
                    t.health = 100.0;
                    t.down = 0.0;
                }
                if let Some(occupant) = t.occupant.take()
                    && let Some(pi) = self.pidx(occupant)
                {
                    self.leave_turret(pi);
                }
                continue;
            }
            t.cooldown = (t.cooldown - dt).max(0.0);
            let head = t.pos + Vec3::Y * TURRET_PIVOT;
            let speed = Ammo::TurretBolt.ballistics().speed;
            // Manned: the gunner aims and fires.
            if let Some(occupant) = t.occupant {
                let Some(p) = self.players.iter().find(|p| p.id == occupant && p.alive()) else {
                    t.occupant = None;
                    continue;
                };
                let (cmd, team) = (p.cmd, p.team);
                let step = TURRET_TURN * dt;
                t.yaw = wrap_angle(t.yaw + wrap_angle(cmd.yaw - t.yaw).clamp(-step, step));
                let want = cmd.pitch.clamp(TURRET_PITCH.0, TURRET_PITCH.1);
                t.pitch += (want - t.pitch).clamp(-step, step);
                if cmd.fire && powered && t.cooldown <= 0.0 {
                    t.cooldown = TURRET_INTERVAL;
                    t.barrel = !t.barrel;
                    let mut rng = Seq::new(self.tick ^ 0x7a11, ti as u64);
                    let d = jitter(look_dir(t.yaw, t.pitch), TURRET_SPREAD * 0.5, &mut rng);
                    let side = Vec3::new(-d.z, 0.0, d.x).normalize_or(Vec3::X)
                        * if t.barrel { 0.55 } else { -0.55 };
                    let from = head + d * TURRET_BARREL + side;
                    let lag = if cmd.view_tick > 0 {
                        self.tick.saturating_sub(cmd.view_tick).min(MAX_LAG)
                    } else {
                        0
                    };
                    self.fx(Fx::Muzzle {
                        at: from,
                        ammo: Ammo::TurretBolt,
                    });
                    self.launch(Ammo::TurretBolt, occupant, team, from, d * speed, lag, None);
                    self.drain(base, TURRET_SHOT_COST);
                }
                continue;
            }
            let Some(team) = owner.filter(|_| powered) else {
                t.target = None;
                continue;
            };
            t.think -= dt;
            if t.think <= 0.0 {
                t.think = 0.5;
                t.target = None;
                let mut best = f32::MAX;
                for p in &self.players {
                    if !p.afoot() || p.team == team || p.hidden_from(head) {
                        continue;
                    }
                    let c = p.feet() + Vec3::Y * 1.0;
                    let d = (c - head).length();
                    if d < TURRET_RANGE
                        && d < best
                        && self.world.ray(head, c).is_none()
                        && !kit::smoked_in(&self.smokes, head, c)
                    {
                        best = d;
                        t.target = Some(Target::Player(p.id));
                    }
                }
                for v in &self.vehicles {
                    if v.team == team {
                        continue;
                    }
                    let c = v.motion.pos + Vec3::Y * 1.5;
                    let d = (c - head).length();
                    let range = if v.flying() {
                        TURRET_AIR_RANGE
                    } else {
                        TURRET_RANGE
                    };
                    if v.kind == VehicleKind::SpawnBus && v.deployed && d > CLOAK_SEEN {
                        continue;
                    }
                    if d < range
                        && d < best
                        && self.world.ray(head, c).is_none()
                        && !kit::smoked_in(&self.smokes, head, c)
                    {
                        best = d;
                        t.target = Some(Target::Vehicle(v.id));
                    }
                }
            }
            let aim = match t.target {
                Some(Target::Player(id)) => self
                    .players
                    .iter()
                    .find(|p| p.id == id && p.afoot())
                    .map(|p| {
                        (
                            p.feet() + Vec3::Y * 1.0,
                            Vec3::new(p.body.vel.x, p.body.vy, p.body.vel.y),
                        )
                    }),
                Some(Target::Vehicle(id)) => self
                    .vehicles
                    .iter()
                    .find(|v| v.id == id)
                    .map(|v| (v.motion.pos + Vec3::Y * 1.5, v.motion.vel)),
                None => None,
            };
            let Some((at, vel)) = aim else {
                t.target = None;
                continue;
            };
            let lead = at + vel * ((at - head).length() / speed);
            let dir = (lead - head).normalize_or(Vec3::NEG_Z);
            let want_yaw = yaw_of(dir);
            let want_pitch = dir.y.asin();
            let step = 2.5 * dt;
            let t = &mut self.turrets[ti];
            t.yaw = wrap_angle(t.yaw + wrap_angle(want_yaw - t.yaw).clamp(-step, step));
            let want_pitch = want_pitch.clamp(TURRET_PITCH.0, TURRET_PITCH.1);
            t.pitch += (want_pitch - t.pitch).clamp(-step, step);
            let off = wrap_angle(want_yaw - t.yaw).abs() + (want_pitch - t.pitch).abs();
            if off < 0.06 && t.cooldown <= 0.0 {
                // (Left to itself, a turret fires at a fraction of a gunner's rate; quicker
                // amped.)
                t.cooldown = TURRET_INTERVAL / TURRET_AUTO_RATE * if amped { 0.6 } else { 1.0 };
                t.barrel = !t.barrel;
                let mut rng = Seq::new(self.tick, ti as u64);
                let d = jitter(look_dir(t.yaw, t.pitch), TURRET_SPREAD, &mut rng);
                let side = Vec3::new(-d.z, 0.0, d.x).normalize_or(Vec3::X)
                    * if t.barrel { 0.55 } else { -0.55 };
                let from = head + d * TURRET_BARREL + side;
                self.fx(Fx::Muzzle {
                    at: from,
                    ammo: Ammo::TurretBolt,
                });
                self.launch(
                    Ammo::TurretBolt,
                    crate::types::HAZARD,
                    team,
                    from,
                    d * speed,
                    0,
                    None,
                );
                self.drain(base, TURRET_SHOT_COST);
            }
        }
    }

    /// Resource nodes: at first `NODES` of them; a mined out one goes, and another grows
    /// somewhere open `NODE_REGROW` later.
    fn step_nodes(&mut self, dt: f32) {
        let before = self.nodes.len();
        self.nodes.retain(|n| n.amount > 0.5);
        for _ in self.nodes.len()..before {
            let t = self.rng.range(NODE_REGROW.0, NODE_REGROW.1);
            self.regrow.push(t);
        }
        // (At the start, all at once.)
        if !self.nodes_seeded {
            self.nodes_seeded = true;
            self.regrow = vec![0.0; NODES.saturating_sub(self.nodes.len())];
        }
        for t in &mut self.regrow {
            *t -= dt;
        }
        let mut k = 0;
        while k < self.regrow.len() {
            if self.regrow[k] > 0.0 || self.nodes.len() >= NODES {
                k += 1;
                continue;
            }
            if self.grow_node() {
                self.regrow.swap_remove(k);
            } else {
                // (Nowhere open found: try again shortly.)
                self.regrow[k] = 5.0;
                k += 1;
            }
        }
    }

    /// A new node somewhere open: on dry, gentle land clear of bases, gates, other nodes and
    /// scenery. Whether one was placed.
    fn grow_node(&mut self) -> bool {
        let Some(terrain) = self.world.terrain.clone() else {
            return false;
        };
        for _ in 0..40 {
            let a = self.rng.range(0.0, std::f32::consts::TAU);
            let r = self.rng.range(200.0, 1550.0);
            let (x, z) = (a.cos() * r, a.sin() * r);
            let y = terrain.height(x, z);
            if y < 2.0 || terrain.water(x, z) > y - 0.3 || terrain.slope(x, z) > 0.45 {
                continue;
            }
            let p = Vec3::new(x, y, z);
            if self
                .map
                .bases
                .iter()
                .any(|b| (b.center - p).length() < b.half + 120.0)
                || self.map.gates.iter().any(|g| (*g - p).length() < 120.0)
                || self.nodes.iter().any(|n| (n.pos - p).length() < 250.0)
            {
                continue;
            }
            // (Not under or in scenery: open from above and from round about.)
            let c = p + Vec3::Y;
            let blocked = self.world.ray(c + Vec3::Y * 80.0, c).is_some()
                || [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z]
                    .iter()
                    .any(|d| self.world.ray(c + *d * 6.0, c).is_some());
            if blocked {
                continue;
            }
            let id = self.next_node;
            self.next_node += 1;
            let size = NODE_SIZES[(self.rng.f() * NODE_SIZES.len() as f32) as usize % 3];
            self.nodes.push(ResourceNode {
                id,
                pos: p,
                amount: size * CARGO,
            });
            return true;
        }
        false
    }

    /// Takes `amount` from base `b`'s silo (bases with one; sanctuaries' never run dry).
    fn drain(&mut self, b: usize, amount: f32) {
        let kind = self.map.bases[b].kind;
        if matches!(kind, BaseKind::Tower | BaseKind::Home) {
            return;
        }
        let rt = &mut self.bases[b];
        let before = rt.resources;
        rt.resources = (rt.resources - amount).max(0.0);
        if before > 0.0 && rt.resources <= 0.0 && rt.owner.is_some() {
            self.drained.push(b as BaseIdx);
        }
    }

    fn step_projectiles(&mut self, dt: f32) {
        let mut i = 0;
        while i < self.projectiles.len() {
            let mut pr = self.projectiles[i];
            let b = pr.ammo.ballistics();
            pr.vel.y -= b.gravity * dt;
            // A missile turns towards what it's locked on.
            if let Some(t) = pr.homing {
                match self.vehicle(t) {
                    Some(v) if v.flares <= 0.0 => {
                        pr.vel = home(pr.vel, v.center() - pr.pos, dt);
                    }
                    _ => pr.homing = None,
                }
            }
            let next = pr.pos + pr.vel * dt;
            let view = self.tick.saturating_sub(pr.lag);
            // The nearest thing along the way.
            let mut hit: Option<(f32, HitOn)> = None;
            // (Every forcefield stops shots, its own faction's too.)
            if let Some((f, at, n)) = self.world.ray(pr.pos, next) {
                let shield = self.world.shields.iter().any(|(c, _)| {
                    let (a, b2) = c.bounds();
                    at.cmpge(a - 0.05).all() && at.cmple(b2 + 0.05).all()
                });
                hit = Some((f, HitOn::World(n, shield)));
            }
            // Warp gates' bubbles stop shots going in or out.
            for g in &self.map.gates {
                if let Some(f) = bubble_crossing(*g, pr.pos, next)
                    && hit.is_none_or(|h| f < h.0)
                {
                    // (The shell's normal, facing where the shot came from.)
                    let d = pr.pos.lerp(next, f) - *g;
                    let r2 = crate::base::GATE_BUBBLE_R.powi(2);
                    let n = Vec3::new(d.x / r2, d.y / crate::base::GATE_BUBBLE_H.powi(2), d.z / r2)
                        .normalize_or(Vec3::Y);
                    let n = if in_gate_bubble(*g, pr.pos) { -n } else { n };
                    hit = Some((f, HitOn::World(n, true)));
                }
            }
            for (pi, p) in self.players.iter().enumerate() {
                if !p.afoot() || p.team == pr.team || p.id == pr.owner {
                    continue;
                }
                let (feet, crouch) = p.at_tick(view);
                if let Some((f, head)) = player::hit_body(feet, crouch, pr.pos, next)
                    && hit.is_none_or(|h| f < h.0)
                {
                    hit = Some((f, HitOn::Player(pi, head)));
                }
            }
            for (vi, v) in self.vehicles.iter().enumerate() {
                if v.team == pr.team || Some(v.id) == pr.from_vehicle {
                    continue;
                }
                let m = v.at_tick(view);
                let half = v.kind.spec().half;
                if let Some(f) = vehicle::hit_hull(&m, half, pr.pos, next)
                    && hit.is_none_or(|h| f < h.0)
                {
                    hit = Some((f, HitOn::Vehicle(vi)));
                }
                if b.airburst > 0.0 && v.flying() {
                    let c = m.pos + Vec3::Y * half.y;
                    let seg = next - pr.pos;
                    let k =
                        ((c - pr.pos).dot(seg) / seg.length_squared().max(1e-6)).clamp(0.0, 1.0);
                    if (pr.pos + seg * k - c).length() < b.airburst && hit.is_none_or(|h| k < h.0) {
                        hit = Some((k, HitOn::Burst));
                    }
                }
            }
            {
                for (ti, t) in self.turrets.iter().enumerate() {
                    if t.health <= 0.0 || self.turret_team(ti).is_none_or(|o| o == pr.team) {
                        continue;
                    }
                    let c = t.pos + Vec3::Y * TURRET_PIVOT;
                    let seg = next - pr.pos;
                    let k =
                        ((c - pr.pos).dot(seg) / seg.length_squared().max(1e-6)).clamp(0.0, 1.0);
                    if (pr.pos + seg * k - c).length() < 1.6 && hit.is_none_or(|h| k < h.0) {
                        hit = Some((k, HitOn::Turret(ti)));
                    }
                }
            }
            if let Some((k, di)) = self.hit_deployable(pr.pos, next, pr.team)
                && hit.is_none_or(|h| k < h.0)
            {
                hit = Some((k, HitOn::Deployable(di)));
            }
            // (Its own face is where the world's ray met it too: the structure wins the tie.)
            if let Some((k, si)) = self.hit_structure(pr.pos, next, pr.team)
                && hit.is_none_or(|h| k <= h.0 + 1e-3)
            {
                hit = Some((k, HitOn::Structure(si)));
            }
            for (b, l) in self.map.bases.iter().enumerate() {
                let Some(g) = l.generator else { continue };
                let rt = &self.bases[b];
                if rt.owner.is_none_or(|o| o == pr.team) || !rt.powered() {
                    continue;
                }
                let (lo, hi) = (
                    g - Vec3::new(GENERATOR_HALF.x, 0.0, GENERATOR_HALF.z),
                    g + Vec3::new(GENERATOR_HALF.x, GENERATOR_HALF.y * 2.0, GENERATOR_HALF.z),
                );
                let seg = next - pr.pos;
                let len = seg.length();
                if len > 1e-6
                    && let Some((t, _)) = crate::collide::ray_box(pr.pos, seg / len, lo, hi, len)
                    && hit.is_none_or(|h| t / len <= h.0 + 1e-3)
                {
                    hit = Some((t / len, HitOn::Generator(b)));
                }
            }
            let travelled = (next - pr.pos).length();
            match hit {
                Some((f, HitOn::World(n, _))) if b.bounces => {
                    // A grenade bounces.
                    let at = pr.pos + (next - pr.pos) * f;
                    let vn = pr.vel.dot(n);
                    pr.vel = (pr.vel - n * vn * 1.6) * 0.55;
                    pr.pos = at + n * 0.05;
                    if pr.vel.length() < 0.5 {
                        pr.vel = Vec3::ZERO;
                    } else {
                        // Its new flight, for the clients to follow.
                        self.fx(Fx::Shot {
                            id: pr.id,
                            owner: pr.owner,
                            ammo: pr.ammo,
                            from: pr.pos,
                            vel: pr.vel,
                            target: None,
                        });
                    }
                }
                Some((f, on)) => {
                    let at = pr.pos + (next - pr.pos) * f;
                    pr.dist += travelled * f;
                    self.projectiles.swap_remove(i);
                    self.impact(pr, at, on);
                    continue;
                }
                None => {
                    pr.pos = next;
                    pr.dist += travelled;
                }
            }
            pr.age += dt;
            if pr.age >= b.life {
                self.projectiles.swap_remove(i);
                if b.splash > 0.0 {
                    self.impact(pr, pr.pos, HitOn::Burst);
                }
                continue;
            }
            self.projectiles[i] = pr;
            i += 1;
        }
    }

    fn impact(&mut self, pr: Projectile, at: Vec3, on: HitOn) {
        let b = pr.ammo.ballistics();
        let what = match on {
            HitOn::World(_, true) => ImpactOn::Shield,
            HitOn::World(..) => ImpactOn::Ground,
            HitOn::Player(..) => ImpactOn::Body,
            HitOn::Vehicle(_)
            | HitOn::Turret(_)
            | HitOn::Generator(_)
            | HitOn::Deployable(_)
            | HitOn::Structure(_) => ImpactOn::Vehicle,
            HitOn::Burst => ImpactOn::Air,
        };
        self.fx(Fx::Impact {
            id: pr.id,
            ammo: pr.ammo,
            at,
            what,
        });
        let cause = if pr.owner == crate::types::HAZARD {
            Cause::Turret
        } else {
            Cause::Weapon(pr.ammo)
        };
        match on {
            HitOn::Player(pi, head) => {
                if matches!(pr.ammo, Ammo::Gun(_)) {
                    self.stat(pr.owner, "hits", 1);
                }
                let mut dmg = pr.ammo.damage_at(pr.dist);
                if head {
                    dmg *= b.headshot;
                }
                self.damage_player(pi, dmg, pr.owner, cause, pr.pos, head, false);
            }
            HitOn::Vehicle(vi) => {
                let id = self.vehicles[vi].id;
                self.damage_vehicle(id, b.vs_vehicle, pr.owner);
            }
            HitOn::Generator(b) => {
                self.damage_generator(b, b_vs_structure(&b_of(pr.ammo)), pr.owner);
            }
            HitOn::Structure(si) => {
                self.damage_structure(si, b_vs_structure(&b), pr.owner);
            }
            HitOn::Deployable(di) => {
                self.damage_deployable(di, b.vs_vehicle.max(b.damage * 0.4), pr.owner);
            }
            HitOn::Turret(ti) => {
                let amp = self.turret_amped(ti);
                let t = &mut self.turrets[ti];
                // (Infantry guns chip at turrets too; an Amp Station halves it.)
                t.health -= b_vs_structure(&b) * if amp { 0.5 } else { 1.0 };
                if pr.owner != crate::types::HAZARD {
                    self.say(
                        Audience::Player(pr.owner),
                        SimEvent::HitConfirm {
                            by: pr.owner,
                            vehicle: true,
                            headshot: false,
                            kill: false,
                        },
                    );
                }
            }
            _ => {}
        }
        if let Ammo::Utility(k) = pr.ammo {
            self.utility_burst(k, at, pr.owner, pr.team);
        } else if b.splash > 0.0 {
            self.splash(
                at,
                b.splash,
                b.splash_damage,
                b.splash_vehicle,
                pr.owner,
                pr.team,
                cause,
                false,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn splash(
        &mut self,
        at: Vec3,
        r: f32,
        dmg: f32,
        vdmg: f32,
        by: PlayerId,
        team: Team,
        cause: Cause,
        everyone: bool,
    ) {
        for pi in 0..self.players.len() {
            let p = &self.players[pi];
            if !p.afoot() || (!everyone && p.team == team && p.id != by) {
                continue;
            }
            let d = (p.feet() + Vec3::Y * 0.9 - at).length();
            if d < r {
                let amount = dmg * (1.0 - d / r);
                self.damage_player(pi, amount, by, cause, at, false, false);
            }
        }
        let hits: Vec<(VehicleId, f32)> = self
            .vehicles
            .iter()
            .filter(|v| everyone || v.team != team)
            .filter_map(|v| {
                let half = v.kind.spec().half;
                let d = vehicle::hull_distance(&v.motion, half, at);
                (d < r).then(|| (v.id, vdmg * (1.0 - d / r)))
            })
            .collect();
        for (id, amount) in hits {
            if amount > 0.0 {
                self.damage_vehicle(id, amount, by);
            }
        }
        if vdmg > 0.0 {
            self.splash_structures(at, r + 0.5, vdmg, by, team);
        }
        // Deployables of other factions.
        for di in 0..self.deployables.len() {
            let d = &self.deployables[di];
            let dist = (d.center() - at).length();
            if dist < r + 0.5 && (everyone || d.team != team) && d.health > 0.0 {
                let k = 1.0 - dist / (r + 0.5);
                self.deployables[di].health -= vdmg.max(dmg * 0.4) * k;
            }
        }
        // Turrets and generators of other factions.
        if vdmg > 0.0 {
            for ti in 0..self.turrets.len() {
                let t = &self.turrets[ti];
                let owner = self.turret_team(ti);
                let d = (t.pos + Vec3::Y * TURRET_PIVOT - at).length();
                if d < r + 1.0 && (everyone || owner.is_some_and(|o| o != team)) && t.health > 0.0 {
                    let k = if self.turret_amped(ti) { 0.5 } else { 1.0 };
                    self.turrets[ti].health -= vdmg * (1.0 - d / (r + 1.0)) * k;
                }
            }
            for b in 0..self.bases.len() {
                if let Some(g) = self.map.bases[b].generator {
                    let d = (g + Vec3::Y * GENERATOR_HALF.y - at).length();
                    if d < r + 2.0 && (everyone || self.bases[b].owner.is_some_and(|o| o != team)) {
                        self.damage_generator(b, vdmg * (1.0 - d / (r + 2.0)), by);
                    }
                }
            }
        }
    }

    /// Damages player `pi` by `amount` (armor first unless `pierce`).
    #[allow(clippy::too_many_arguments)]
    pub fn damage_player(
        &mut self,
        pi: usize,
        amount: f32,
        by: PlayerId,
        cause: Cause,
        from: Vec3,
        head: bool,
        pierce: bool,
    ) {
        let p = &mut self.players[pi];
        if !p.alive() || p.protect > 0.0 || amount <= 0.0 {
            return;
        }
        let attacker_team = self.players.iter().find(|q| q.id == by).map(|q| q.team);
        let p = &mut self.players[pi];
        if attacker_team == Some(p.team) && by != p.id {
            return;
        }
        if pierce {
            p.health = (p.health - amount).max(0.0);
        } else {
            (p.health, p.armor) = weapons::apply_damage(p.health, p.armor, amount);
        }
        if attacker_team.is_some_and(|t| t != p.team) {
            let tick = self.tick;
            p.hurt_by.retain(|h| h.2 + progress::ASSIST_TICKS > tick);
            p.hurt_by.push((by, amount, tick));
        }
        let victim = p.id;
        let dead = p.health <= 0.0;
        self.say(
            Audience::Player(victim),
            SimEvent::Hurt {
                victim,
                by,
                amount,
                from,
            },
        );
        if by != victim && by != crate::types::HAZARD {
            self.say(
                Audience::Player(by),
                SimEvent::HitConfirm {
                    by,
                    vehicle: false,
                    headshot: head,
                    kill: dead,
                },
            );
        }
        if dead {
            self.kill(pi, by, cause, head);
        }
    }

    pub fn damage_generator(&mut self, b: usize, amount: f32, by: PlayerId) {
        let attacker = self.players.iter().find(|q| q.id == by).map(|q| q.team);
        let rt = &mut self.bases[b];
        if rt.owner.is_none() || attacker == rt.owner || rt.generator <= 0.0 {
            return;
        }
        rt.generator = (rt.generator - amount).max(0.0);
        if by != crate::types::HAZARD {
            self.say(
                Audience::Player(by),
                SimEvent::HitConfirm {
                    by,
                    vehicle: true,
                    headshot: false,
                    kill: false,
                },
            );
        }
    }

    pub fn damage_vehicle(&mut self, id: VehicleId, amount: f32, by: PlayerId) {
        let Some(vi) = self.vidx(id) else { return };
        let attacker_team = self.players.iter().find(|q| q.id == by).map(|q| q.team);
        if attacker_team == Some(self.vehicles[vi].team) {
            return;
        }
        // (Armour plating keeps some off.)
        let amount = if self.vehicles[vi].loadout.defense == vehicle::Defense::Plating {
            amount * (1.0 - vehicle::PLATING)
        } else {
            amount
        };
        self.vehicles[vi].health -= amount;
        if by != crate::types::HAZARD {
            self.say(
                Audience::Player(by),
                SimEvent::HitConfirm {
                    by,
                    vehicle: true,
                    headshot: false,
                    kill: self.vehicles[vi].health <= 0.0,
                },
            );
        }
        if self.vehicles[vi].health <= 0.0 {
            self.wreck(id, by);
        }
    }

    pub fn kill(&mut self, pi: usize, by: PlayerId, cause: Cause, head: bool) {
        // (Revivable only where it fell on foot.)
        let afoot = self.players[pi].afoot() && !matches!(cause, Cause::Suicide | Cause::Wreck(_));
        self.unseat(pi);
        if let Some(t) = self.players[pi].turret.take()
            && let Some(tr) = self.turrets.get_mut(t)
        {
            tr.occupant = None;
        }
        let p = &mut self.players[pi];
        if !p.alive() {
            return;
        }
        p.cloaked = false;
        p.hacking = None;
        p.carrying = None;
        p.health = 0.0;
        p.dead = Some(0.0);
        p.downed = afoot;
        p.revive = 0.0;
        p.deaths += 1;
        p.firing = false;
        p.beaming = false;
        let victim = p.id;
        let team = p.team;
        let tick = self.tick;
        let hurt_by = std::mem::take(&mut p.hurt_by);
        let mut killed = false;
        if by != victim
            && let Some(k) = self.players.iter_mut().find(|q| q.id == by)
            && k.team != team
        {
            k.kills += 1;
            killed = true;
        }
        // Stats, and the killcam's word to the one killed.
        let cause_name = cause.name();
        self.stat(victim, "deaths", 1);
        self.stat(victim, &format!("deaths:{cause_name}"), 1);
        if killed {
            self.stat(by, "kills", 1);
            self.stat(by, &format!("kills:{cause_name}"), 1);
            if head {
                self.stat(by, "headshots", 1);
            }
        }
        if by != victim
            && let Some(k) = self.player(by)
        {
            let at = self.players[pi].feet();
            let info = SimEvent::DeathInfo {
                by,
                cause,
                distance: (k.feet() - at).length(),
                killer_at: k.alive().then(|| k.eye()),
                killer_health: k.health,
                killer_armor: k.armor,
            };
            self.say(Audience::Player(victim), info);
        }
        if killed {
            self.spot_kill(crate::events::SpotTarget::Player(victim), by);
            self.award(by, Award::Kill, progress::KILL_XP);
            if head {
                self.award(by, Award::Headshot, progress::HEADSHOT_XP);
            }
        }
        // Assists: enough damage lately from others than the killer.
        let mut helped: Vec<(PlayerId, f32)> = Vec::new();
        for &(who, amount, t) in &hurt_by {
            if who == by || t + progress::ASSIST_TICKS <= tick {
                continue;
            }
            match helped.iter_mut().find(|h| h.0 == who) {
                Some(h) => h.1 += amount,
                None => helped.push((who, amount)),
            }
        }
        for (who, dmg) in helped {
            if dmg >= progress::ASSIST_DAMAGE
                && let Some(ai) = self.pidx(who)
                && self.players[ai].team != team
            {
                self.players[ai].assists += 1;
                self.award(who, Award::Assist, progress::ASSIST_XP);
            }
        }
        self.say(
            Audience::All,
            SimEvent::Killed {
                victim,
                by,
                cause,
                headshot: head,
            },
        );
    }

    /// Players and vehicles in a warp gate's core.
    fn check_gates(&mut self) {
        for (gi, g) in self.map.gates.iter().enumerate() {
            let inside = |p: Vec3| {
                Vec2::new(p.x - g.x, p.z - g.z).length() < GATE_CORE
                    && p.y < g.y + GATE_H
                    && p.y > g.y - 3.0
            };
            for v in &self.vehicles {
                if v.driver().is_some() && inside(v.motion.pos) {
                    self.warps.push((Warper::Vehicle(v.id), gi as u8));
                }
            }
            for p in &self.players {
                if p.afoot() && inside(p.feet()) {
                    self.warps.push((Warper::Player(p.id), gi as u8));
                }
            }
        }
    }

    /// Where something arriving through gate `gate` appears: outside the core, towards the
    /// continent's middle, facing it.
    pub fn arrival(&self, gate: u8, vehicle: bool) -> (Vec3, f32) {
        let g = self.map.gates[gate as usize];
        let inward = Vec3::new(-g.x, 0.0, -g.z).normalize_or(Vec3::Z);
        let off = if vehicle {
            GATE_CORE + 22.0
        } else {
            GATE_CORE + 8.0
        };
        let mut at = g + inward * off;
        let side = Vec3::new(-inward.z, 0.0, inward.x) * self.rng.clone().range(-8.0, 8.0);
        at += side;
        at.y = self.world.ground(at.x, at.z, at.y + 5.0).unwrap_or(at.y);
        (at, yaw_of(inward))
    }

    /// The snapshot for player `viewer`: what it can see from where it is.
    pub fn snapshot_for(&self, viewer: PlayerId, since: u64) -> Snapshot {
        let me = self.player(viewer);
        let (eye, team) = me.map_or((Vec3::ZERO, None), |p| (p.eye(), Some(p.team)));
        let my_vehicle = me.and_then(|p| p.seat).map(|s| s.0);
        let d2 = |p: Vec3| (p - eye).length();
        let cloaked =
            |v: &Vehicle| v.kind == VehicleKind::SpawnBus && v.deployed && Some(v.team) != team;
        let vehicles: Vec<VehicleSnap> = self
            .vehicles
            .iter()
            .filter(|v| {
                let d = d2(v.motion.pos);
                let range = if v.flying() { VIEW_AIR } else { VIEW_VEHICLE };
                Some(v.id) == my_vehicle || (d < range && (!cloaked(v) || d < CLOAK_SEEN))
            })
            .map(|v| {
                let mut s = v.snap(cloaked(v) || (v.kind == VehicleKind::SpawnBus && v.deployed));
                if team.is_some_and(|t| {
                    t != v.team && self.spotted(t, crate::events::SpotTarget::Vehicle(v.id))
                }) {
                    s.flags |= crate::snapshot::vflags::SPOTTED;
                }
                s
            })
            .collect();
        let spotters = team.map_or(Vec::new(), |t| self.spotted_by(t));
        let players = self
            .players
            .iter()
            .filter(|p| p.id != viewer)
            // (A cloaked enemy only near.)
            .filter(|p| Some(p.team) == team || !p.hidden_from(eye))
            .filter(|p| match p.seat {
                Some((v, _)) => vehicles.iter().any(|s| s.id == v),
                None => d2(p.feet()) < VIEW_INFANTRY,
            })
            // (The dead for a moment; the downed for as long as their faction can revive them.)
            .filter(|p| {
                p.alive()
                    || p.dead.is_some_and(|t| t < 3.0)
                    || (p.revivable() && Some(p.team) == team)
            })
            .map(|p| {
                let mut s = p.snap(d2(p.feet()) < POSE_RANGE);
                if Some(p.team) != team {
                    s.flags &= !flags::DOWNED;
                    // (Seen by the viewer's faction's interlink.)
                    let near_ours = spotters
                        .iter()
                        .any(|c| (*c - p.feet()).length() < build::INTERLINK_RANGE);
                    let spotted = team
                        .is_some_and(|t| self.spotted(t, crate::events::SpotTarget::Player(p.id)));
                    if p.alive() && !p.cloaked && (near_ours || spotted) {
                        s.flags |= flags::SPOTTED;
                    }
                }
                s
            })
            .chain(me.map(|p| p.snap(true)))
            .collect();
        let turrets = self
            .turrets
            .iter()
            .enumerate()
            .filter(|(_, t)| d2(t.pos) < VIEW_VEHICLE * 0.7)
            .map(|(ti, t)| TurretSnap {
                index: t.index,
                base: t.base,
                yaw: t.yaw,
                pitch: t.pitch,
                health: t.health.max(0.0),
                team: self.turret_team(ti),
                occupant: t.occupant,
                powered: self.turrets_powered(t.base as usize),
            })
            .collect();
        let bases = self
            .bases
            .iter()
            .map(|b| {
                let mut hacked = 0u16;
                let mut hacker = None;
                for (i, h) in b.hacked.iter().enumerate().take(16) {
                    if let Some((t, _)) = h {
                        hacked |= 1 << i;
                        hacker = Some(*t);
                    }
                }
                let points = (b.points.len() == 3).then(|| {
                    std::array::from_fn(|i| {
                        let p = &b.points[i];
                        crate::snapshot::PointSnap {
                            owner: p.owner,
                            by: p.by,
                            // (In steps of 2 %: fewer changes to send.)
                            progress: ((p.progress * 50.0).round() as u8).min(50) * 2,
                            contested: p.contested,
                        }
                    })
                });
                crate::snapshot::BaseSnap {
                    generator: b.generator,
                    resources: b.resources.round() as u16,
                    hacked,
                    hacker,
                    points,
                }
            })
            .collect();
        let nodes = self
            .nodes
            .iter()
            .filter(|n| d2(n.pos) < VIEW_VEHICLE * 1.3)
            .map(|n| crate::snapshot::NodeSnap {
                id: n.id,
                pos: n.pos,
                amount: n.amount.round() as u16,
            })
            .collect();
        let fx = self
            .fx
            .iter()
            .filter(|(t, f)| {
                *t > since
                    && match f {
                        // (One's own shots the client shows itself; not its grenades.)
                        Fx::Shot { owner, ammo, .. }
                            if *owner == viewer && !ammo.ballistics().bounces =>
                        {
                            false
                        }
                        Fx::Shot { from, .. } => d2(*from) < 700.0,
                        _ => d2(f.at()) < 1000.0,
                    }
            })
            .map(|(_, f)| *f)
            .collect();
        Snapshot {
            tick: self.tick,
            players,
            vehicles,
            turrets,
            bases,
            nodes,
            deployables: self.deploy_snaps(team, eye),
            structures: self.structure_snaps(team, eye),
            me: me.map(|p| {
                let mut m = p.me();
                // In a vehicle: whether enemies lock on to it or a missile is coming.
                if let Some(vid) = my_vehicle {
                    let locking = self
                        .players
                        .iter()
                        .any(|q| q.team != p.team && q.lock.is_some_and(|l| l.0 == vid));
                    let missile = self.projectiles.iter().any(|pr| pr.homing == Some(vid));
                    m.warning = if missile {
                        2
                    } else if locking {
                        1
                    } else {
                        0
                    };
                    m.flare_wait = self.vehicle(vid).map_or(0.0, |v| v.flare_wait);
                }
                m.dead = p.dead.map(|t| (self.respawn_time(p.team) - t).max(0.0));
                m.alt_ammo = p.seat.and_then(|(vid, seat)| {
                    let v = self.vehicle(vid)?;
                    let spec = v.kind.spec();
                    let gi = spec.seats.get(seat as usize)?.alt_gun?;
                    Some((v.guns.get(gi)?.ammo, v.gun_spec(gi).ammo_max))
                });
                m.seat_ammo = p.seat.and_then(|(vid, seat)| {
                    let v = self.vehicle(vid)?;
                    let spec = v.kind.spec();
                    let gi = spec.seats.get(seat as usize)?.gun?;
                    Some((v.guns.get(gi)?.ammo, v.gun_spec(gi).ammo_max))
                });
                m
            }),
            fx,
            ..Default::default()
        }
    }

    /// Where a player looks from (for interest checks): its eyes.
    pub fn viewer_pos(&self, id: PlayerId) -> Option<Vec3> {
        self.player(id).map(Player::eye)
    }

    /// The bases the players of `team` may spawn at here.
    pub fn spawnable(&self, team: Team) -> Vec<BaseIdx> {
        self.bases
            .iter()
            .enumerate()
            .filter(|(_, b)| b.owner == Some(team))
            .map(|(i, _)| i as BaseIdx)
            .collect()
    }

    pub fn base_kind(&self, b: BaseIdx) -> BaseKind {
        self.map.def.bases[b as usize].kind
    }
}

#[derive(Clone, Copy, Debug)]
enum HitOn {
    World(Vec3, bool),
    Generator(usize),
    Player(usize, bool),
    Vehicle(usize),
    Turret(usize),
    Deployable(usize),
    Structure(usize),
    Burst,
}

/// A homing missile's velocity turned (at `weapons::HOMING_TURN`) towards `to`, its speed
/// kept.
pub fn home(vel: Vec3, to: Vec3, dt: f32) -> Vec3 {
    let speed = vel.length();
    let (a, b) = (vel.normalize_or(Vec3::NEG_Z), to.normalize_or(Vec3::NEG_Z));
    let angle = a.angle_between(b);
    if angle < 1e-4 {
        return vel;
    }
    let k = (weapons::HOMING_TURN * dt / angle).min(1.0);
    a.slerp(b, k).normalize_or(a) * speed
}

/// Testing: `PS_CHEAT=1` lets vehicles be ordered from anywhere, without a cooldown.
pub fn cheats() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("PS_CHEAT").is_ok_and(|v| v == "1"))
}

fn b_of(a: Ammo) -> crate::weapons::Ballistics {
    a.ballistics()
}

/// What a hit does to a turret or generator (percent).
fn b_vs_structure(b: &crate::weapons::Ballistics) -> f32 {
    b.vs_vehicle.max(b.damage * 0.08)
}

/// Where (the fraction along it) the segment `a`..`b` crosses the bubble of the gate at `gate`,
/// in or out.
fn bubble_crossing(gate: Vec3, a: Vec3, b: Vec3) -> Option<f32> {
    let (ia, ib) = (in_gate_bubble(gate, a), in_gate_bubble(gate, b));
    if ia == ib {
        return None;
    }
    // Bisect for the shell.
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..12 {
        let m = (lo + hi) / 2.0;
        if in_gate_bubble(gate, a.lerp(b, m)) == ia {
            lo = m;
        } else {
            hi = m;
        }
    }
    Some(lo)
}

/// `dir` turned randomly within a cone of half-angle `spread`.
pub fn jitter(dir: Vec3, spread: f32, rng: &mut Seq) -> Vec3 {
    if spread <= 0.0 {
        return dir;
    }
    let a = rng.range(0.0, std::f32::consts::TAU);
    let r = spread * rng.f().sqrt();
    let side = dir.any_orthonormal_vector();
    let up = dir.cross(side);
    (dir + (side * a.cos() + up * a.sin()) * r).normalize()
}

mod build;
pub mod kit;
pub mod spot;
pub use build::Structure;
pub use kit::Deployable;

#[cfg(test)]
mod tests;
