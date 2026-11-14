//! Bots: players the server runs on contested continents. Each picks a goal from the state of
//! the continent: stop a hack on one of its faction's bases, hack a base its faction may take,
//! hold a base its faction is hacking, or push towards the front. It walks there (through the
//! base's doors to the console), fights whoever it sees on the way, and works the console's
//! puzzle when it gets there.
//!
//! Bots go about in squads of four that share their leader's goal. They find their way over
//! the land with the continent's nav grid (`crate::nav`), take tanks, AAA and VTOLs from the
//! bases' pads, crew each other's guns (a heavy tank waits a moment for its gunner), go for
//! aircraft first in AAA, fire rockets at armour on foot and mend what's broken with repair
//! guns.
//!
//! In a base they follow its ways: in to the console (one works it, the rest of the squad
//! guards the way up), out of a tower's floors, its bunker or a spawn room. They keep a little
//! apart, and making no headway they look for the way again, then back off, then give up on
//! where they were going.
//!
//! They fire only with a chance of a hit: in the gun's reach (the sidearm where only that
//! reaches), a clear line of fire, the aim on the target within its size and the gun's spread
//! (rockets only at armour). Out of reach they close in, or, hurt and outgunned, take cover.
//! Clumsier bots fire at worse odds, aim worse and react slower.

use std::sync::Arc;

use glam::{EulerRot, Vec2, Vec3};

use crate::base::USE_REACH;
use crate::events::{Request, SpawnPoint};
use crate::nav::{GROUND_VEHICLE, INFANTRY, Nav, Route};
use crate::noise::Seq;
use crate::player::{BODY_HIT_RADIUS, eye_height};
use crate::sim::{ContinentSim, Player};
use crate::types::{PlayerCmd, PlayerId, Team, look_dir, wrap_angle, yaw_of};
use crate::vehicle::VehicleKind;
use crate::weapons::{ArmorKind, GrenadeKind, Kit, Loadout, Weapon};
use crate::worldgen::BaseIdx;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Goal {
    /// Go to a base's console and work it (to hack it, or to stop a hack).
    Console(BaseIdx),
    /// Stay near a base (holding a hack, guarding).
    Hold(BaseIdx),
    /// Go and stand on a base's capture point (to take it, or to hold it).
    Point(BaseIdx, u8),
    /// Walk somewhere.
    Go(Vec3),
}

#[derive(Clone, Debug)]
struct Bot {
    id: PlayerId,
    goal: Option<Goal>,
    rethink: f32,
    /// Following a way through a base's building.
    walk: Option<Walk>,
    last_pos: Vec3,
    /// Headway: where it was when it last made some, how long ago, and how often it has been
    /// stuck lately (each time trying something else).
    anchor: Vec3,
    anchor_t: f32,
    stuck_n: u32,
    calm: f32,
    /// Whether the straight line to where it walks is open (where to, and when to look again).
    clear: Option<(Vec3, bool)>,
    clear_in: f32,
    stuck_for: f32,
    dodge: f32,
    dodge_dir: Vec2,
    target: Option<PlayerId>,
    seen_for: f32,
    look_for_target: f32,
    aim_err: Vec2,
    strafe: f32,
    poke_in: f32,
    jumps: u32,
    reloads: u32,
    /// Somewhere out of sight of who it's fighting (and when to look again).
    cover: Option<Vec3>,
    cover_in: f32,
    yaw: f32,
    pitch: f32,
    /// Driving: where to, and how long it has been stuck (reversing while negative).
    drive_to: Option<Vec3>,
    drive_stuck: f32,
    deploys: u32,
    /// The way over the land to where it's going.
    route: Route,
    /// Its squad's goal (the leader's), if it isn't the leader.
    squad_goal: Option<Goal>,
    /// Driving a vehicle with empty gun seats: waits this long for crew.
    wait_crew: f32,
    /// A VTOL pulling out of an attack run: how long still, and which way.
    pull_out: f32,
    pull_yaw: f32,
    /// Seconds a gunner has sat without a driver.
    no_driver: f32,
    /// A VTOL out of rockets (or hurt) on its way to a rearm pad.
    rearming: bool,
    /// On foot, going to crew this vehicle's gun.
    board: Option<crate::types::VehicleId>,
}

/// A way through a base (`Layout::to_console`, `from_spawn`, or out the way in): its points
/// and which is next.
#[derive(Clone, Debug)]
struct Walk {
    base: BaseIdx,
    pts: Vec<Vec3>,
    next: usize,
    /// The way in (to the console), not out.
    inward: bool,
}

impl Bot {
    fn new(id: PlayerId, aim: Vec2) -> Bot {
        Bot {
            id,
            goal: None,
            rethink: 0.0,
            walk: None,
            last_pos: Vec3::ZERO,
            anchor: Vec3::ZERO,
            anchor_t: 0.0,
            stuck_n: 0,
            calm: 0.0,
            clear: None,
            clear_in: 0.0,
            stuck_for: 0.0,
            dodge: 0.0,
            dodge_dir: Vec2::ZERO,
            target: None,
            seen_for: 0.0,
            look_for_target: 0.0,
            aim_err: aim,
            strafe: 1.0,
            poke_in: 0.0,
            jumps: 0,
            reloads: 0,
            cover: None,
            cover_in: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            drive_to: None,
            drive_stuck: 0.0,
            deploys: 0,
            route: Route::default(),
            squad_goal: None,
            wait_crew: 0.0,
            pull_out: 0.0,
            pull_yaw: 0.0,
            no_driver: 0.0,
            rearming: false,
            board: None,
        }
    }
}

/// Bots per squad.
const SQUAD: usize = 4;
/// How far apart bots of a side keep.
const SPACE: f32 = 1.4;

/// A squad a human leads here, with bots in it (told by the server): they follow its
/// waypoint, or the leader, and spawn on the leader.
#[derive(Clone, Debug, PartialEq)]
pub struct HumanSquad {
    pub leader: PlayerId,
    pub bots: Vec<PlayerId>,
    pub waypoint: Option<Vec3>,
}

/// The bots on one continent.
#[derive(Default)]
pub struct Bots {
    /// Human-led squads with bots in them.
    pub human_squads: Vec<HumanSquad>,
    bots: Vec<Bot>,
    nav: Option<Arc<Nav>>,
    /// 0 (clumsy) ..1 (sharp).
    pub skill: f32,
    rng: Option<Seq>,
}

const NAMES: [&str; 32] = [
    "Vex", "Nyx", "Kade", "Juno", "Rook", "Sable", "Onyx", "Wren", "Talon", "Ash", "Cinder",
    "Echo", "Flint", "Ghost", "Halo", "Iris", "Jinx", "Kilo", "Lark", "Mako", "Nova", "Orion",
    "Pike", "Quill", "Raze", "Sol", "Tesla", "Umbra", "Volt", "Wisp", "Xeno", "Zed",
];

pub fn bot_name(id: PlayerId) -> String {
    format!("{}-{}", NAMES[id as usize % NAMES.len()], id % 100)
}

impl Bots {
    pub fn new(skill: f32, seed: u64) -> Bots {
        Bots {
            human_squads: Vec::new(),
            bots: Vec::new(),
            nav: None,
            skill,
            rng: Some(Seq::new(seed, 0xb07)),
        }
    }

    fn rng(&mut self) -> &mut Seq {
        self.rng.get_or_insert_with(|| Seq::new(1, 0xb07))
    }

    pub fn ids(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.bots.iter().map(|b| b.id)
    }

    pub fn count(&self, sim: &ContinentSim, team: Team) -> usize {
        self.bots
            .iter()
            .filter(|b| sim.player(b.id).is_some_and(|p| p.team == team))
            .count()
    }

    /// Adds a bot with id `id` on `team`.
    pub fn add(&mut self, sim: &mut ContinentSim, id: PlayerId, team: Team) {
        sim.add_player(id, &bot_name(id), team, true);
        let roll = self.rng().f();
        let rifle = if roll < 0.7 {
            Weapon::Rifle
        } else if roll < 0.87 {
            Weapon::Scatter
        } else {
            Weapon::Lance
        };
        let roll = self.rng().f();
        let special = if roll < 0.32 {
            Weapon::Striker
        } else if roll < 0.45 {
            Weapon::Skyhook
        } else if roll < 0.75 {
            Weapon::Medgun
        } else {
            Weapon::RepairGun
        };
        // A few MAXes (one of the arms in place of the rifle) and infiltrators. (Testing:
        // PS_BOT_ARMOR=max|stealth makes them all so.)
        let roll = match std::env::var("PS_BOT_ARMOR").as_deref() {
            Ok("max") => 0.0,
            Ok("stealth") => 0.1,
            _ => self.rng().f(),
        };
        let (armor, rifle) = if roll < 0.07 {
            let arm = Weapon::MAX_ARMS[self.rng().int(0, 3) as usize];
            (ArmorKind::Max, arm)
        } else if roll < 0.14 {
            (ArmorKind::Stealth, rifle)
        } else {
            (ArmorKind::ALL[self.rng().int(0, 3) as usize], rifle)
        };
        // Some carry their faction's own rifle; a few another grenade than the frag.
        let rifle = if self.rng().chance(0.25) && Weapon::RIFLES.contains(&rifle) {
            Weapon::faction_rifle(team)
        } else {
            rifle
        };
        let grenade = GrenadeKind::ALL[if self.rng().chance(0.7) {
            0
        } else {
            self.rng().int(1, 4) as usize
        }];
        // Engineers carry a deployable.
        let roll = self.rng().f();
        let kit = if special != Weapon::RepairGun {
            Kit::None
        } else if roll < 0.5 {
            Kit::Spitfire
        } else if roll < 0.75 {
            Kit::Mines
        } else {
            Kit::AmmoPack
        };
        if let Some(p) = sim.player_mut(id) {
            p.loadout = Loadout {
                armor,
                rifle,
                sidearm: Weapon::Pistol,
                special,
                kit,
                grenade,
            };
            // Ready to spawn at once.
            p.dead = Some(crate::sim::RESPAWN_TIME);
        }
        let aim = Vec2::new(self.rng().range(-1.0, 1.0), self.rng().range(-1.0, 1.0));
        self.bots.push(Bot::new(id, aim));
    }

    /// Takes over a bot that arrived from another continent (through a gate).
    pub fn adopt(&mut self, id: PlayerId) {
        if self.bots.iter().any(|b| b.id == id) {
            return;
        }
        let aim = Vec2::new(self.rng().range(-1.0, 1.0), self.rng().range(-1.0, 1.0));
        self.bots.push(Bot::new(id, aim));
    }

    /// Takes a bot of `team` off the continent (making room): one that is dead if any.
    pub fn remove_one(&mut self, sim: &mut ContinentSim, team: Team) -> Option<PlayerId> {
        let pick = self
            .bots
            .iter()
            .enumerate()
            .filter(|(_, b)| sim.player(b.id).is_some_and(|p| p.team == team))
            .min_by_key(|(_, b)| sim.player(b.id).is_some_and(|p| p.alive()) as u8)
            .map(|(i, _)| i)?;
        let b = self.bots.remove(pick);
        sim.remove_player(b.id);
        Some(b.id)
    }

    pub fn remove_all(&mut self, sim: &mut ContinentSim) -> Vec<PlayerId> {
        let ids: Vec<PlayerId> = self.bots.drain(..).map(|b| b.id).collect();
        for &id in &ids {
            sim.remove_player(id);
        }
        ids
    }

