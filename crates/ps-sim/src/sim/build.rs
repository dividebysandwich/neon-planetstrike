//! Base benefits and construction.
//!
//! Benefits: a faction holding an outpost of a facility kind (`ContinentDef::facility`) with
//! its generator up has that facility's benefit on the continent (a sanctuary's owner has them
//! all there): a Tech Plant lets it order the advanced vehicles, a Bio Lab halves its respawn
//! time, an Amp Station halves the damage its wall turrets take and quickens their fire, an
//! Interlink shows its players the enemies near its bases (`flags::SPOTTED`).
//!
//! Construction: at its faction's harvester (`TERMINAL_REACH`), a player pulls a structure
//! (`StructureKind`) from the construction terminal, the harvester's load paying, and carries
//! it until it puts it down (`BUILD_REACH` away at most) on open ground out of the bases
//! (`NO_BUILD`). Each structure draws its upkeep from the nearest mobile silo of its faction
//! within `SILO_RANGE` that isn't empty; with none (a silo: empty) it decays (`DECAY`) until it
//! falls. Harvesters unload into mobile silos as into base silos. Walls block, Bastions shoot at
//! enemies, tubes are spawn points (one near a silo). Their faction's engineers repair them.

use glam::Vec3;

use super::{ContinentSim, Target, jitter};
use crate::construct::{
    BASTION_AIR_RANGE, BASTION_INTERVAL, BASTION_PIVOT, BASTION_RANGE, BUILD_REACH, DECAY,
    NO_BUILD, SILO_HOLDS, SILO_LOW, SILO_RANGE, SILO_START, SavedStructure, StructureKind,
    TERMINAL_REACH, TUBE_SPAWN,
};
use crate::events::{Audience, Deny, SimEvent};
use crate::noise::Seq;
use crate::progress::{self, Award};
use crate::snapshot::{Fx, ImpactOn, StructSnap};
use crate::types::{PlayerId, Team, look_dir, wrap_angle, yaw_of};
use crate::weapons::Ammo;
use crate::worldgen::{BaseKind, Facility};

/// Interlink: enemies this near one of the faction's bases are spotted.
pub const INTERLINK_RANGE: f32 = 250.0;

#[derive(Clone, Debug)]
pub struct Structure {
    pub id: u32,
    pub kind: StructureKind,
    pub team: Team,
    /// Who built it (a Bastion's kills are its).
    pub owner: PlayerId,
    pub pos: Vec3,
    pub yaw: f32,
    pub health: f32,
    pub cooldown: f32,
    pub think: f32,
    pub target: Option<Target>,
    pub aim_yaw: f32,
    pub aim_pitch: f32,
    pub firing: bool,
    /// A silo's resources.
    pub resources: f32,
    /// Drawing on a silo (a silo: not empty) this tick; else decaying.
    pub supplied: bool,
    /// A silo's warning given: 1 low, 2 empty.
    warned: u8,
}

impl Structure {
    fn new(id: u32, kind: StructureKind, team: Team, owner: PlayerId, pos: Vec3, yaw: f32) -> Self {
        Structure {
            id,
            kind,
            team,
            owner,
            pos,
            yaw,
            health: kind.max_health(),
            cooldown: 0.0,
            think: 0.0,
            target: None,
            aim_yaw: yaw,
            aim_pitch: 0.0,
            firing: false,
            resources: if kind == StructureKind::Silo {
                SILO_START
            } else {
                0.0
            },
            supplied: true,
            warned: 0,
        }
    }

    pub fn collider(&self) -> crate::collide::Collider {
        self.kind.collider(self.pos, self.yaw)
    }

    fn snap(&self) -> StructSnap {
        StructSnap {
            id: self.id,
            kind: self.kind,
            team: self.team,
            pos: self.pos,
            yaw: self.yaw,
            aim_yaw: self.aim_yaw,
            aim_pitch: self.aim_pitch,
            health: (self.health / self.kind.max_health() * 100.0).max(0.0),
            firing: self.firing,
            fill: (self.resources / SILO_HOLDS * 100.0)
                .round()
                .clamp(0.0, 100.0) as u8,
            supplied: self.supplied,
        }
    }

    /// Where one spawns at a tube.
    pub fn spawn_spot(&self) -> Vec3 {
        self.pos + look_dir(self.yaw, 0.0) * TUBE_SPAWN
    }
}

impl ContinentSim {
    /// Whether `team` has facility `f`'s benefit here.
    pub fn benefit(&self, team: Team, f: Facility) -> bool {
        let def = &self.map.def;
        self.bases.iter().enumerate().any(|(b, rt)| {
            rt.owner == Some(team)
                && (def.bases[b].kind == BaseKind::Home
                    || (def.facility(b as u8) == Some(f) && rt.powered()))
        })
    }