    /// Every bot decides what to do this tick.
    pub fn think(&mut self, sim: &mut ContinentSim) {
        let dt = crate::sim::TICK;
        let skill = self.skill;
        let mut rng = self.rng.take().unwrap_or_else(|| Seq::new(1, 0xb07));
        let nav = self
            .nav
            .get_or_insert_with(|| Arc::new(Nav::build(&sim.map, &sim.map.world)))
            .clone();
        let mut bots = std::mem::take(&mut self.bots);
        bots.retain(|b| sim.player(b.id).is_some());
        let followers: Vec<PlayerId> = self
            .human_squads
            .iter()
            .flat_map(|s| s.bots.iter().copied())
            .collect();
        let mut squads = squads(&bots, sim, &followers);
        // Bots in a human's squad: its waypoint, else the leader (far from it: to it).
        for hs in &self.human_squads {
            let leader = sim.player(hs.leader).filter(|p| p.alive());
            let goal = hs
                .waypoint
                .map(Goal::Go)
                .or_else(|| leader.map(|l| Goal::Go(l.feet())));
            for &b in &hs.bots {
                squads.push((b, goal));
            }
        }
        crew(&bots, sim);
        // Everyone on foot (bots keep apart from their own side).
        let afoot: Vec<(PlayerId, Team, Vec3)> = sim
            .players
            .iter()
            .filter(|p| p.afoot())
            .map(|p| (p.id, p.team, p.feet()))
            .collect();
        for bot in &mut bots {
            let Some(me) = sim.player(bot.id).cloned() else {
                continue;
            };
            let lead = squads.iter().find(|s| s.0 == bot.id);
            bot.squad_goal = lead.and_then(|s| s.1);
            if !me.alive() {
                if sim.may_spawn(bot.id) && !sim.awaiting_revive(bot.id) {
                    // In a human's squad: on the leader if it can; else as ever.
                    if let Some(hs) = self.human_squads.iter().find(|s| s.bots.contains(&bot.id)) {
                        sim.request(bot.id, Request::Spawn(SpawnPoint::Squad(hs.leader)));
                    }
                    if !sim.player(bot.id).is_some_and(|p| p.alive()) {
                        spawn(bot, &me, sim, &mut rng);
                    }
                }
                continue;
            }
            if let Some((vid, seat)) = me.seat {
                // A missile coming at its aircraft: flares (if they're ready).
                if seat == 0
                    && sim.projectiles.iter().any(|p| p.homing == Some(vid))
                    && let Some(vi) = sim.vehicles.iter().position(|v| v.id == vid)
                {
                    sim.flare(vi);
                }
                let cmd = drive(bot, &me, vid, seat, sim, &nav, skill, dt, &mut rng);
                sim.set_cmd(bot.id, cmd);
                continue;
            }
            let cmd = act(bot, &me, sim, &nav, &afoot, skill, dt, &mut rng);
            sim.set_cmd(bot.id, cmd);
        }
        self.bots = bots;
        self.rng = Some(rng);
    }
}

/// Vehicles waiting at their base for a gunner: a bot of the faction waiting to spawn comes
/// in at that base and gets in.
fn crew(bots: &[Bot], sim: &mut ContinentSim) {
    let waiting: Vec<(crate::types::VehicleId, usize, BaseIdx, Team)> = bots
        .iter()
        .filter(|b| b.wait_crew > 0.0)
        .filter_map(|b| {
            let (vid, seat) = sim.player(b.id)?.seat?;
            let v = sim.vehicle(vid)?;
            let spec = v.kind.spec();
            let free = (1..v.seats.len())
                .find(|&s| v.seats[s].is_none() && spec.seats[s].gun.is_some())?;
            (seat == 0).then_some((vid, free, v.from_base?, v.team))
        })
        .collect();
    for (vid, seat, base, team) in waiting {
        let Some(id) = bots.iter().map(|b| b.id).find(|&id| {
            sim.player(id).is_some_and(|p| p.team == team && !p.alive())
                && sim.may_spawn(id)
                && !sim.awaiting_revive(id)
        }) else {
            continue;
        };
        sim.request(
            id,
            Request::Spawn(SpawnPoint::Base {
                continent: sim.id,
                base,
            }),
        );
        sim.bot_board(id, vid, seat);
    }
}

/// Squads: each faction's bots in fours (by id); the first one alive on foot leads. For each
/// bot that isn't a leader: its leader's goal.
fn squads(
    bots: &[Bot],
    sim: &ContinentSim,
    followers: &[PlayerId],
) -> Vec<(PlayerId, Option<Goal>)> {
    let mut out = Vec::new();
    for team in Team::ALL {
        // (Not those in a human's squad.)
        let mut ids: Vec<&Bot> = bots
            .iter()
            .filter(|b| !followers.contains(&b.id))
            .filter(|b| sim.player(b.id).is_some_and(|p| p.team == team))
            .collect();
        ids.sort_by_key(|b| b.id);
        for squad in ids.chunks(SQUAD) {
            let leader = squad
                .iter()
                .find(|b| sim.player(b.id).is_some_and(|p| p.alive() && p.afoot()));
            let Some(leader) = leader else { continue };
            for b in squad.iter().filter(|b| b.id != leader.id) {
                out.push((b.id, leader.goal));
            }
        }
    }
    out
}

fn spawn(bot: &mut Bot, me: &Player, sim: &mut ContinentSim, rng: &mut Seq) {
    bot.goal = None;
    bot.walk = None;
    bot.stuck_n = 0;
    bot.rethink = 0.0;
    bot.target = None;
    bot.route.clear();
    bot.wait_crew = 0.0;
    bot.pull_out = 0.0;
    bot.no_driver = 0.0;
    bot.rearming = false;
    bot.board = None;
    let owned = sim.spawnable(me.team);
    if !owned.is_empty() {
        // The owned base nearest the action: one being hacked, else one next to hackable ones.
        let mut best = owned[rng.int(0, owned.len() as i32) as usize];
        if let Some(&b) = owned
            .iter()
            .find(|&&b| sim.bases[b as usize].hack.is_some())
        {
            best = b;
        } else {
            // Near the squad's goal, else near what there is to take.
            let squad = match bot.squad_goal {
                Some(Goal::Console(b) | Goal::Hold(b) | Goal::Point(b, _)) => {
                    vec![sim.map.bases[b as usize].center]
                }
                Some(Goal::Go(p)) => vec![p],
                None => Vec::new(),
            };
            let targets: Vec<Vec3> = if squad.is_empty() {
                hack_targets(sim, me.team)
                    .iter()
                    .map(|&t| sim.map.bases[t as usize].center)
                    .collect()
            } else {
                squad
            };
            if !targets.is_empty() {
                best = *owned
                    .iter()
                    .min_by(|&&a, &&b| {
                        let da = near_dist(sim.map.bases[a as usize].center, &targets);
                        let db = near_dist(sim.map.bases[b as usize].center, &targets);
                        da.total_cmp(&db)
                    })
                    .unwrap();
            }
        }
        sim.request(
            bot.id,
            Request::Spawn(SpawnPoint::Base {
                continent: sim.id,
                base: best,
            }),
        );
        // Crew: an empty gun seat in a friendly vehicle by the base.
        let at = sim.map.bases[best as usize].center;
        if rng.chance(0.7)
            && let Some((vid, seat)) = sim
                .vehicles
                .iter()
                .filter(|v| v.team == me.team && v.driver().is_some() && !v.flying())
                .filter(|v| (v.motion.pos - at).length() < 150.0)
                .find_map(|v| {
                    let spec = v.kind.spec();
                    (1..v.seats.len())
                        .find(|&s| v.seats[s].is_none() && spec.seats[s].gun.is_some())
                        .map(|s| (v.id, s))
                })
        {
            sim.bot_board(bot.id, vid, seat);
            return;
        }
        // A harvester when the faction's silos run low and nobody is on it.
        let low = sim.map.bases.iter().enumerate().any(|(b, l)| {
            l.silo.is_some()
                && sim.bases[b].owner == Some(me.team)
                && sim.bases[b].resources < crate::state::SILO_CAP * 0.5
        });
        let harvesting = sim
            .vehicles
            .iter()
            .any(|v| v.team == me.team && v.kind == VehicleKind::Harvester);
        if low && !harvesting && sim.map.bases[best as usize].silo.is_some() {
            sim.bot_vehicle(bot.id, VehicleKind::Harvester, Some(best as usize), None);
            return;
        }
        // Some take a vehicle from the base's pad (a few per faction at a time).
        let driving = bot_vehicles(sim, me.team);
        if driving < MAX_BOT_VEHICLES
            && rng.chance(0.3)
            && (sim.map.bases[best as usize].console.is_some()
                || !sim.map.bases[best as usize].points.is_empty())
        {
            // AAA more often when enemy aircraft are about.
            let air = sim
                .vehicles
                .iter()
                .any(|v| v.team != me.team && v.kind.aircraft());
            let roll = rng.f();
            let kind = if air && roll < 0.2 {
                VehicleKind::LightAaa
            } else if air && roll < 0.4 {
                VehicleKind::Interceptor
            } else if roll < 0.3 {
                VehicleKind::LightTank
            } else if roll < 0.5 {
                VehicleKind::HeavyTank
            } else if roll < 0.62 {
                VehicleKind::Rover
            } else if roll < 0.74 {
                VehicleKind::LightAaa
            } else if roll < 0.86 {
                VehicleKind::Vtol
            } else if roll < 0.93 {
                VehicleKind::Dropship
            } else {
                VehicleKind::Interceptor
            };
            if sim.bot_vehicle(bot.id, kind, Some(best as usize), None) {
                bot.drive_to = None;
                // Waits a little for a gunner.
                if kind.spec().seats.len() > 1 {
                    bot.wait_crew = 12.0;
                }
            }
        }
        return;
    }
    // No base here: at the faction's deployed spawn bus if there is one.
    if let Some(bus) = sim
        .vehicles
        .iter()
        .find(|v| v.team == me.team && v.kind == VehicleKind::SpawnBus && v.deployed)
        .map(|v| v.id)
    {
        sim.request(bot.id, Request::Spawn(SpawnPoint::Bus(bus)));
        if sim.player(bot.id).is_some_and(|p| p.alive()) {
            return;
        }
    }
    // No base here: in through a gate from a continent the faction holds.
    let gates: Vec<u8> = sim
        .gate_owners
        .iter()
        .enumerate()
        .filter(|(_, o)| **o == Some(me.team))
        .map(|(i, _)| i as u8)
        .collect();
    if let Some(&g) = gates.get(rng.int(0, gates.len().max(1) as i32) as usize) {
        let (at, yaw) = sim.arrival(g, false);
        let jitter = Vec3::new(rng.range(-6.0, 6.0), 0.0, rng.range(-6.0, 6.0));
        let mut at = at + jitter;
        at.y = sim.world.ground(at.x, at.z, at.y + 5.0).unwrap_or(at.y);
        sim.spawn_at(bot.id, at, yaw);
        // No bus of the faction's here: this one brings one forward.
        let has_bus = sim
            .vehicles
            .iter()
            .any(|v| v.team == me.team && v.kind == VehicleKind::SpawnBus);
        if !has_bus {
            let (at, yaw) = sim.arrival(g, true);
            sim.bot_vehicle(bot.id, VehicleKind::SpawnBus, None, Some((at, yaw)));
            bot.drive_to = None;
        }
    }
}

/// Steers vehicle motion `m` towards `dest` (stopping within `arrive`); backs out when stuck.
fn steer(
    bot: &mut Bot,
    m: &crate::vehicle::Motion,
    dest: Vec3,
    arrive: f32,
    dt: f32,
    cmd: &mut PlayerCmd,
) -> bool {
    let to = Vec2::new(dest.x - m.pos.x, dest.z - m.pos.z);
    if to.length() < arrive {
        return true;
    }
    let want = yaw_of(Vec3::new(to.x, 0.0, to.y));
    let err = wrap_angle(want - m.yaw);
    if bot.drive_stuck < 0.0 {
        bot.drive_stuck += dt;
        cmd.drive.throttle = -1.0;
        cmd.drive.steer = err.signum();
        return false;
    }
    cmd.drive.steer = (-err * 1.5).clamp(-1.0, 1.0);
    cmd.drive.throttle = if err.abs() < 0.7 { 1.0 } else { 0.3 };
    if m.vel.length() < 1.0 {
        bot.drive_stuck += dt;
        if bot.drive_stuck > 3.0 {
            bot.drive_stuck = -2.0;
        }
    } else {
        bot.drive_stuck = 0.0;
    }
    false
}

/// A bot in a harvester: to a node, mine it full, to a silo, unload, again.
fn harvest(
    bot: &mut Bot,
    me: &Player,
    v: &crate::sim::Vehicle,
    sim: &ContinentSim,
    nav: &Nav,
    dt: f32,
) -> PlayerCmd {
    let mut cmd = PlayerCmd::default();
    let m = v.motion;
    let Some((dest, node)) = harvest_target(sim, me.team, m.pos, v.cargo) else {
        return cmd;
    };
    // Far: over the land by the nav grid; near: straight there.
    let far = Vec2::new(dest.x - m.pos.x, dest.z - m.pos.z).length() > 45.0;
    if far {
        let way = bot.route.toward(nav, m.pos, dest, GROUND_VEHICLE, 14.0, dt);
        steer(bot, &m, way, 0.0, dt, &mut cmd);
        return cmd;
    }
    if node {
        if steer(bot, &m, dest, 18.0, dt, &mut cmd) {
            let eye = m.to_world(v.kind.spec().guns[0].pivot);
            let d = (dest + Vec3::Y * 1.5 - eye).normalize_or(Vec3::NEG_Z);
            cmd.yaw = yaw_of(d);
            cmd.pitch = d.y.clamp(-1.0, 1.0).asin();
            cmd.fire = true;
        }
    } else {
        // Parked at the silo it unloads by itself.
        steer(bot, &m, dest, 9.0, dt, &mut cmd);
    }
    cmd
}

/// Bot-driven vehicles of a faction on the continent.
fn bot_vehicles(sim: &ContinentSim, team: Team) -> usize {
    sim.vehicles
        .iter()
        .filter(|v| {
            v.team == team
                && v.kind != VehicleKind::SpawnBus
                && v.driver()
                    .and_then(|d| sim.player(d))
                    .is_some_and(|p| p.bot)
        })
        .count()
}

const MAX_BOT_VEHICLES: usize = 4;

/// Where a vehicle of `team` heads: a bus to a staging point short of the base its faction
/// may hack nearest it; armour towards the nearest base to fight over.
fn vehicle_target(sim: &ContinentSim, team: Team, from: Vec3, bus: bool) -> Option<Vec3> {
    let targets: Vec<usize> = (0..sim.bases.len())
        .filter(|&b| {
            let r = &sim.bases[b];
            r.hackable[team.index()]
                || (r.owner != Some(team) && r.hack.is_some())
                || (r.owner == Some(team) && r.hack.is_some())
        })
        .collect();
    let nearest = |list: &[usize]| {
        list.iter().copied().min_by(|&a, &b| {
            (sim.map.bases[a].center - from)
                .length()
                .total_cmp(&(sim.map.bases[b].center - from).length())
        })
    };
    if let Some(b) = nearest(&targets) {
        let c = sim.map.bases[b].center;
        let back = (from - c) * Vec3::new(1.0, 0.0, 1.0);
        let off: f32 = if bus { 230.0 } else { 110.0 };
        return Some(c + back.normalize_or(Vec3::X) * off.min(back.length()));
    }
    // Nothing to take here: on through the gate to where there is (into its core).
    if let Some(g) = sim
        .gate_fronts
        .iter()
        .enumerate()
        .filter(|(_, f)| f[team.index()])
        .map(|(g, _)| sim.map.gates[g])
        .min_by(|a, b| (*a - from).length().total_cmp(&(*b - from).length()))
    {
        return Some(g);
    }
    let enemy: Vec<usize> = (0..sim.bases.len())
        .filter(|&b| sim.bases[b].owner.is_some_and(|o| o != team))
        .collect();
    if let Some(b) = nearest(&enemy) {
        return Some(sim.map.bases[b].center);
    }
    // All ours: wherever the enemy is (come in through a gate).
    sim.players
        .iter()
        .filter(|p| p.alive() && p.team != team)
        .map(|p| p.feet())
        .chain(
            sim.vehicles
                .iter()
                .filter(|v| v.team != team)
                .map(|v| v.motion.pos),
        )
        .min_by(|a, b| (*a - from).length().total_cmp(&(*b - from).length()))
}

/// A harvester's next stop: a node to mine (beside it), or, full, its faction's silo most in
/// need.
fn harvest_target(sim: &ContinentSim, team: Team, from: Vec3, cargo: f32) -> Option<(Vec3, bool)> {
    if cargo < crate::sim::CARGO * 0.95 {
        return sim
            .nodes
            .iter()
            .filter(|n| n.amount > 10.0)
            .min_by(|a, b| (a.pos - from).length().total_cmp(&(b.pos - from).length()))
            .map(|n| (n.pos, true));
    }
    sim.map
        .bases
        .iter()
        .enumerate()
        .filter(|(b, l)| l.silo.is_some() && sim.bases[*b].owner == Some(team))
        .min_by(|a, b| {
            sim.bases[a.0]
                .resources
                .total_cmp(&sim.bases[b.0].resources)
        })
        .and_then(|(_, l)| l.silo)
        .map(|s| (s, false))
}

/// What a gun goes for in the air.
#[derive(Clone, Copy, PartialEq)]
enum Air {
    /// Aircraft first (AAA).
    First,
    /// Aircraft as well as the rest.
    Also,
    /// Not aircraft (tank cannons can't lift that high).
    Never,
}

/// The nearest enemy (vehicle or infantry) in range and in sight of `eye`: where and how fast
/// it goes, and whether it flies.
fn pick_enemy(
    sim: &ContinentSim,
    team: Team,
    eye: Vec3,
    range: f32,
    air: Air,
) -> Option<(Vec3, Vec3, bool)> {
    let seen = |p: Vec3| sim.world.ray(eye, p).is_none() && !sim.smoked(eye, p);
    let near = |list: Vec<(Vec3, Vec3, bool)>| {
        list.into_iter()
            .filter(|(p, _, _)| (*p - eye).length() < range)
            .filter(|(p, _, _)| seen(*p))
            .min_by(|a, b| (a.0 - eye).length().total_cmp(&(b.0 - eye).length()))
    };
    let vehicles = || {
        sim.vehicles
            .iter()
            .filter(|o| o.team != team)
            .map(|o| (o.motion.pos + Vec3::Y * 1.2, o.motion.vel, o.flying()))
    };
    if air == Air::First
        && let Some(a) = near(vehicles().filter(|v| v.2).collect())
    {
        return Some(a);
    }
    let infantry = sim
        .players
        .iter()
        .filter(|p| p.afoot() && p.alive() && p.team != team && !p.hidden_from(eye))
        .map(|p| {
            (
                p.feet() + Vec3::Y,
                Vec3::new(p.body.vel.x, 0.0, p.body.vel.y),
                false,
            )
        });
    near(
        vehicles()
            .filter(|v| air != Air::Never || !v.2)
            .chain(infantry)
            .collect(),
    )
}

/// Aims `cmd` at a moving target from `eye` with a gun firing `ammo` (leading it as well as
/// the bot can), and fires once the gun (pointing `gun_dir`) is on it.
#[allow(clippy::too_many_arguments)]
fn aim_gun(
    bot: &Bot,
    cmd: &mut PlayerCmd,
    eye: Vec3,
    at: Vec3,
    vel: Vec3,
    ammo: crate::weapons::Ammo,
    gun_dir: Vec3,
    skill: f32,
) {
    let b = ammo.ballistics();
    let t = (at - eye).length() / b.speed.max(1.0);
    let lead = at + vel * t * skill + Vec3::Y * (0.5 * b.gravity * t * t);
    let dir = (lead - eye).normalize_or(Vec3::NEG_Z);
    let err = (1.0 - skill) * 0.03;
    cmd.yaw = yaw_of(dir) + bot.aim_err.x * err;
    cmd.pitch = dir.y.clamp(-1.0, 1.0).asin() + bot.aim_err.y * err;
    cmd.fire = gun_dir.dot(look_dir(cmd.yaw, cmd.pitch)) > 0.995;
}

/// Where gun `g` of vehicle `v` points.
fn gun_dir(v: &crate::sim::Vehicle, g: usize) -> Vec3 {
    let spec = v.kind.spec();
    if spec.guns[g].fixed {
        return v.motion.forward();
    }
    let gun = v.guns[g];
    v.motion.rot
        * (glam::Quat::from_rotation_y(gun.yaw) * glam::Quat::from_rotation_x(gun.pitch))
        * Vec3::NEG_Z
}