    /// How long `team`'s dead wait to respawn here.
    pub fn respawn_time(&self, team: Team) -> f32 {
        if self.benefit(team, Facility::BioLab) {
            super::RESPAWN_TIME * 0.5
        } else {
            super::RESPAWN_TIME
        }
    }

    /// The structures' colliders (with the vehicles' while players move; alone for shots).
    pub(super) fn structure_colliders(
        &self,
    ) -> impl Iterator<Item = crate::collide::Collider> + '_ {
        self.structures.iter().map(Structure::collider)
    }

    /// Player `pi` pulls `kind` from harvester `v`'s construction terminal (its load paying).
    pub(super) fn pull(&mut self, pi: usize, v: crate::types::VehicleId, kind: StructureKind) {
        let p = &self.players[pi];
        let (id, team, eye) = (p.id, p.team, p.eye());
        if !p.afoot() {
            return self.deny(id, Deny::TooFar);
        }
        if p.carrying.is_some() {
            return self.deny(id, Deny::Carrying);
        }
        let Some(vi) = self.vidx(v) else {
            return self.deny(id, Deny::TooFar);
        };
        let h = &self.vehicles[vi];
        if h.kind != crate::vehicle::VehicleKind::Harvester || h.team != team {
            return self.deny(id, Deny::NotYours);
        }
        if crate::vehicle::hull_distance(&h.motion, h.kind.spec().half, eye) > TERMINAL_REACH {
            return self.deny(id, Deny::TooFar);
        }
        // (Testing: PS_CHEAT=1 pulls for nothing.)
        let cost = kind.cost() * self.cost_factor[team.index()];
        if !super::cheats() {
            if h.cargo < cost {
                return self.deny(id, Deny::NoResources);
            }
            self.vehicles[vi].cargo -= cost;
        }
        self.players[pi].carrying = Some(kind);
    }

    /// Player `pi` puts the structure it carries (`kind`) down at `at` turned `yaw`.
    pub(super) fn build(&mut self, pi: usize, kind: StructureKind, at: Vec3, yaw: f32) {
        let p = &self.players[pi];
        let (id, team, feet) = (p.id, p.team, p.feet());
        // (Testing: PS_CHEAT=1 builds without carrying it.)
        if p.carrying != Some(kind) && !super::cheats() {
            return self.deny(id, Deny::NotCarrying);
        }
        // (Testing: PS_CHEAT=1 builds anywhere.)
        let cheat = super::cheats();
        if !p.afoot()
            || !at.is_finite()
            || !yaw.is_finite()
            || ((at - feet).length() > BUILD_REACH && !cheat)
        {
            return self.deny(id, Deny::TooFar);
        }
        if self.no_build(at) && !cheat {
            return self.deny(id, Deny::NoBuildZone);
        }
        if kind == StructureKind::Tube
            && self.structures.iter().any(|s| {
                s.kind == StructureKind::Tube
                    && s.team == team
                    && (s.pos - at).length() < SILO_RANGE
            })
        {
            return self.deny(id, Deny::TooManyStructures);
        }
        let Some(pos) = self.build_spot(kind, at, yaw) else {
            return self.deny(id, Deny::NoRoom);
        };
        let s = Structure::new(self.next_structure, kind, team, id, pos, yaw);
        self.next_structure = self.next_structure.wrapping_add(1).max(1);
        self.structures.push(s);
        self.players[pi].carrying = None;
        self.award(id, Award::Build, progress::BUILD_XP);
        self.stat(id, "structures_built", 1);
        self.say(
            Audience::Near(pos, 400.0),
            SimEvent::Built {
                by: id,
                team,
                kind,
                at: pos,
            },
        );
    }

    /// Whether `at` is where no one builds: in a base (or by a tower).
    pub fn no_build(&self, at: Vec3) -> bool {
        self.map.bases.iter().any(|b| {
            let r = if b.kind == BaseKind::Tower {
                30.0
            } else {
                b.half * NO_BUILD
            };
            (b.center - at).truncate_xz().length() < r
        })
    }

    /// The structures as the server keeps them over a restart.
    pub fn saved_structures(&self) -> Vec<SavedStructure> {
        self.structures
            .iter()
            .map(|s| SavedStructure {
                kind: s.kind,
                team: s.team,
                pos: s.pos.to_array(),
                yaw: s.yaw,
                health: s.health,
                resources: s.resources,
            })
            .collect()
    }

    /// Puts back the structures kept over a restart (none of them anyone's).
    pub fn restore_structures(&mut self, list: Vec<SavedStructure>) {
        for k in list {
            let mut s = Structure::new(
                self.next_structure,
                k.kind,
                k.team,
                crate::types::HAZARD,
                Vec3::from_array(k.pos),
                k.yaw,
            );
            s.health = k.health;
            s.resources = k.resources;
            self.next_structure = self.next_structure.wrapping_add(1).max(1);
            self.structures.push(s);
        }
    }

    /// A harvester of `team` at `pos` unloading: the mobile silo of its faction near it with
    /// room (its index).
    pub(super) fn silo_near(&self, team: Team, pos: Vec3) -> Option<usize> {
        self.structures.iter().position(|s| {
            s.kind == StructureKind::Silo
                && s.team == team
                && s.resources < SILO_HOLDS
                && (s.pos - pos).truncate_xz().length() < crate::construct::SILO_UNLOAD + 1.8
        })
    }

    /// Where `kind` stands put at `at` turned `yaw`: on the ground there (the lowest under its
    /// corners), if the ground is level enough and nothing is in the way.
    pub fn build_spot(&self, kind: StructureKind, at: Vec3, yaw: f32) -> Option<Vec3> {
        let corners = kind.corners(at, yaw);
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for c in corners.iter().chain(std::iter::once(&at.truncate_xz())) {
            let y = self.world.ground(c.x, c.y, at.y + 3.0)?;
            lo = lo.min(y);
            hi = hi.max(y);
        }
        if hi - lo > 1.2 {
            return None;
        }
        let pos = Vec3::new(at.x, lo, at.z);
        let (min, max) = kind.collider(pos, yaw).bounds();
        // (Sunk a little into the ground: what stands on it is in the way, not the floor.)
        let (min, max) = (min + Vec3::new(0.05, 0.3, 0.05), max - Vec3::splat(0.05));
        let overlaps = |a: (Vec3, Vec3)| {
            a.0.x < max.x
                && a.1.x > min.x
                && a.0.y < max.y
                && a.1.y > min.y
                && a.0.z < max.z
                && a.1.z > min.z
        };
        let blocked = self
            .map
            .world
            .near_colliders(min.truncate_xz(), max.truncate_xz())
            .iter()
            .any(|c| overlaps(c.bounds()))
            || self
                .structures
                .iter()
                .any(|s| overlaps(s.collider().bounds()))
            || self.world.shields.iter().any(|(c, _)| overlaps(c.bounds()))
            || self.vehicles.iter().any(|v| overlaps(v.bounds().bounds()))
            || self.players.iter().any(|p| {
                p.afoot()
                    && overlaps((
                        p.feet() - Vec3::new(0.5, 0.0, 0.5),
                        p.feet() + Vec3::new(0.5, 1.8, 0.5),
                    ))
            })
            || self.in_bubble(pos + Vec3::Y)
            || self
                .deployables
                .iter()
                .any(|d| overlaps((d.pos - Vec3::splat(0.4), d.pos + Vec3::splat(0.4))));
        (!blocked).then_some(pos)
    }

    /// Spawns dead player `pi` at tube `id`.
    pub(super) fn tube_spawn(&mut self, pi: usize, id: u32) -> Result<(), Deny> {
        let team = self.players[pi].team;
        let Some(t) = self
            .structures
            .iter()
            .find(|s| s.id == id && s.kind == StructureKind::Tube)
        else {
            return Err(Deny::NotReady);
        };
        if t.team != team {
            return Err(Deny::NotYours);
        }
        let (spot, yaw, owner) = (t.spawn_spot(), t.yaw, t.owner);
        let y = self
            .world
            .ground(spot.x, spot.z, spot.y + 3.0)
            .unwrap_or(spot.y);
        let pid = self.players[pi].id;
        self.spawn_at(pid, Vec3::new(spot.x, y, spot.z), yaw);
        if owner != pid {
            self.award(owner, Award::Spawn, progress::SPAWN_XP);
        }
        Ok(())
    }

    pub(super) fn step_structures(&mut self, dt: f32) {
        for si in 0..self.structures.len() {
            let s = &mut self.structures[si];
            s.firing = false;
            s.cooldown = (s.cooldown - dt).max(0.0);
            if s.kind == StructureKind::Bastion {
                self.step_bastion(si, dt);
            }
        }
        self.supply(dt);
        let down: Vec<Structure> = self
            .structures
            .iter()
            .filter(|s| s.health <= 0.0)
            .cloned()
            .collect();
        if down.is_empty() {
            return;
        }
        self.structures.retain(|s| s.health > 0.0);
        for s in down {
            let at = s.pos + Vec3::Y * s.kind.size().1 * 0.5;
            self.fx(Fx::Impact {
                id: 0,
                ammo: Ammo::HeavyShell,
                at,
                what: ImpactOn::Vehicle,
            });
            self.say(
                Audience::Near(at, 600.0),
                SimEvent::StructureDown {
                    kind: s.kind,
                    team: s.team,
                    at,
                },
            );
        }
    }

    /// Upkeep: each structure draws on the nearest silo of its faction that isn't empty (a
    /// silo on itself); without one it decays. Silos running low or dry warn their faction.
    fn supply(&mut self, dt: f32) {
        for si in 0..self.structures.len() {
            let s = &self.structures[si];
            let (kind, team, pos) = (s.kind, s.team, s.pos);
            let need = kind.upkeep() * dt;
            let from = if kind == StructureKind::Silo {
                (s.resources > 0.0).then_some(si)
            } else {
                self.structures
                    .iter()
                    .enumerate()
                    .filter(|(_, o)| {
                        o.kind == StructureKind::Silo
                            && o.team == team
                            && o.resources > 0.0
                            && (o.pos - pos).length() < SILO_RANGE
                    })
                    .min_by(|a, b| {
                        (a.1.pos - pos)
                            .length()
                            .total_cmp(&(b.1.pos - pos).length())
                    })
                    .map(|(i, _)| i)
            };
            match from {
                Some(i) => {
                    let silo = &mut self.structures[i];
                    silo.resources = (silo.resources - need).max(0.0);
                    self.structures[si].supplied = true;
                }
                None => {
                    let s = &mut self.structures[si];
                    s.supplied = false;
                    s.health -= DECAY * dt;
                }
            }
        }
        // Silos low or dry: their faction near them is told (once each way down).
        for si in 0..self.structures.len() {
            let s = &mut self.structures[si];
            if s.kind != StructureKind::Silo {
                continue;
            }
            let level = if s.resources <= 0.0 {
                2
            } else if s.resources < SILO_HOLDS * SILO_LOW {
                1
            } else {
                0
            };
            if level > s.warned {
                s.warned = level;
                let (team, at) = (s.team, s.pos);
                self.say(
                    Audience::Team(team),
                    SimEvent::SiloLow {
                        at,
                        empty: level == 2,
                    },
                );
            } else if level == 0 {
                s.warned = 0;
            }
        }
    }

    fn step_bastion(&mut self, si: usize, dt: f32) {
        let s = &self.structures[si];
        let (team, owner) = (s.team, s.owner);
        let head = s.pos + Vec3::Y * BASTION_PIVOT;
        let mut think = s.think - dt;
        let mut target = s.target;
        if think <= 0.0 {
            think = 0.5;
            target = None;
            let mut best = f32::MAX;
            for p in &self.players {
                if !p.afoot() || p.team == team || p.hidden_from(head) {
                    continue;
                }
                let c = p.feet() + Vec3::Y;
                let d = (c - head).length();
                if d < BASTION_RANGE
                    && d < best
                    && self.world.ray(head, c).is_none()
                    && !self.smoked(head, c)
                {
                    best = d;
                    target = Some(Target::Player(p.id));
                }
            }
            for v in &self.vehicles {
                if v.team == team {
                    continue;
                }
                let c = v.center();
                let d = (c - head).length();
                let range = if v.flying() {
                    BASTION_AIR_RANGE
                } else {
                    BASTION_RANGE
                };
                if d < range
                    && d < best
                    && self.world.ray(head, c).is_none()
                    && !self.smoked(head, c)
                {
                    best = d;
                    target = Some(Target::Vehicle(v.id));
                }
            }
        }
        let aim = match target {
            Some(Target::Player(id)) => {
                self.players
                    .iter()
                    .find(|p| p.id == id && p.afoot())
                    .map(|p| {
                        (
                            p.feet() + Vec3::Y,
                            Vec3::new(p.body.vel.x, p.body.vy, p.body.vel.y),
                        )
                    })
            }
            Some(Target::Vehicle(id)) => self
                .vehicles
                .iter()
                .find(|v| v.id == id)
                .map(|v| (v.center(), v.motion.vel)),
            None => None,
        };
        let s = &mut self.structures[si];
        s.think = think;
        s.target = target.filter(|_| aim.is_some());
        let Some((at, vel)) = aim else { return };
        let speed = Ammo::TurretBolt.ballistics().speed;
        let lead = at + vel * ((at - head).length() / speed);
        let dir = (lead - head).normalize_or(glam::Vec3::NEG_Z);
        let (want_yaw, want_pitch) = (yaw_of(dir), dir.y.clamp(-1.0, 1.0).asin());
        let step = 2.5 * dt;
        s.aim_yaw = wrap_angle(s.aim_yaw + wrap_angle(want_yaw - s.aim_yaw).clamp(-step, step));
        s.aim_pitch += (want_pitch.clamp(-0.6, 1.2) - s.aim_pitch).clamp(-step, step);
        let off = wrap_angle(want_yaw - s.aim_yaw).abs() + (want_pitch - s.aim_pitch).abs();
        if off > 0.06 || s.cooldown > 0.0 {
            return;
        }
        s.cooldown = BASTION_INTERVAL;
        s.firing = true;
        let mut rng = Seq::new(self.tick ^ 0xba57, s.id as u64);
        let d = jitter(look_dir(s.aim_yaw, s.aim_pitch), 0.02, &mut rng);
        let from = head + d * 1.7;
        self.fx(Fx::Muzzle {
            at: from,
            ammo: Ammo::TurretBolt,
        });
        self.launch(Ammo::TurretBolt, owner, team, from, d * speed, 0, None);
    }

    /// The structure a shot from `a` to `b` meets first (not `team`'s): the fraction along and
    /// its index.
    pub(super) fn hit_structure(&self, a: Vec3, b: Vec3, team: Team) -> Option<(f32, usize)> {
        self.structures
            .iter()
            .enumerate()
            .filter(|(_, s)| s.team != team)
            .filter_map(|(i, s)| s.kind.hit(s.pos, s.yaw, a, b).map(|f| (f, i)))
            .min_by(|x, y| x.0.total_cmp(&y.0))
    }

    pub(super) fn damage_structure(&mut self, si: usize, amount: f32, by: PlayerId) {
        let s = &mut self.structures[si];
        if s.health <= 0.0 {
            return;
        }
        s.health -= amount;
        let gone = s.health <= 0.0;
        let team = s.team;
        if by != crate::types::HAZARD {
            self.say(
                Audience::Player(by),
                SimEvent::HitConfirm {
                    by,
                    vehicle: true,
                    headshot: false,
                    kill: gone,
                },
            );
            if gone && self.player(by).is_some_and(|p| p.team != team) {
                self.award(by, Award::Structure, progress::STRUCTURE_XP);
            }
        }
    }

    /// Splash at `at` (reach `r`, `vdmg` at the middle) on other factions' structures.
    pub(super) fn splash_structures(
        &mut self,
        at: Vec3,
        r: f32,
        vdmg: f32,
        by: PlayerId,
        team: Team,
    ) {
        for si in 0..self.structures.len() {
            let s = &self.structures[si];
            if s.team == team {
                continue;
            }
            let (min, max) = s.collider().bounds();
            let d = (at.clamp(min, max) - at).length();
            if d < r {
                self.damage_structure(si, vdmg * (1.0 - d / r), by);
            }
        }
    }

    /// The repair gun mends the faction's structure along the beam: what it mended.
    pub(super) fn repair_structure(
        &mut self,
        team: Team,
        from: Vec3,
        to: Vec3,
        repair: f32,
    ) -> Option<f32> {
        let s = self
            .structures
            .iter_mut()
            .filter(|s| s.team == team && s.health < s.kind.max_health())
            .find(|s| s.kind.hit(s.pos, s.yaw, from, to).is_some())?;
        let before = s.health;
        s.health = (s.health + repair).min(s.kind.max_health());
        Some(s.health - before)
    }

    /// The structures `team` (at `eye`) is sent: its own anywhere, others' in sight range.
    pub(super) fn structure_snaps(&self, team: Option<Team>, eye: Vec3) -> Vec<StructSnap> {
        self.structures
            .iter()
            .filter(|s| Some(s.team) == team || (s.pos - eye).length() < super::VIEW_VEHICLE)
            .map(Structure::snap)
            .collect()
    }

    /// The enemies of `team` its interlink sees: near one of its bases.
    pub(super) fn spotted_by(&self, team: Team) -> Vec<Vec3> {
        if !self.benefit(team, Facility::Interlink) {
            return Vec::new();
        }
        self.bases
            .iter()
            .enumerate()
            .filter(|(_, rt)| rt.owner == Some(team))
            .map(|(b, _)| self.map.bases[b].center)
            .collect()
    }
}

trait Xz {
    fn truncate_xz(self) -> glam::Vec2;
}

impl Xz for Vec3 {
    fn truncate_xz(self) -> glam::Vec2 {
        glam::Vec2::new(self.x, self.z)
    }
}