/// A bot in a vehicle: drives it where it's needed (the bus deploys there), flies it, or
/// mans its gun, and fights from it whoever comes in range.
#[allow(clippy::too_many_arguments)]
fn drive(
    bot: &mut Bot,
    me: &Player,
    vid: crate::types::VehicleId,
    seat: u8,
    sim: &mut ContinentSim,
    nav: &Nav,
    skill: f32,
    dt: f32,
    rng: &mut Seq,
) -> PlayerCmd {
    let mut cmd = PlayerCmd {
        jumps: bot.jumps,
        deploys: bot.deploys,
        ..Default::default()
    };
    let Some(v) = sim.vehicle(vid).cloned() else {
        return cmd;
    };
    let m = v.motion;
    let spec = v.kind.spec();
    let bus = v.kind == VehicleKind::SpawnBus;
    if seat != 0 {
        // Crew: man the gun while there's a driver; out once there isn't (or at a deployed bus).
        if v.driver().is_none() || (bus && v.deployed) {
            bot.no_driver += dt;
            if bot.no_driver > 4.0 || (bus && v.deployed) {
                sim.request(bot.id, Request::ExitVehicle);
            }
            return cmd;
        }
        bot.no_driver = 0.0;
        if let Some(g) = spec.seats[seat as usize].gun {
            let gs = v.gun_spec(g);
            let eye = m.to_world(gs.pivot);
            let air = if gs.pitch_max > 0.8 {
                Air::Also
            } else {
                Air::Never
            };
            if let Some((at, vel, _)) = pick_enemy(sim, me.team, eye, 300.0, air) {
                aim_gun(bot, &mut cmd, eye, at, vel, gs.ammo, gun_dir(&v, g), skill);
            } else {
                cmd.yaw = yaw_of(m.forward());
            }
        }
        return cmd;
    }
    if bus && v.deployed {
        // Its job done: out, on foot.
        sim.request(bot.id, Request::ExitVehicle);
        return cmd;
    }
    if v.kind == VehicleKind::Harvester {
        return harvest(bot, me, &v, sim, nav, dt);
    }
    if bot.drive_to.is_none() || rng.chance(0.002) {
        bot.drive_to = vehicle_target(sim, me.team, m.pos, bus);
    }
    if v.kind.aircraft() {
        // A dropship waits on the pad a little for its gunners.
        if bot.wait_crew > 0.0 && m.grounded {
            bot.wait_crew -= dt;
            let empty =
                (1..v.seats.len()).any(|s| v.seats[s].is_none() && spec.seats[s].gun.is_some());
            if empty {
                return cmd;
            }
        }
        bot.wait_crew = 0.0;
        // Aircraft move on quickly: a fresh look every few seconds, and a patrol over a base of
        // ours when there's nowhere to be.
        if rng.chance(0.005) || bot.drive_to.is_none() {
            bot.drive_to = vehicle_target(sim, me.team, m.pos, false).or_else(|| {
                let ours: Vec<Vec3> = (0..sim.bases.len())
                    .filter(|&b| sim.bases[b].owner == Some(me.team))
                    .map(|b| sim.map.bases[b].center)
                    .collect();
                ours.get(rng.int(0, ours.len().max(1) as i32) as usize)
                    .copied()
            });
        }
        return fly(bot, me, &v, sim, skill, dt);
    }
    // Fight: the nearest enemy in sight (AAA: aircraft first).
    let enemy = if bus || spec.guns.is_empty() {
        None
    } else {
        let gs = v.gun_spec(0);
        let eye = m.to_world(gs.pivot);
        let (range, air) = if v.kind == VehicleKind::LightAaa {
            (450.0, Air::First)
        } else {
            (260.0, Air::Never)
        };
        pick_enemy(sim, me.team, eye, range, air).inspect(|&(at, vel, _)| {
            aim_gun(bot, &mut cmd, eye, at, vel, gs.ammo, gun_dir(&v, 0), skill);
        })
    };
    if enemy.is_none() {
        cmd.yaw = yaw_of(m.forward());
        cmd.pitch = 0.0;
    }
    // Waiting for crew a moment (the gunner seats empty).
    if bot.wait_crew > 0.0 {
        bot.wait_crew -= dt;
        let empty = (1..v.seats.len()).any(|s| v.seats[s].is_none() && spec.seats[s].gun.is_some());
        if empty {
            return cmd;
        }
        bot.wait_crew = 0.0;
    }
    // Out of shells or badly hurt: back to a rearm pad of ours, and wait there till mended.
    let dry = v.guns.first().is_some_and(|g| g.ammo == 0) || v.health < 30.0;
    if dry && !bus {
        bot.rearming = true;
    }
    let full = v
        .guns
        .iter()
        .enumerate()
        .all(|(gi, g)| g.ammo >= v.gun_spec(gi).ammo_max)
        && v.health >= 90.0;
    if bot.rearming && full {
        bot.rearming = false;
    }
    if bot.rearming
        && let Some(pad) = sim
            .map
            .bases
            .iter()
            .enumerate()
            .filter(|(b, _)| sim.bases[*b].owner == Some(me.team))
            .flat_map(|(_, l)| l.rearm.iter().copied())
            .min_by(|a, b| (*a - m.pos).length().total_cmp(&(*b - m.pos).length()))
    {
        let d = Vec2::new(pad.x - m.pos.x, pad.z - m.pos.z).length();
        if d > 4.0 {
            let way = if d > 45.0 {
                bot.route.toward(nav, m.pos, pad, GROUND_VEHICLE, 14.0, dt)
            } else {
                pad
            };
            steer(bot, &m, way, 0.0, dt, &mut cmd);
        }
        return cmd;
    }
    // Drive.
    let Some(dest) = bot.drive_to else { return cmd };
    let dist = Vec2::new(dest.x - m.pos.x, dest.z - m.pos.z).length();
    let arrive = if bus { 25.0 } else { 40.0 };
    if dist < arrive {
        if bus {
            bot.deploys += 1;
            cmd.deploys = bot.deploys;
        }
        return cmd;
    }
    // Stopped short of the fight when there's someone to shoot (AAA keeps its distance from
    // aircraft by holding still too).
    // (A rover's gun is its gunner's: without one it drives on.)
    let armed = spec.seats[0].gun.is_some()
        || (1..v.seats.len()).any(|s| v.seats[s].is_some() && spec.seats[s].gun.is_some());
    if enemy.is_some() && armed && !bus && dist < 260.0 {
        return cmd;
    }
    let way = bot.route.toward(nav, m.pos, dest, GROUND_VEHICLE, 14.0, dt);
    steer(bot, &m, way, 0.0, dt, &mut cmd);
    cmd
}

/// A bot flying a VTOL: up to a safe height over the land ahead, over to where the fighting
/// is, rocket runs on what it finds there (diving at it, pulling out before it's too close),
/// circling when there's nothing.
fn fly(
    bot: &mut Bot,
    me: &Player,
    v: &crate::sim::Vehicle,
    sim: &ContinentSim,
    skill: f32,
    dt: f32,
) -> PlayerCmd {
    let mut cmd = PlayerCmd {
        deploys: bot.deploys,
        ..Default::default()
    };
    let m = v.motion;
    let spec = v.kind.spec();
    let (yaw, pitch, _) = m.rot.to_euler(EulerRot::YXZ);
    let flat = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let t = &sim.map.terrain;
    let ground = [0.0, 60.0, 140.0, 240.0]
        .iter()
        .map(|d| {
            let p = m.pos + flat * *d;
            t.surface(p.x, p.z)
        })
        .fold(f32::MIN, f32::max);
    let gs = v.gun_spec(0);
    let nose = m.to_world(gs.pivot);
    let mut want_alt = ground + 60.0;
    let mut want_speed: f32 = 38.0;
    let mut want_yaw = yaw;
    let mut want_pitch = 0.0;
    // Out of rockets or hurt: down on a rearm pad of ours until full again.
    let ammo = v.guns.first().map_or(0, |g| g.ammo);
    if ammo == 0 || v.health < 35.0 {
        bot.rearming = true;
    }
    if bot.rearming && ammo >= gs.ammo_max && v.health >= 90.0 {
        bot.rearming = false;
    }
    let pad = bot
        .rearming
        .then(|| {
            sim.map
                .bases
                .iter()
                .enumerate()
                .filter(|(b, _)| sim.bases[*b].owner == Some(me.team))
                .flat_map(|(_, l)| l.rearm.iter().copied())
                .min_by(|a, b| (*a - m.pos).length().total_cmp(&(*b - m.pos).length()))
        })
        .flatten();
    if let Some(pad) = pad {
        let to = Vec2::new(pad.x - m.pos.x, pad.z - m.pos.z);
        let d = to.length();
        if m.grounded && d < crate::base::REARM_RADIUS {
            // Down: wait.
            cmd.drive.throttle = -1.0;
            return cmd;
        }
        let mut lift;
        let mut want_speed = (d * 0.4).clamp(0.0, 38.0);
        let mut want_yaw = yaw_of(Vec3::new(to.x, 0.0, to.y));
        if d < 6.0 {
            // Over it: straight down, gently.
            want_speed = 0.0;
            want_yaw = yaw;
            // (Aiming a little under it: it has to settle to count as down.)
            lift = ((pad.y - 1.5 - m.pos.y) * 0.15).clamp(-0.5, 0.3);
        } else {
            let alt = if d > 120.0 {
                ground + 50.0
            } else {
                (pad.y + 12.0 + d * 0.3).max(t.surface(m.pos.x, m.pos.z) + 10.0)
            };
            lift = ((alt - m.pos.y) * 0.15).clamp(-1.0, 1.0);
        }
        if m.vel.y < -5.0 {
            lift = lift.max(0.5);
        }
        cmd.drive.turn = Vec2::new(
            (wrap_angle(want_yaw - yaw) * 0.08).clamp(-0.035, 0.035),
            ((0.0 - pitch) * 0.1).clamp(-0.03, 0.03),
        );
        cmd.drive.lift = lift;
        cmd.drive.throttle = ((want_speed - m.throttle) * 0.3).clamp(-1.0, 1.0);
        cmd.yaw = yaw;
        cmd.pitch = pitch;
        return cmd;
    }
    // A dropship has no forward gun: its gunners fight, it flies on and circles. An interceptor
    // hunts aircraft first.
    let enemy = match v.kind {
        VehicleKind::Dropship => None,
        VehicleKind::Interceptor => pick_enemy(sim, me.team, nose, 600.0, Air::First),
        _ => pick_enemy(sim, me.team, nose, 500.0, Air::Also),
    };
    let fast = v.kind == VehicleKind::Interceptor;
    if fast {
        want_speed = 60.0;
    }
    // Missiles once locked.
    if spec.seats[0].alt_gun.is_some() {
        cmd.alt = me.lock.is_some_and(|l| l.1 >= crate::weapons::LOCK_TIME);
    }
    if bot.pull_out > 0.0 {
        // Pulling out: on and up, then round for another pass.
        bot.pull_out -= dt;
        want_yaw = bot.pull_yaw;
        want_alt = ground + 80.0;
        want_speed = 45.0;
        want_pitch = 0.2;
    } else if let Some((at, vel, _)) = enemy {
        let d = (at - nose).length();
        if d < 80.0 {
            bot.pull_out = 4.0;
            bot.pull_yaw = yaw + 0.4;
        } else {
            let b = gs.ammo.ballistics();
            let tt = d / b.speed.max(1.0);
            let lead = at + vel * tt * skill + Vec3::Y * (0.5 * b.gravity * tt * tt);
            let dir = (lead - nose).normalize_or(Vec3::NEG_Z);
            want_yaw = yaw_of(dir);
            want_pitch = dir.y.clamp(-0.8, 0.4).asin();
            want_speed = if fast { 45.0 } else { 24.0 };
            // Holds its height in the dive (above the ground at least).
            want_alt = m.pos.y.max(ground + 25.0);
            let err = (1.0 - skill) * 0.02;
            cmd.fire = m.forward().dot(dir) > 0.993 - err && d < 380.0;
        }
    } else if let Some(dest) = bot.drive_to {
        let to = dest - m.pos;
        let d = Vec2::new(to.x, to.z).length();
        want_yaw = yaw_of(Vec3::new(to.x, 0.0, to.z));
        if d < 150.0 {
            // There: circle.
            want_yaw += 1.2;
            want_speed = 28.0;
        }
    }
    let above = m.pos.y - t.surface(m.pos.x, m.pos.z);
    if m.grounded || above < 12.0 {
        // Off the pad straight up first.
        want_speed = 0.0;
        want_pitch = 0.0;
        want_alt = want_alt.max(m.pos.y + 20.0);
    }
    // Climbing by the nose too when the land ahead rises (lift alone is slow).
    if bot.pull_out <= 0.0 && enemy.is_none() {
        want_pitch = ((want_alt - m.pos.y) / 120.0).clamp(-0.25, 0.5);
        if want_alt - m.pos.y > 40.0 {
            want_speed = want_speed.min(25.0);
        }
    }
    cmd.drive.turn = Vec2::new(
        (wrap_angle(want_yaw - yaw) * 0.08).clamp(-0.035, 0.035),
        ((want_pitch - pitch) * 0.1).clamp(-0.03, 0.03),
    );
    cmd.drive.lift = ((want_alt - m.pos.y) * 0.15).clamp(-1.0, 1.0);
    cmd.drive.throttle = ((want_speed - m.throttle) * 0.3).clamp(-1.0, 1.0);
    cmd.yaw = yaw;
    cmd.pitch = pitch;
    cmd
}

/// Something of `team`'s near `pos` that wants mending (a vehicle, a turret, a generator):
/// where to aim.
fn to_repair(sim: &ContinentSim, team: Team, pos: Vec3) -> Option<Vec3> {
    const REACH: f32 = 30.0;
    let vehicles = sim
        .vehicles
        .iter()
        .filter(|v| v.team == team && v.health < 85.0)
        .map(|v| v.motion.pos + Vec3::Y * 1.0);
    let turrets = sim
        .turrets
        .iter()
        .filter(|t| sim.bases[t.base as usize].owner == Some(team) && t.health < 90.0)
        .map(|t| t.pos + Vec3::Y * crate::base::TURRET_PIVOT);
    let generators = sim
        .map
        .bases
        .iter()
        .enumerate()
        .filter(|(b, _)| sim.bases[*b].owner == Some(team) && sim.bases[*b].generator < 95.0)
        .filter_map(|(_, l)| l.generator);
    let eye = pos + Vec3::Y * eye_height(false);
    vehicles
        .chain(turrets)
        .chain(generators)
        .filter(|p| (*p - pos).length() < REACH)
        // In sight (up to the thing itself).
        .filter(|p| {
            let d = (*p - eye).length();
            sim.world.ray(eye, *p).is_none_or(|h| h.0 * d > d - 3.5)
        })
        .min_by(|a, b| (*a - pos).length().total_cmp(&(*b - pos).length()))
}

fn near_dist(p: Vec3, list: &[Vec3]) -> f32 {
    list.iter()
        .map(|q| (*q - p).length())
        .fold(f32::MAX, f32::min)
}

/// Bases `team` may hack here.
fn hack_targets(sim: &ContinentSim, team: Team) -> Vec<BaseIdx> {
    sim.bases
        .iter()
        .enumerate()
        .filter(|(_, b)| b.hackable[team.index()] && b.hack.is_none_or(|h| h.team != team))
        .filter(|(i, _)| {
            sim.map.bases[*i].console.is_some() || !sim.map.bases[*i].points.is_empty()
        })
        .map(|(i, _)| i as BaseIdx)
        .collect()
}

/// Which of base `b`'s capture points a bot of `team` at `pos` goes for: one its faction
/// doesn't hold (the nearest; every third bot the next), else one being taken from it, else
/// the nearest.
fn pick_point(sim: &ContinentSim, b: BaseIdx, team: Team, pos: Vec3, id: PlayerId) -> u8 {
    let rt = &sim.bases[b as usize];
    let at = &sim.map.bases[b as usize].points;
    let mut cand: Vec<usize> = (0..rt.points.len())
        .filter(|&i| rt.points[i].owner != Some(team))
        .collect();
    if cand.is_empty() {
        cand = (0..rt.points.len())
            .filter(|&i| rt.points[i].by.is_some_and(|t| t != team))
            .collect();
    }
    if cand.is_empty() {
        cand = (0..rt.points.len()).collect();
    }
    cand.sort_by(|&a, &c| {
        (at[a].pos - pos)
            .length()
            .total_cmp(&(at[c].pos - pos).length())
    });
    let k = if cand.len() > 1 && id % 3 == 2 { 1 } else { 0 };
    cand.get(k).map_or(0, |&i| i as u8)
}

/// Going to take (or save) base `b`: its console, or one of its capture points.
fn take(sim: &ContinentSim, b: BaseIdx, me: &Player) -> Goal {
    if sim.bases[b as usize].points.is_empty() {
        Goal::Console(b)
    } else {
        Goal::Point(b, pick_point(sim, b, me.team, me.feet(), me.id))
    }
}

/// Whether one of `team`'s capture point bases has a point held or being taken by another.
fn points_threatened(rt: &crate::sim::BaseRt, team: Team) -> bool {
    rt.points
        .iter()
        .any(|p| p.owner != Some(team) || p.by.is_some_and(|t| t != team))
}

fn choose_goal(bot: &Bot, me: &Player, sim: &ContinentSim, rng: &mut Seq) -> Option<Goal> {
    let team = me.team;
    let pos = me.feet();
    let dist = |b: BaseIdx| (sim.map.bases[b as usize].center - pos).length();
    // Our bases under hack: most go and stop it.
    let hacked: Vec<BaseIdx> = sim
        .bases
        .iter()
        .enumerate()
        .filter(|(_, b)| b.owner == Some(team) && (b.hack.is_some() || points_threatened(b, team)))
        .map(|(i, _)| i as BaseIdx)
        .collect();
    if let Some(&b) = hacked.iter().min_by(|a, b| dist(**a).total_cmp(&dist(**b)))
        && rng.chance(0.65)
    {
        return Some(take(sim, b, me));
    }
    // Bases we are hacking: some hold them.
    let ours: Vec<BaseIdx> = sim
        .bases
        .iter()
        .enumerate()
        .filter(|(_, b)| b.hack.is_some_and(|h| h.team == team))
        .map(|(i, _)| i as BaseIdx)
        .collect();
    if let Some(&b) = ours.iter().min_by(|a, b| dist(**a).total_cmp(&dist(**b)))
        && rng.chance(0.45)
    {
        return Some(Goal::Hold(b));
    }
    // Repairers see to a broken generator of ours.
    if me.loadout.special == Weapon::RepairGun
        && let Some(g) = sim
            .map
            .bases
            .iter()
            .enumerate()
            .filter(|(b, _)| sim.bases[*b].owner == Some(team) && sim.bases[*b].generator < 60.0)
            .filter_map(|(_, l)| l.generator)
            .min_by(|a, b| (*a - pos).length().total_cmp(&(*b - pos).length()))
        && rng.chance(0.6)
    {
        return Some(Goal::Go(g));
    }
    let targets = hack_targets(sim, team);
    // Towers first: a foothold to spawn at.
    let towers: Vec<BaseIdx> = targets
        .iter()
        .copied()
        .filter(|&b| sim.map.bases[b as usize].kind == crate::worldgen::BaseKind::Tower)
        .collect();
    if let Some(&b) = towers.iter().min_by(|a, b| dist(**a).total_cmp(&dist(**b)))
        && (rng.chance(0.75) || towers.len() == targets.len())
    {
        return Some(take(sim, b, me));
    }
    if !targets.is_empty() {
        // Mostly the nearest, sometimes another.
        let pick = if rng.chance(0.7) {
            *targets
                .iter()
                .min_by(|a, b| dist(**a).total_cmp(&dist(**b)))
                .unwrap()
        } else {
            targets[rng.int(0, targets.len() as i32) as usize]
        };
        return Some(take(sim, pick, me));
    }
    let _ = bot;
    // Nothing to take: roam towards an enemy base or guard one of ours.
    let any: Vec<BaseIdx> = (0..sim.bases.len() as BaseIdx)
        .filter(|&b| sim.bases[b as usize].owner != Some(team))
        .collect();
    if let Some(&b) = any.get(rng.int(0, any.len().max(1) as i32) as usize) {
        return Some(Goal::Go(sim.map.bases[b as usize].center));
    }
    let n = sim.bases.len() as i32;
    (n > 0).then(|| Goal::Hold(rng.int(0, n) as BaseIdx))
}

#[allow(clippy::too_many_arguments)]
fn act(
    bot: &mut Bot,
    me: &Player,
    sim: &mut ContinentSim,
    nav: &Nav,
    afoot: &[(PlayerId, Team, Vec3)],
    skill: f32,
    dt: f32,
    rng: &mut Seq,
) -> PlayerCmd {
    let pos = me.feet();
    let eye = me.eye();
    bot.rethink -= dt;
    if bot.goal.is_none() || bot.rethink <= 0.0 {
        // Mostly with the squad.
        let g = match bot.squad_goal {
            Some(sg) if rng.chance(0.85) => Some(sg),
            _ => choose_goal(bot, me, sim, rng),
        };
        if g != bot.goal {
            bot.walk = None;
        }
        bot.goal = g;
        bot.rethink = rng.range(15.0, 30.0);
    }
    // Stale goals: a hack ended, a base changed hands.
    if let Some(Goal::Console(b)) = bot.goal {
        let rt = &sim.bases[b as usize];
        let ours = rt.owner == Some(me.team);
        if (ours && rt.hack.is_none()) || (!ours && rt.hack.is_some_and(|h| h.team == me.team)) {
            bot.goal = Some(Goal::Hold(b));
            bot.rethink = rng.range(8.0, 20.0);
        } else if !ours && !rt.hackable[me.team.index()] {
            bot.goal = None;
        }
    }
    if let Some(Goal::Point(b, i)) = bot.goal {
        let rt = &sim.bases[b as usize];
        let ours = rt.owner == Some(me.team);
        let held = rt.points.get(i as usize).is_none_or(|p| {
            p.owner == Some(me.team) && !p.contested && p.by.is_none_or(|t| t == me.team)
        });
        if !ours && !rt.hackable[me.team.index()] {
            bot.goal = None;
        } else if ours && rt.hack.is_none() && !points_threatened(rt, me.team) {
            bot.goal = Some(Goal::Hold(b));
            bot.rethink = rng.range(8.0, 20.0);
        } else if held {
            // Ours: on to another (or staying to guard it).
            let j = pick_point(sim, b, me.team, pos, me.id);
            if j != i {
                bot.goal = Some(Goal::Point(b, j));
                bot.walk = None;
            }
        }
    }
    // A MAX can't work a console: it holds the base instead.
    if me.loadout.max()
        && let Some(Goal::Console(b)) = bot.goal
    {
        bot.goal = Some(Goal::Hold(b));
    }
    // (A MAX's arms are its only gun.)
    let special = (!me.loadout.max()).then_some(me.loadout.special);
    // Combat: look for someone to shoot now and then (a little beyond its guns' reach: it
    // closes in).
    bot.look_for_target -= dt;
    // Blinded by a flash: nobody to see.
    if me.flashed > 0.0 {
        bot.target = None;
    } else if bot.look_for_target <= 0.0 {
        bot.look_for_target = 0.3;
        let range = (reach(me.loadout.rifle).max(reach(me.loadout.sidearm)) * 1.4).min(300.0);
        let prev = bot.target;
        bot.target = sim
            .players
            .iter()
            .filter(|p| p.afoot() && p.team != me.team && p.protect <= 0.0)
            .filter(|p| !p.hidden_from(eye))
            .map(|p| (p.id, (p.eye() - eye).length(), p.feet() + Vec3::Y * 1.2))
            .filter(|(_, d, _)| *d < range)
            .filter(|(_, _, c)| sim.world.ray(eye, *c).is_none() && !sim.smoked(eye, *c))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _, _)| id);
        if bot.target != prev {
            bot.seen_for = 0.0;
            bot.aim_err = Vec2::new(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0));
        }
    }
    let mut cmd = PlayerCmd {
        jumps: bot.jumps,
        reloads: bot.reloads,
        deploys: bot.deploys,
        slot: 0,
        ..Default::default()
    };
    // Medics heal a hurt team mate in front of them now and then.
    let mut aim_at: Option<Vec3> = None;
    let mut repair_at: Option<Vec3> = None;
    let target = bot
        .target
        .and_then(|t| sim.player(t))
        .filter(|p| p.afoot())
        .map(|p| {
            (
                p.feet(),
                Vec3::new(p.body.vel.x, p.body.vy, p.body.vel.y),
                p.cmd.crouch,
            )
        });
    // Fighting a body: how far, and how far the gun in hand and the rifle reach. Rockets at
    // armour: its size.
    let mut fight: Option<(f32, f32, f32)> = None;
    let mut rocket_at: Option<f32> = None;
    // Something not a body to shoot at (a structure, a deployable, a turret): its size.
    let mut object_at: Option<f32> = None;
    // An infiltrator's hack: what, and where to stand.
    let mut hack: Option<(crate::events::HackTarget, Vec3, f32)> = None;
    let mut skyhook = false;
    if let Some((feet, vel, crouch)) = target {
        bot.seen_for += dt;
        let chest = feet + Vec3::Y * (eye_height(crouch) - 0.45);
        let d = (chest - eye).length();
        cmd.slot = pick_slot(me, d);
        let w = me.guns[cmd.slot as usize].weapon;
        let lead = chest + vel * (d / w.stats().speed) * skill;
        aim_at = Some(lead);
        fight = Some((d, reach(w), reach(me.loadout.rifle)));
        // Strafe while fighting.
        if rng.chance(0.01) {
            bot.strafe = -bot.strafe;
        }
    } else if me.loadout.rifle == Weapon::Mauler
        && let Some((at, vel, size)) = armour_near(sim, me.team, eye)
    {
        // A MAX's cannons at armour.
        let s = Weapon::Mauler.stats();
        let t = (at - eye).length() / s.speed;
        aim_at = Some(at + vel * t * skill + Vec3::Y * (0.5 * s.gravity * t * t));
        rocket_at = Some(size);
    } else if me.loadout.rifle == Weapon::Burster
        && let Some((at, vel, _)) = pick_enemy(sim, me.team, eye, 450.0, Air::First).filter(|e| e.2)
    {
        // A MAX's flak at aircraft.
        let s = Weapon::Burster.stats();
        let t = (at - eye).length() / s.speed;
        aim_at = Some(at + vel * t * skill + Vec3::Y * (0.5 * s.gravity * t * t));
        rocket_at = Some(4.0);
    } else if special == Some(Weapon::Medgun)
        && let Some(body) = sim
            .players
            .iter()
            .filter(|p| p.team == me.team && p.revivable())
            .map(|p| p.feet())
            .filter(|f| (*f - pos).length() < 30.0)
            .min_by(|a, b| (*a - pos).length().total_cmp(&(*b - pos).length()))
    {
        // Medics revive the downed near them: up to the body, the beam on it.
        cmd.slot = 2;
        aim_at = Some(body + Vec3::Y * 0.3);
        repair_at = Some(body);
    } else if special == Some(Weapon::Striker)
        && let Some((at, vel, size)) = armour_near(sim, me.team, eye)
    {
        // Rockets at armour (not at bodies), allowing for their drop.
        let s = Weapon::Striker.stats();
        let t = (at - eye).length() / s.speed;
        cmd.slot = 2;
        aim_at = Some(at + vel * t * skill + Vec3::Y * (0.5 * s.gravity * t * t));
        rocket_at = Some(size);
    } else if special == Some(Weapon::Skyhook)
        && let Some(air) = sim
            .vehicles
            .iter()
            .filter(|v| v.team != me.team && v.flying() && v.flares <= 0.0)
            .map(|v| v.center())
            .filter(|c| (*c - eye).length() < crate::weapons::LOCK_RANGE_AIR)
            .filter(|c| {
                sim.world
                    .ray(eye, *c)
                    .is_none_or(|h| (h.1 - *c).length() < 6.0)
            })
            .min_by(|a, b| (*a - eye).length().total_cmp(&(*b - eye).length()))
    {
        // The Skyhook on an aircraft: held on it till it locks, then fired.
        cmd.slot = 2;
        aim_at = Some(air);
        skyhook = me.lock.is_some_and(|l| l.1 >= crate::weapons::LOCK_TIME);
    } else if special == Some(Weapon::RepairGun)
        && let Some(fix) = to_repair(sim, me.team, pos)
    {
        cmd.slot = 2;
        aim_at = Some(fix);
        repair_at = Some(fix);
    } else if special == Some(Weapon::Medgun)
        && let Some(hurt) = sim
            .players
            .iter()
            .filter(|p| p.afoot() && p.team == me.team && p.id != me.id && p.health < 70.0)
            .find(|p| (p.feet() - pos).length() < 14.0)
    {
        cmd.slot = 2;
        aim_at = Some(hurt.feet() + Vec3::Y * 1.0);
    } else if me.loadout.armor == ArmorKind::Stealth
        && let Some(h) = hack_near(sim, me.team, pos)
    {
        // An infiltrator: up to the enemy terminal or turret near, and hack it.
        hack = Some(h);
    } else if let Some((at, size)) = object_near(
        sim,
        me.team,
        eye,
        reach(me.loadout.rifle).clamp(30.0, 150.0),
    ) {
        // Nobody to fight: the enemy's structures, deployables and turrets (launchers on the
        // big ones).
        if special == Some(Weapon::Striker) && size >= 1.0 {
            let s = Weapon::Striker.stats();
            let t = (at - eye).length() / s.speed;
            cmd.slot = 2;
            aim_at = Some(at + Vec3::Y * (0.5 * s.gravity * t * t));
        } else {
            cmd.slot = pick_slot(me, (at - eye).length());
            aim_at = Some(at);
        }
        object_at = Some(size);
    }
    // Crew: a friendly vehicle near, waiting with its gun seat empty: go and get in.
    if target.is_none()
        && bot.board.is_none()
        && !me.loadout.max()
        && (sim.tick + bot.id as u64).is_multiple_of(60)
    {
        bot.board = sim
            .vehicles
            .iter()
            .filter(|v| v.team == me.team && v.driver().is_some() && !v.flying())
            .filter(|v| v.motion.vel.length() < 2.0 && (v.motion.pos - pos).length() < 60.0)
            .find(|v| {
                let spec = v.kind.spec();
                (1..v.seats.len()).any(|s| v.seats[s].is_none() && spec.seats[s].gun.is_some())
            })
            .map(|v| v.id)
            .filter(|_| rng.chance(0.8));
    }
    let boarding = bot.board.and_then(|vid| sim.vehicle(vid)).filter(|v| {
        let spec = v.kind.spec();
        v.motion.vel.length() < 3.0
            && (1..v.seats.len()).any(|s| v.seats[s].is_none() && spec.seats[s].gun.is_some())
    });
    if boarding.is_none() {
        bot.board = None;
    }
    let board_at = boarding.map(|v| v.motion.pos);
    if let Some(v) = boarding
        && crate::vehicle::hull_distance(&v.motion, v.kind.spec().half, eye) < 3.0
    {
        sim.request(bot.id, Request::EnterVehicle(v.id));
        bot.board = None;
    }
    // Where to go: up to what it mends, or the vehicle it crews; else its goal (far off: over
    // the land by the nav grid).
    let mut dest = destination(bot, me, sim, afoot);
    if let Some(at) = board_at {
        dest = Some(at);
    }
    if board_at.is_some() {
        // Straight to it.
    } else if let Some((_, at, reach)) = hack {
        dest = (Vec2::new(at.x - pos.x, at.z - pos.z).length() > reach * 0.6).then_some(at);
    } else if let Some(r) = repair_at {
        dest = (Vec2::new(r.x - pos.x, r.z - pos.z).length() > 5.0).then_some(r);
    } else if let Some(d) = dest
        && bot.walk.is_none()
    {
        // Straight there when it's near and nothing stands between; else by the nav grid.
        let far = Vec2::new(d.x - pos.x, d.z - pos.z).length() > 40.0;
        bot.clear_in -= dt;
        if !far && (bot.clear_in <= 0.0 || bot.clear.is_none_or(|(to, _)| (to - d).length() > 3.0))
        {
            bot.clear_in = 0.5;
            bot.clear = Some((d, walkable(sim, me.team, pos, d, 40.0)));
        }
        if far || bot.clear.is_some_and(|(_, open)| !open) {
            dest = Some(bot.route.toward(nav, pos, d, INFANTRY, 5.0, dt));
        }
    }
    let mut stick = Vec2::ZERO;
    if let Some(d) = dest {
        let to = Vec2::new(d.x - pos.x, d.z - pos.z);
        if to.length() > 0.8 {
            stick = to.normalize();
        }
    }
    // Headway: none for a few seconds while walking (not fighting): find the way again and
    // sidestep; then back off; then give up on where it was going.
    bot.calm += dt;
    if bot.calm > 12.0 {
        bot.stuck_n = 0;
    }
    if stick != Vec2::ZERO && target.is_none() {
        bot.anchor_t += dt;
        if Vec2::new(pos.x - bot.anchor.x, pos.z - bot.anchor.z).length() > 2.0
            || (pos.y - bot.anchor.y).abs() > 1.5
        {
            bot.anchor = pos;
            bot.anchor_t = 0.0;
        }
    } else {
        bot.anchor = pos;
        bot.anchor_t = 0.0;
    }
    if bot.anchor_t > 3.0 {
        bot.anchor_t = 0.0;
        bot.stuck_n += 1;
        bot.calm = 0.0;
        bot.walk = None;
        bot.route.clear();
        bot.clear = None;
        let side = Vec2::new(stick.y, -stick.x) * if rng.chance(0.5) { 1.0 } else { -1.0 };
        match bot.stuck_n {
            1 => {
                bot.dodge = 1.0;
                bot.dodge_dir = side;
                bot.jumps += 1;
            }
            2 => {
                bot.dodge = 1.5;
                bot.dodge_dir = (-stick + side * 0.5).normalize_or_zero();
            }
            _ => {
                bot.dodge = 2.0;
                bot.dodge_dir = (-stick + side).normalize_or_zero();
                bot.goal = None;
                bot.rethink = 0.0;
                bot.stuck_n = 0;
            }
        }
        cmd.jumps = bot.jumps;
    }
    // Pressed against something: dodge sideways a moment, hop.
    let moved = (pos - bot.last_pos).length();
    bot.last_pos = pos;
    if stick != Vec2::ZERO && moved < 0.02 {
        bot.stuck_for += dt;
    } else {
        bot.stuck_for = (bot.stuck_for - dt * 0.5).max(0.0);
    }
    if bot.stuck_for > 0.8 && bot.dodge <= 0.0 {
        bot.dodge = rng.range(0.6, 1.4);
        let a = rng.range(0.0, std::f32::consts::TAU);
        bot.dodge_dir = Vec2::new(a.cos(), a.sin());
        bot.jumps += 1;
        cmd.jumps = bot.jumps;
        bot.stuck_for = 0.0;
    }
    if bot.dodge > 0.0 {
        bot.dodge -= dt;
        stick = (stick * 0.3 + bot.dodge_dir).normalize_or_zero();
    }
    // Room: away from its own side's bodies it's crowding (the squad walks the same ways).
    let sep = afoot
        .iter()
        .filter(|(id, team, _)| *id != me.id && *team == me.team)
        .fold(Vec2::ZERO, |acc, &(id, _, q)| {
            let off = Vec2::new(pos.x - q.x, pos.z - q.z);
            let d = off.length();
            if d >= SPACE || (pos.y - q.y).abs() > 1.5 {
                return acc;
            }
            // (Right on top of each other: apart by id.)
            let away = if d > 0.05 {
                off / d
            } else {
                let a = (me.id as f32 - id as f32) * 2.4;
                Vec2::new(a.cos(), a.sin())
            };
            acc + away * (1.0 - d / SPACE)
        });
    if sep != Vec2::ZERO {
        stick = (stick + sep * 1.5).clamp_length_max(1.0);
    }
    let mut closing = false;
    if let (Some((d, wr, best)), Some((feet, ..))) = (fight, target) {
        let to = Vec2::new(feet.x - pos.x, feet.z - pos.z).normalize_or_zero();
        let side = Vec2::new(to.y, -to.x) * bot.strafe;
        // Can the enemy reach it where its own guns can't reach the enemy?
        let outgunned = sim
            .player(bot.target.unwrap_or(0))
            .is_some_and(|p| d < reach(p.weapon()));
        bot.cover_in -= dt;
        if d > wr && me.health < 50.0 && outgunned {
            // Hurt and outranged: into cover from it.
            if bot.cover_in <= 0.0 {
                bot.cover_in = 1.0;
                bot.cover = cover_from(sim, me.team, pos, feet + Vec3::Y * 1.5);
            }
            stick = match bot.cover {
                Some(c) if Vec2::new(c.x - pos.x, c.z - pos.z).length() > 0.8 => {
                    Vec2::new(c.x - pos.x, c.z - pos.z).normalize()
                }
                Some(_) => Vec2::ZERO,
                None => (-to + side).normalize_or_zero(),
            };
        } else if d > best * 0.9 {
            // Out of its rifle's reach: closer (no use spraying from here; the sidearm may do
            // meanwhile).
            stick = (to + side * 0.3).normalize_or_zero();
            closing = true;
        } else {
            // In reach: slower, strafing.
            let side = Vec2::new(stick.y, -stick.x) * bot.strafe;
            stick = (stick * 0.4 + side * 0.6).clamp_length_max(1.0);
        }
    } else {
        bot.cover = None;
    }
    cmd.stick = stick;
    cmd.sprint = (target.is_none() || closing) && stick != Vec2::ZERO;
    // Looking: at the target, else where it walks.
    let want = match aim_at {
        Some(a) => {
            let err = (1.0 - skill) * 0.06 * (1.0 - (bot.seen_for * 1.5).min(0.8));
            let d = (a - eye).normalize_or(Vec3::NEG_Z);
            let yaw = yaw_of(d) + bot.aim_err.x * err;
            let pitch = d.y.clamp(-1.0, 1.0).asin() + bot.aim_err.y * err;
            (yaw, pitch)
        }
        None if stick != Vec2::ZERO => (yaw_of(Vec3::new(stick.x, 0.0, stick.y)), 0.0),
        None => (bot.yaw, 0.0),
    };
    let turn = (4.0 + 6.0 * skill) * dt;
    bot.yaw = wrap_angle(bot.yaw + wrap_angle(want.0 - bot.yaw).clamp(-turn, turn));
    bot.pitch += (want.1 - bot.pitch).clamp(-turn, turn);
    cmd.yaw = bot.yaw;
    cmd.pitch = bot.pitch;
    if me.loadout.max() {
        cmd.slot = 0;
    }
    let gun = me.guns[(cmd.slot as usize).min(me.guns.len() - 1)];
    if let Some(a) = aim_at {
        let look = look_dir(cmd.yaw, cmd.pitch);
        let reaction = 0.5 - 0.35 * skill;
        let ready = target.is_none() || bot.seen_for > reaction;
        // Only with a chance of a hit: in reach, a clear line, the aim on it.
        let (least, judge) = discipline(skill);
        // Past close quarters, aiming down the sights (the lance through its scope).
        cmd.alt = target.is_some() && gun.weapon != Weapon::Scatter && (a - eye).length() > 15.0;
        let moving = me.body.vel.length() / crate::player::WALK_SPEED;
        let airborne = !me.body.on_ground();
        let chance = |w: Weapon, size: f32, reach: f32| {
            let spread = w.spread_for(cmd.alt, cmd.crouch, moving, airborne);
            hit_chance(&sim.world, w, spread, eye, look, a, size, reach, judge)
        };
        let shoot = if target.is_some() {
            chance(gun.weapon, BODY_HIT_RADIUS, reach(gun.weapon)) >= least
        } else if gun.weapon == Weapon::Skyhook {
            skyhook
        } else if let Some(size) = rocket_at {
            chance(gun.weapon, size, STRIKER_REACH) >= least
        } else if let Some(size) = object_at {
            chance(
                gun.weapon,
                size,
                reach(gun.weapon).max(STRIKER_REACH * (cmd.slot == 2) as u8 as f32),
            ) >= least
        } else {
            // Beams: on it.
            look.dot((a - eye).normalize_or(Vec3::NEG_Z)) > 0.985
        };
        if shoot && ready {
            // Semi-automatic weapons: pull and release.
            let auto = gun.weapon.stats().automatic;
            cmd.fire = auto || (sim.tick / 6).is_multiple_of(2);
        }
    } else if gun.reload <= 0.0
        && gun.reserve > 0
        && (gun.mag as f32) < gun.weapon.stats().mag as f32 * 0.4
    {
        // Nothing to shoot: reload.
        bot.reloads += 1;
        cmd.reloads = bot.reloads;
    }
    if gun.mag == 0 && gun.reserve > 0 && aim_at.is_some() {
        // (Pulling the trigger on an empty magazine reloads.)
        cmd.fire = true;
    }
    // Infiltrators cloak on the way (not fighting); firing drops it.
    if me.loadout.armor == ArmorKind::Stealth
        && target.is_none()
        && !me.cloaked
        && me.cloak > 0.7
        && stick != Vec2::ZERO
        && rng.chance(0.02)
    {
        bot.deploys += 1;
        cmd.deploys = bot.deploys;
    }
    // Engineers put their kit down at the base they're at (not fighting).
    if me.kit_left > 0
        && target.is_none()
        && rng.chance(0.004)
        && let Some(Goal::Console(b) | Goal::Hold(b) | Goal::Point(b, _)) = bot.goal
        && (sim.map.bases[b as usize].center - pos).length() < sim.map.bases[b as usize].half
    {
        bot.deploys += 1;
        cmd.deploys = bot.deploys;
    }
    // Hacking: there, standing still till it's done.
    if let Some((what, at, reach)) = hack
        && Vec2::new(at.x - pos.x, at.z - pos.z).length() <= reach * 0.6
    {
        cmd.stick = Vec2::ZERO;
        cmd.sprint = false;
        if me.hacking.is_none() {
            sim.request(bot.id, Request::Hack(what));
        }
    }
    // At a console: work its puzzle.
    if let Some(Goal::Console(b)) = bot.goal
        && let Some((at, _)) = sim.map.bases[b as usize].console
        && (at - pos).length() < USE_REACH
        && target.is_none()
    {
        bot.poke_in -= dt;
        cmd.stick = Vec2::ZERO;
        if bot.poke_in <= 0.0 {
            bot.poke_in = rng.range(0.6, 1.2) * (1.5 - skill);
            sim.bot_poke(bot.id, b);
        }
    }
    cmd
}

/// How far a bot reckons gun `w` hits a body: short of where its damage has fallen off, or
/// its shots die.
fn reach(w: Weapon) -> f32 {
    let s = w.stats();
    match s.shot {
        crate::weapons::ShotKind::Beam => s.near,
        crate::weapons::ShotKind::Bolt => (s.far * 0.85).min(s.speed * s.life * 0.9),
    }
}

/// How far bots fire rockets at armour.
const STRIKER_REACH: f32 = 200.0;

/// The least chance of a hit a bot of `skill` fires at, and how much of its aim's error it
/// notices (clumsy bots fire sooner, at worse odds).
fn discipline(skill: f32) -> (f32, f32) {
    (0.12 + 0.18 * skill, 0.4 + 0.6 * skill)
}

/// The chance that a pull of `w`'s trigger from `eye` along `aim` hits something of radius
/// `size` at `at`: none past `reach` or with something in the line of fire (walls, the land,
/// forcefields), else the share of its spread the target covers (any of its pellets). `judge`
/// scales the aim's error as the shooter sees it; `spread` is the gun's as fired now
/// (`Weapon::spread_for`: moving, crouching, in the air).
#[allow(clippy::too_many_arguments)]
fn hit_chance(
    world: &crate::collide::World,
    w: Weapon,
    spread: f32,
    eye: Vec3,
    aim: Vec3,
    at: Vec3,
    size: f32,
    reach: f32,
    judge: f32,
) -> f32 {
    let to = at - eye;
    let d = to.length();
    if d > reach {
        return 0.0;
    }
    if d < size {
        return 1.0;
    }
    let s = w.stats();
    let wide = (size / d).atan();
    let off = aim.angle_between(to) * judge;
    if off > wide + spread {
        return 0.0;
    }
    // The line of fire, as far as the target.
    if let Some((f, _, _)) = world.ray(eye, eye + aim * d)
        && f * d < d - size - 0.3
    {
        return 0.0;
    }
    let p = if spread <= 0.0 {
        1.0
    } else {
        let cover = (wide / spread).min(1.0).powi(2);
        let centred = if off <= wide {
            1.0
        } else {
            1.0 - (off - wide) / spread
        };
        (cover * centred).clamp(0.0, 1.0)
    };
    1.0 - (1.0 - p).powi(s.pellets.max(1) as i32)
}

/// The gun a bot fights a body `d` away with: its rifle, or its sidearm where only that
/// reaches (or the rifle is empty); a reload under way goes on.
fn pick_slot(me: &Player, d: f32) -> u8 {
    let (rifle, side) = (me.guns[0], me.guns[1]);
    let dry = |g: crate::sim::Gun| g.mag == 0 && g.reserve == 0;
    if dry(side) || (me.slot() == 0 && rifle.reload > 0.0) {
        return 0;
    }
    let only_side = d > reach(rifle.weapon) && d <= reach(side.weapon);
    u8::from(dry(rifle) || only_side)
}

/// The nearest enemy armour (on the ground) in rocket reach and in sight of `eye`: where, how
/// fast it goes, and its size.
fn armour_near(sim: &ContinentSim, team: Team, eye: Vec3) -> Option<(Vec3, Vec3, f32)> {
    sim.vehicles
        .iter()
        .filter(|v| v.team != team && !v.flying())
        .map(|v| {
            let h = v.kind.spec().half;
            (v.motion.pos + Vec3::Y * 1.2, v.motion.vel, h.x.max(h.z))
        })
        .filter(|(p, _, _)| (*p - eye).length() < STRIKER_REACH)
        .filter(|(p, _, size)| {
            let d = (*p - eye).length();
            // (Up to the hull itself.)
            sim.world
                .ray_team(eye, *p, Some(team))
                .is_none_or(|h| h.0 * d > d - size - 0.5)
        })
        .min_by(|a, b| (a.0 - eye).length().total_cmp(&(b.0 - eye).length()))
}

/// A spot a few steps away, about level, that it can walk to and `threat` can't see.
fn cover_from(sim: &ContinentSim, team: Team, pos: Vec3, threat: Vec3) -> Option<Vec3> {
    let mut best: Option<(Vec3, f32)> = None;
    for k in 0..12 {
        let a = k as f32 / 12.0 * std::f32::consts::TAU;
        for r in [3.0, 6.0, 10.0] {
            let p = pos + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
            let Some(y) = sim.world.ground(p.x, p.z, pos.y + 1.5) else {
                continue;
            };
            let p = Vec3::new(p.x, y, p.z);
            if (y - pos.y).abs() > 1.2 || !walkable(sim, team, pos, p, 20.0) {
                break;
            }
            if sim.world.ray(threat, p + Vec3::Y * 1.2).is_some() {
                if best.is_none_or(|b| r < b.1) {
                    best = Some((p, r));
                }
                break;
            }
        }
    }
    best.map(|b| b.0)
}

/// Whether a body could walk straight from `a` to `b` (within `reach`): nothing in the way at
/// knee height (forcefields of its own side let it through).
fn walkable(sim: &ContinentSim, team: Team, a: Vec3, b: Vec3, reach: f32) -> bool {
    let up = Vec3::Y * 0.7;
    Vec2::new(b.x - a.x, b.z - a.z).length() < reach
        && sim.world.ray_team(a + up, b + up, Some(team)).is_none()
}

/// Points of `pts` about level with `pos` (not the floor above or the bunker below), nearest
/// first: index and how far.
fn level_with(pos: Vec3, pts: &[Vec3]) -> Vec<(usize, f32)> {
    let mut near: Vec<(usize, f32)> = pts
        .iter()
        .enumerate()
        .filter(|(_, p)| (p.y - pos.y).abs() < 3.0)
        .map(|(i, p)| {
            let flat = Vec2::new(p.x - pos.x, p.z - pos.z).length();
            (i, flat + 3.0 * (p.y - pos.y).abs())
        })
        .collect();
    near.sort_by(|a, b| a.1.total_cmp(&b.1));
    near
}

/// The point of `pts` nearest `pos` about level with it that it can walk straight to.
fn nearest_on(sim: &ContinentSim, team: Team, pos: Vec3, pts: &[Vec3]) -> Option<(usize, f32)> {
    level_with(pos, pts)
        .into_iter()
        .find(|&(i, _)| walkable(sim, team, pos, pts[i], 60.0))
}

/// The way base `b`'s bots go in for `goal`: to its console, or to a capture point.
fn goal_path(sim: &ContinentSim, b: BaseIdx, goal: Option<Goal>) -> &[Vec3] {
    let l = &sim.map.bases[b as usize];
    match goal {
        Some(Goal::Point(g, i)) if g == b => l.points.get(i as usize).map_or(&[], |p| &p.path),
        _ => &l.to_console,
    }
}

/// The way into base `b` from where it is (for the bot's goal): along its path to the console
/// (or the capture point) from the nearest point it can walk to.
fn way_in(
    sim: &ContinentSim,
    team: Team,
    b: BaseIdx,
    pos: Vec3,
    goal: Option<Goal>,
) -> Option<Walk> {
    let pts = goal_path(sim, b, goal);
    let (next, _) = nearest_on(sim, team, pos, pts)?;
    Some(Walk {
        base: b,
        pts: pts.to_vec(),
        next,
        inward: true,
    })
}

/// The way out of base `b`'s building (a tower's upper floors, its bunker, a spawn room): back
/// along the way in, or along the way out of the spawn room, from the nearest point it can
/// walk to (seeing none, the nearest about level).
fn way_out(sim: &ContinentSim, team: Team, b: BaseIdx, pos: Vec3) -> Option<Walk> {
    let l = &sim.map.bases[b as usize];
    let back: Vec<Vec3> = l.to_console.iter().rev().copied().collect();
    let seqs = [back, l.from_spawn.clone()];
    let mut best: Option<(usize, usize, f32)> = None;
    for seeing in [true, false] {
        for (s, pts) in seqs.iter().enumerate() {
            let near = if seeing {
                nearest_on(sim, team, pos, pts)
            } else {
                level_with(pos, pts).first().copied()
            };
            if let Some((i, c)) = near
                && best.is_none_or(|b| c < b.2)
            {
                best = Some((s, i, c));
            }
        }
        if best.is_some() {
            break;
        }
    }
    let (s, next, _) = best?;
    Some(Walk {
        base: b,
        pts: seqs[s].clone(),
        next,
        inward: false,
    })
}

/// How far along base `b`'s way in the bot goes: to the console, or (holding the base, or a
/// team mate already at the console) to a point short of it, guarding the way.
fn walk_stop(
    bot: &Bot,
    me: &Player,
    sim: &ContinentSim,
    afoot: &[(PlayerId, Team, Vec3)],
    b: BaseIdx,
) -> usize {
    let l = &sim.map.bases[b as usize];
    if let Some(Goal::Point(..)) = bot.goal {
        return goal_path(sim, b, bot.goal).len().saturating_sub(1);
    }
    let n = l.to_console.len();
    let Some((at, _)) = l.console else {
        return n.saturating_sub(1);
    };
    match bot.goal {
        Some(Goal::Console(g)) if g == b => {
            let taken = afoot
                .iter()
                .any(|&(id, t, q)| id != me.id && t == me.team && (q - at).length() < USE_REACH);
            if taken && (me.feet() - at).length() > USE_REACH {
                n.saturating_sub(2 + bot.id as usize % 3)
            } else {
                n - 1
            }
        }
        _ => n.saturating_sub(1 + bot.id as usize % 4),
    }
}

/// Where the bot walks now: along a base's way in when going to its console or holding it,
/// out of a building it has nothing to do in, else to its goal.
fn destination(
    bot: &mut Bot,
    me: &Player,
    sim: &ContinentSim,
    afoot: &[(PlayerId, Team, Vec3)],
) -> Option<Vec3> {
    let pos = me.feet();
    let team = me.team;
    let flat = |p: Vec3| Vec2::new(p.x - pos.x, p.z - pos.z).length();
    let goal_base = match bot.goal {
        Some(Goal::Console(b) | Goal::Hold(b)) if sim.map.bases[b as usize].console.is_some() => {
            Some(b)
        }
        Some(Goal::Point(b, _)) => Some(b),
        _ => None,
    };
    if bot.walk.as_ref().is_some_and(|w| {
        w.inward
            && (Some(w.base) != goal_base
                || w.pts.last() != goal_path(sim, w.base, bot.goal).last())
    }) {
        bot.walk = None;
    }
    // In a building it has nothing to do in: out (of a tower's floors, a bunker, a spawn room),
    // unless it's passing through the ground floor straight to where it's going.
    let goal_at = match bot.goal {
        Some(Goal::Console(b) | Goal::Hold(b)) => {
            sim.map.bases[b as usize].to_console.first().copied()
        }
        Some(Goal::Point(b, _)) => goal_path(sim, b, bot.goal).first().copied(),
        Some(Goal::Go(p)) => Some(p),
        None => None,
    };
    if bot.walk.is_none()
        && let Some(b) = sim.map.bases.iter().position(|l| l.in_building(pos))
        && goal_base != Some(b as BaseIdx)
        && ((pos.y - sim.map.bases[b].center.y).abs() > 1.5
            || goal_at.is_none_or(|g| !walkable(sim, team, pos, g, 40.0)))
    {
        bot.walk = way_out(sim, team, b as BaseIdx, pos);
    }
    if let Some(w) = &mut bot.walk
        && let Some(&p) = w.pts.get(w.next)
        && flat(p) < 1.2
        && (p.y - pos.y).abs() < 2.5
    {
        w.next += 1;
        // On the way out of where it's going: in as soon as it can.
        if !w.inward
            && goal_base == Some(w.base)
            && let Some(inw) = way_in(sim, team, w.base, pos, bot.goal)
        {
            *w = inw;
        }
    }
    if let Some(w) = &bot.walk {
        if w.inward && w.next > walk_stop(bot, me, sim, afoot, w.base) {
            // There: on guard (at a capture point, on its own spot in it).
            if let Some(Goal::Point(..)) = bot.goal {
                bot.walk = None;
            } else {
                return None;
            }
        }
    }
    if let Some(w) = &bot.walk {
        if let Some(&p) = w.pts.get(w.next) {
            return Some(p);
        }
        // Out (or in): the way over the land from here, not from where it was planned.
        bot.walk = None;
        bot.route.clear();
        bot.clear = None;
    }
    match bot.goal? {
        Goal::Point(b, i) => {
            let layout = &sim.map.bases[b as usize];
            let pt = layout.points.get(i as usize)?;
            // Its own spot in the zone (a little off the others').
            let a = bot.id as f32 * 2.4;
            let r = if layout.in_building(pt.pos) { 1.2 } else { 2.0 };
            let spot = pt.stand + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
            let level = (pos.y - pt.pos.y).abs() < 1.5;
            let zone = Vec2::new(pt.pos.x - pos.x, pt.pos.z - pos.z).length() < pt.radius;
            if level && flat(spot) < 1.0 {
                return None;
            }
            if level && zone {
                return Some(spot);
            }
            let path = &pt.path;
            let near = layout.in_building(pos)
                || layout.inside(pos)
                || walkable(sim, team, pos, path[0], 30.0);
            if near {
                bot.walk = way_in(sim, team, b, pos, bot.goal);
                if bot.walk.is_none() && layout.in_building(pos) {
                    bot.walk = way_out(sim, team, b, pos);
                }
                if let Some(w) = &bot.walk {
                    return w.pts.get(w.next).copied();
                }
            }
            Some(path[0])
        }
        Goal::Console(b) | Goal::Hold(b) if goal_base == Some(b) => {
            let layout = &sim.map.bases[b as usize];
            if let Some(Goal::Console(_)) = bot.goal
                && let Some((at, _)) = layout.console
                && (at - pos).length() < USE_REACH * 0.8
            {
                return None;
            }
            let door = layout.to_console[0];
            let near = layout.in_building(pos)
                || layout.inside(pos)
                || walkable(sim, team, pos, door, 30.0);
            if near {
                bot.walk = way_in(sim, team, b, pos, bot.goal);
                if bot.walk.is_none() && layout.in_building(pos) {
                    bot.walk = way_out(sim, team, b, pos);
                }
                if let Some(w) = &bot.walk {
                    return w.pts.get(w.next).copied();
                }
            }
            Some(door)
        }
        Goal::Console(_) => None,
        Goal::Hold(b) => {
            let layout = &sim.map.bases[b as usize];
            let c = layout.center;
            if (c - pos).length() < layout.half * 0.8 {
                None
            } else {
                Some(c + Vec3::new(0.0, 0.0, -layout.half * 0.5))
            }
        }
        Goal::Go(p) => ((p - pos).length() > 30.0).then_some(p),
    }
}

impl ContinentSim {
    /// A bot at base `b`'s console pokes the piece that gets it nearer to solving it.
    pub fn bot_poke(&mut self, id: PlayerId, b: BaseIdx) {
        let Some(p) = self.player(id) else { return };
        let team = p.team;
        let rt = &self.bases[b as usize];
        let ours = rt.owner == Some(team);
        if (ours && rt.hack.is_none())
            || (!ours && !rt.hackable[team.index()])
            || (!ours && rt.hack.is_some_and(|h| h.team == team))
        {
            return;
        }
        let Some(puzzle) = rt.puzzle.clone() else {
            return;
        };
        if let Some(piece) = puzzle.hint(&rt.at) {
            self.poke_piece(b as usize, piece, id, team);
        }
    }
}

/// How well bots play.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard];

    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Normal => "normal",
            Difficulty::Hard => "hard",
        }
    }

    pub fn skill(self) -> f32 {
        match self {
            Difficulty::Easy => 0.2,
            Difficulty::Normal => 0.5,
            Difficulty::Hard => 0.85,
        }
    }

    pub fn parse(s: &str) -> Option<Difficulty> {
        Difficulty::ALL.into_iter().find(|d| d.name() == s)
    }
}

#[cfg(test)]
mod tests;

/// The nearest enemy structure, deployable or (standing) turret in sight from `eye` within
/// `range`: where to aim, and how big it is.
fn object_near(sim: &ContinentSim, team: Team, eye: Vec3, range: f32) -> Option<(Vec3, f32)> {
    let structures = sim.structures.iter().filter(|s| s.team != team).map(|s| {
        let (h, height) = s.kind.size();
        (s.pos + Vec3::Y * height * 0.5, h.x.min(h.y).max(0.6))
    });
    let deployables = sim
        .deployables
        .iter()
        .filter(|d| d.team != team && d.health > 0.0)
        .map(|d| (d.center(), d.radius()));
    let turrets = (0..sim.turrets.len())
        .filter(|&ti| {
            sim.turrets[ti].health > 0.0 && sim.turret_team(ti).is_some_and(|t| t != team)
        })
        .map(|ti| {
            (
                sim.turrets[ti].pos + Vec3::Y * crate::base::TURRET_PIVOT,
                1.2,
            )
        });
    structures
        .chain(deployables)
        .chain(turrets)
        .filter(|(c, _)| (*c - eye).length() < range)
        // (In sight: the line may end on the thing's own shell.)
        .filter(|(c, size)| {
            sim.world
                .ray(eye, *c)
                .is_none_or(|h| (h.1 - *c).length() < size + 1.5)
                && !sim.smoked(eye, *c)
        })
        .min_by(|a, b| (a.0 - eye).length().total_cmp(&(b.0 - eye).length()))
}

/// An infiltrator's hack: the nearest enemy terminal or standing turret within 40 m of `pos`
/// (not its faction's already): what, where to stand, and the reach.
fn hack_near(
    sim: &ContinentSim,
    team: Team,
    pos: Vec3,
) -> Option<(crate::events::HackTarget, Vec3, f32)> {
    use crate::events::HackTarget;
    let terminals = sim.map.bases.iter().enumerate().flat_map(|(b, l)| {
        l.terminals.iter().enumerate().filter_map(move |(i, t)| {
            let owner = sim.terminal_team(b, i)?;
            (owner != team).then_some((
                HackTarget::Terminal {
                    base: b as BaseIdx,
                    index: i as u8,
                },
                t.pos,
                USE_REACH,
            ))
        })
    });
    let turrets = (0..sim.turrets.len()).filter_map(|ti| {
        let t = &sim.turrets[ti];
        let owner = sim.turret_team(ti)?;
        (owner != team && t.health > 0.0).then_some((
            HackTarget::Turret(ti as u16),
            // (Behind it, on the wall walk.)
            t.pos + glam::Quat::from_rotation_y(t.yaw) * Vec3::Z * 2.0,
            crate::sim::kit::TURRET_REACH,
        ))
    });
    terminals
        .chain(turrets)
        .filter(|(_, at, _)| (*at - pos).length() < 40.0)
        .min_by(|a, b| (a.1 - pos).length().total_cmp(&(b.1 - pos).length()))
}
