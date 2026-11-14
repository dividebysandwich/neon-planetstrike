//! The class tools: the infiltrator's cloak and hacking, and the engineer's deployables
//! (spitfire turrets, anti-tank mines, ammo packs).
//!
//! The cloak (the deploy key in the infiltration suit) lasts `CLOAK_TIME` from full and fills
//! again slowly; firing, hacking, a seat or death drop it. Enemies are sent a cloaked player
//! only within `CLOAK_SEEN` (and faint), and turrets, spitfires and bots don't see one further
//! off.
//!
//! Hacking: an infiltrator at an enemy terminal (equipment, medical, vehicle) or turret, still
//! for `HACK_TIME`, turns it to its faction's for `HACKED_FOR`: its faction may use the
//! terminal, and the turret fires for (and may be manned by) its faction. A base changing hands
//! clears its hacks.
//!
//! Deployables (the deploy key on foot with the repair gun's kit): put down a step ahead on the
//! ground, `Kit::count` of a kind at a time per player (the oldest goes for a new one), as many
//! as the loadout carries (refilled at an equipment terminal). They stay until destroyed or
//! their owner leaves the continent (an ammo pack runs out after `AMMO_PACK_LIFE`).

use glam::Vec3;

use super::{ContinentSim, Player, Target, jitter};
use crate::base::{TURRET_PIVOT, TerminalKind, USE_REACH};
use crate::events::{Audience, Cause, Deny, HackTarget, SimEvent};
use crate::noise::Seq;
use crate::progress::Award;
use crate::snapshot::{DeploySnap, Fx, ImpactOn};
use crate::types::{PlayerId, Team, look_dir, wrap_angle, yaw_of};
use crate::vehicle;
use crate::weapons::{self, Ammo, ArmorKind, Kit};

/// A spitfire: how far it sees, how fast it fires and turns.
pub const SPITFIRE_RANGE: f32 = 40.0;
pub const SPITFIRE_INTERVAL: f32 = 0.22;
const SPITFIRE_TURN: f32 = 3.0;
const SPITFIRE_SPREAD: f32 = 0.03;
/// The spitfire's gun's height over the ground.
pub const SPITFIRE_PIVOT: f32 = 0.9;
/// A mine goes off with an enemy ground vehicle's hull this near (m).
pub const MINE_TRIGGER: f32 = 1.6;
/// A mine's blast: vehicle damage (percent) at the middle, its reach, infantry damage.
pub const MINE_DAMAGE: f32 = 45.0;
pub const MINE_RADIUS: f32 = 5.0;
pub const MINE_INFANTRY: f32 = 90.0;
/// Enemies see a mine only this near.
pub const MINE_SEEN: f32 = 15.0;
/// An ammo pack: its reach, how often it hands out, and how long it lasts.
pub const AMMO_PACK_REACH: f32 = 5.0;
const AMMO_PACK_EVERY: f32 = 1.0;
pub const AMMO_PACK_LIFE: f32 = 120.0;
/// XP to the hacker.
pub const HACK_TERMINAL_XP: u32 = 100;
/// How far from a turret's base an infiltrator may hack it.
pub const TURRET_REACH: f32 = 6.5;

#[derive(Clone, Debug)]
pub struct Deployable {
    pub id: u32,
    pub kind: Kit,
    pub team: Team,
    pub owner: PlayerId,
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub health: f32,
    pub cooldown: f32,
    pub think: f32,
    pub target: Option<Target>,
    /// Seconds since it was put down.
    pub age: f32,
    pub firing: bool,
}

impl Deployable {
    /// How big it is to shots (a sphere about its middle).
    pub fn center(&self) -> Vec3 {
        let h = match self.kind {
            Kit::Spitfire => SPITFIRE_PIVOT * 0.7,
            Kit::Mines => 0.1,
            _ => 0.3,
        };
        self.pos + Vec3::Y * h
    }

    pub fn radius(&self) -> f32 {
        match self.kind {
            Kit::Spitfire => 0.75,
            Kit::Mines => 0.45,
            _ => 0.5,
        }
    }

    pub fn max_health(&self) -> f32 {
        match self.kind {
            Kit::Spitfire => 100.0,
            _ => 40.0,
        }
    }

    pub fn snap(&self) -> DeploySnap {
        DeploySnap {
            id: self.id,
            kind: self.kind,
            team: self.team,
            owner: self.owner,
            pos: self.pos,
            yaw: self.yaw,
            pitch: self.pitch,
            health: (self.health / self.max_health() * 100.0).max(0.0),
            firing: self.firing,
        }
    }
}

impl Player {
    /// Hidden from someone at `from` by its cloak.
    pub fn hidden_from(&self, from: Vec3) -> bool {
        self.cloaked && (self.feet() - from).length() > weapons::CLOAK_SEEN
    }

    pub fn max_suit(&self) -> bool {
        self.loadout.armor == ArmorKind::Max
    }

    fn uncloak(&mut self) {
        self.cloaked = false;
    }
}

impl ContinentSim {
    /// Whose terminal `t` of base `b` is now: a hack's faction, else the base's owner.
    pub fn terminal_team(&self, b: usize, t: usize) -> Option<Team> {
        let rt = self.bases.get(b)?;
        rt.hacked
            .get(t)
            .copied()
            .flatten()
            .map(|h| h.0)
            .or(rt.owner)
    }

    /// Whether turret `ti`'s faction has an Amp Station's benefit.
    pub fn turret_amped(&self, ti: usize) -> bool {
        self.turret_team(ti)
            .is_some_and(|t| self.benefit(t, crate::worldgen::Facility::AmpStation))
    }

    /// Whose turret `ti` is now: a hack's faction, else its base's owner.
    pub fn turret_team(&self, ti: usize) -> Option<Team> {
        let t = self.turrets.get(ti)?;
        t.hacked.map(|h| h.0).or(self.bases[t.base as usize].owner)
    }

    /// A terminal of `kind` in reach of player `pi` that its faction may use: (base, index).
    pub(super) fn terminal_near(&self, pi: usize, kind: TerminalKind) -> Option<(usize, usize)> {
        let p = &self.players[pi];
        if !p.afoot() {
            return None;
        }
        self.map.bases.iter().enumerate().find_map(|(bi, b)| {
            b.terminals.iter().enumerate().find_map(|(ti, t)| {
                (t.kind == kind
                    && (t.pos - p.feet()).length() < USE_REACH + 0.5
                    && self.terminal_team(bi, ti) == Some(p.team))
                .then_some((bi, ti))
            })
        })
    }

    /// Starts player `pi` hacking `what` (checked: an infiltrator on foot, in reach, the thing
    /// an enemy's).
    pub(super) fn start_hack(&mut self, pi: usize, what: HackTarget) {
        let p = &self.players[pi];
        let id = p.id;
        if p.loadout.armor != ArmorKind::Stealth {
            return self.deny(
                id,
                if p.max_suit() {
                    Deny::Max
                } else {
                    Deny::NotInfiltrator
                },
            );
        }
        if !p.afoot() {
            return self.deny(id, Deny::TooFar);
        }
        let Some(team) = self.hack_owner(what) else {
            return self.deny(id, Deny::NotReady);
        };
        if !self.in_hack_reach(pi, what) {
            return self.deny(id, Deny::TooFar);
        }
        if team == Some(self.players[pi].team) {
            return self.deny(id, Deny::Already);
        }
        let p = &mut self.players[pi];
        p.hacking = Some((what, 0.0));
        p.uncloak();
    }

    /// Whose the hack target is now (None: no such thing; Some(None): nobody's, it can't be
    /// hacked, so that's refused too).
    fn hack_owner(&self, what: HackTarget) -> Option<Option<Team>> {
        match what {
            HackTarget::Terminal { base, index } => {
                let l = self.map.bases.get(base as usize)?;
                let t = l.terminals.get(index as usize)?;
                if !matches!(
                    t.kind,
                    TerminalKind::Equipment | TerminalKind::Medical | TerminalKind::Vehicle
                ) {
                    return None;
                }
                let team = self.terminal_team(base as usize, index as usize)?;
                Some(Some(team))
            }
            HackTarget::Turret(ti) => {
                let t = self.turrets.get(ti as usize)?;
                if t.health <= 0.0 {
                    return None;
                }
                Some(Some(self.turret_team(ti as usize)?))
            }
        }
    }

    fn in_hack_reach(&self, pi: usize, what: HackTarget) -> bool {
        let p = &self.players[pi];
        match what {
            HackTarget::Terminal { base, index } => self
                .map
                .bases
                .get(base as usize)
                .and_then(|l| l.terminals.get(index as usize))
                .is_some_and(|t| (t.pos - p.feet()).length() < USE_REACH + 0.8),
            HackTarget::Turret(ti) => self.turrets.get(ti as usize).is_some_and(|t| {
                (t.pos + Vec3::Y * TURRET_PIVOT * 0.5 - p.eye()).length() < TURRET_REACH
            }),
        }
    }

    /// Per tick: cloaks, hacks, putting deployables down, the hacks' and deployables' time.
    pub(super) fn step_kit(&mut self, dt: f32) {
        for pi in 0..self.players.len() {
            self.step_class(pi, dt);
        }
        for rt in &mut self.bases {
            for h in rt.hacked.iter_mut() {
                if let Some((_, left)) = h {
                    *left -= dt;
                    if *left <= 0.0 {
                        *h = None;
                    }
                }
            }
        }
        for t in &mut self.turrets {
            if let Some((_, left)) = &mut t.hacked {
                *left -= dt;
                if *left <= 0.0 {
                    t.hacked = None;
                    t.target = None;
                }
            }
        }
        self.step_deployables(dt);
    }

    fn step_class(&mut self, pi: usize, dt: f32) {
        let p = &mut self.players[pi];
        // (The deploy key on foot: the cloak or a deployable. In a seat it's the vehicle's.)
        let afoot = p.afoot();
        let pressed = afoot && p.deploys_seen.is_some_and(|s| s != p.cmd.deploys);
        if afoot {
            p.deploys_seen = Some(p.cmd.deploys);
        }
        if !afoot {
            p.cloaked = false;
            p.hacking = None;
        }
        // The cloak.
        if p.loadout.armor != ArmorKind::Stealth {
            p.cloaked = false;
        } else if p.cloaked {
            p.cloak -= dt / weapons::CLOAK_TIME;
            if p.cloak <= 0.0 {
                p.cloak = 0.0;
                p.cloaked = false;
            }
        } else {
            p.cloak = (p.cloak + weapons::CLOAK_REFILL * dt).min(1.0);
        }
        if pressed {
            if p.loadout.armor == ArmorKind::Stealth {
                if p.cloaked {
                    p.cloaked = false;
                } else if p.cloak > 0.15 && p.hacking.is_none() {
                    p.cloaked = true;
                }
            } else if p.loadout.kit() != Kit::None {
                self.put_down(pi);
            }
        }
        // A hack under way: still at it and in reach, until done.
        let Some((what, done)) = self.players[pi].hacking else {
            return;
        };
        let still = self.players[pi].body.vel.length() < 1.0;
        let owner = self.hack_owner(what);
        let team = self.players[pi].team;
        if !self.in_hack_reach(pi, what) || !still || owner.is_none_or(|o| o == Some(team)) {
            self.players[pi].hacking = None;
            return;
        }
        let done = done + dt;
        if done < weapons::HACK_TIME {
            self.players[pi].hacking = Some((what, done));
            return;
        }
        self.players[pi].hacking = None;
        let id = self.players[pi].id;
        match what {
            HackTarget::Terminal { base, index } => {
                let rt = &mut self.bases[base as usize];
                if rt.hacked.len() <= index as usize {
                    rt.hacked.resize(index as usize + 1, None);
                }
                rt.hacked[index as usize] = Some((team, weapons::HACKED_FOR));
            }
            HackTarget::Turret(ti) => {
                let ti = ti as usize;
                // Its gunner (of the other side) is put out.
                if let Some(occ) = self.turrets[ti].occupant
                    && let Some(oi) = self.pidx(occ)
                    && self.players[oi].team != team
                {
                    self.leave_turret(oi);
                }
                let t = &mut self.turrets[ti];
                t.hacked = Some((team, weapons::HACKED_FOR));
                t.target = None;
            }
        }
        self.award(id, Award::Hack, HACK_TERMINAL_XP);
        self.stat(id, "terminal_hacks", 1);
        let at = self.players[pi].feet();
        self.say(
            Audience::Near(at, 300.0),
            SimEvent::Hacked { by: id, team, what },
        );
    }

    /// Player `pi` puts its kit's deployable down a step ahead.
    fn put_down(&mut self, pi: usize) {
        let p = &self.players[pi];
        let (id, team, kind) = (p.id, p.team, p.loadout.kit());
        if p.kit_left == 0 {
            return self.deny(id, Deny::NotReady);
        }
        let ahead = look_dir(p.cmd.yaw, 0.0);
        let mut at = p.feet() + ahead * 1.4;
        let Some(y) = self.world.ground(at.x, at.z, at.y + 1.0) else {
            return self.deny(id, Deny::NoRoom);
        };
        if (y - p.feet().y).abs() > 1.2
            || self.in_bubble(at + Vec3::Y)
            || self
                .world
                .ray(p.eye(), Vec3::new(at.x, y + 0.3, at.z))
                .is_some()
            || self
                .deployables
                .iter()
                .any(|d| (d.pos - Vec3::new(at.x, y, at.z)).length() < 1.2)
        {
            return self.deny(id, Deny::NoRoom);
        }
        at.y = y;
        // Over the limit: the oldest of the kind goes.
        let mine: Vec<usize> = (0..self.deployables.len())
            .filter(|&i| self.deployables[i].owner == id && self.deployables[i].kind == kind)
            .collect();
        if mine.len() >= kind.count() as usize {
            self.deployables.remove(mine[0]);
        }
        let d = Deployable {
            id: self.next_deployable,
            kind,
            team,
            owner: id,
            pos: at,
            yaw: self.players[pi].cmd.yaw,
            pitch: 0.0,
            health: 0.0,
            cooldown: 0.0,
            think: 0.0,
            target: None,
            age: 0.0,
            firing: false,
        };
        let d = Deployable {
            health: d.max_health(),
            ..d
        };
        self.next_deployable = self.next_deployable.wrapping_add(1).max(1);
        self.deployables.push(d);
        self.players[pi].kit_left -= 1;
        self.say(
            Audience::Near(at, 60.0),
            SimEvent::Deployed { by: id, kind, at },
        );
    }

    /// Takes away a player's deployables (it left the continent).
    pub(super) fn drop_deployables(&mut self, id: PlayerId) {
        self.deployables.retain(|d| d.owner != id);
    }

    fn step_deployables(&mut self, dt: f32) {
        let mut gone: Vec<u32> = Vec::new();
        for di in 0..self.deployables.len() {
            let d = &mut self.deployables[di];
            d.age += dt;
            d.cooldown = (d.cooldown - dt).max(0.0);
            d.firing = false;
            if d.health <= 0.0 {
                gone.push(d.id);
                continue;
            }
            match d.kind {
                Kit::Spitfire => self.step_spitfire(di, dt),
                Kit::Mines => {
                    if let Some(vid) = self.mine_trigger(di) {
                        let d = self.deployables[di].clone();
                        gone.push(d.id);
                        self.mine_blast(&d, vid);
                    }
                }
                Kit::AmmoPack => {
                    let d = &self.deployables[di];
                    if d.age >= AMMO_PACK_LIFE {
                        gone.push(d.id);
                        continue;
                    }
                    if d.cooldown <= 0.0 {
                        let (pos, team) = (d.pos, d.team);
                        self.deployables[di].cooldown = AMMO_PACK_EVERY;
                        self.resupply(pos, team);
                    }
                }
                Kit::None => gone.push(d.id),
            }
        }
        if !gone.is_empty() {
            for d in self.deployables.iter().filter(|d| gone.contains(&d.id)) {
                if d.health <= 0.0 {
                    self.fx.push_back((
                        self.tick,
                        Fx::Impact {
                            id: 0,
                            ammo: Ammo::Grenade,
                            at: d.center(),
                            what: ImpactOn::Vehicle,
                        },
                    ));
                }
            }
            self.deployables.retain(|d| !gone.contains(&d.id));
        }
    }

    fn step_spitfire(&mut self, di: usize, dt: f32) {
        let d = &self.deployables[di];
        let (team, owner) = (d.team, d.owner);
        let head = d.pos + Vec3::Y * SPITFIRE_PIVOT;
        let mut think = d.think - dt;
        let mut target = d.target;
        if think <= 0.0 {
            think = 0.4;
            target = None;
            let mut best = SPITFIRE_RANGE;
            for p in &self.players {
                if !p.afoot() || p.team == team || p.hidden_from(head) {
                    continue;
                }
                let c = p.feet() + Vec3::Y * 1.0;
                let dist = (c - head).length();
                if dist < best && self.world.ray(head, c).is_none() && !self.smoked(head, c) {
                    best = dist;
                    target = Some(Target::Player(p.id));
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
                            p.feet() + Vec3::Y * 1.0,
                            Vec3::new(p.body.vel.x, p.body.vy, p.body.vel.y),
                        )
                    })
            }
            _ => None,
        };
        let d = &mut self.deployables[di];
        d.think = think;
        d.target = target.filter(|_| aim.is_some());
        let Some((at, vel)) = aim else { return };
        let speed = Ammo::Spitfire.ballistics().speed;
        let lead = at + vel * ((at - head).length() / speed);
        let dir = (lead - head).normalize_or(Vec3::NEG_Z);
        let (want_yaw, want_pitch) = (yaw_of(dir), dir.y.clamp(-1.0, 1.0).asin());
        let step = SPITFIRE_TURN * dt;
        d.yaw = wrap_angle(d.yaw + wrap_angle(want_yaw - d.yaw).clamp(-step, step));
        d.pitch += (want_pitch.clamp(-0.6, 0.9) - d.pitch).clamp(-step, step);
        let off = wrap_angle(want_yaw - d.yaw).abs() + (want_pitch - d.pitch).abs();
        if off > 0.08 || d.cooldown > 0.0 {
            return;
        }
        d.cooldown = SPITFIRE_INTERVAL;
        d.firing = true;
        let mut rng = Seq::new(self.tick ^ 0x5b17, d.id as u64);
        let dir = jitter(look_dir(d.yaw, d.pitch), SPITFIRE_SPREAD, &mut rng);
        let from = head + dir * 0.6;
        self.fx(Fx::Muzzle {
            at: from,
            ammo: Ammo::Spitfire,
        });
        self.launch(Ammo::Spitfire, owner, team, from, dir * speed, 0, None);
    }

    /// An enemy ground vehicle over mine `di`.
    fn mine_trigger(&self, di: usize) -> Option<crate::types::VehicleId> {
        let d = &self.deployables[di];
        self.vehicles
            .iter()
            .filter(|v| v.team != d.team && !v.flying())
            .find(|v| {
                vehicle::hull_distance(&v.motion, v.kind.spec().half, d.pos + Vec3::Y * 0.3)
                    < MINE_TRIGGER
            })
            .map(|v| v.id)
    }

    fn mine_blast(&mut self, d: &Deployable, on: crate::types::VehicleId) {
        self.fx(Fx::Impact {
            id: 0,
            ammo: Ammo::Mine,
            at: d.pos + Vec3::Y * 0.3,
            what: ImpactOn::Vehicle,
        });
        // The vehicle over it takes the brunt; the blast reaches what's around.
        self.damage_vehicle(on, MINE_DAMAGE, d.owner);
        self.splash(
            d.pos + Vec3::Y * 0.3,
            MINE_RADIUS,
            MINE_INFANTRY,
            MINE_DAMAGE * 0.3,
            d.owner,
            d.team,
            Cause::Deployable(Kit::Mines),
            false,
        );
    }

    /// An ammo pack at `pos` hands out to `team` near it: a tenth of each gun's reserve, and
    /// a grenade now and then.
    fn resupply(&mut self, pos: Vec3, team: Team) {
        let tick = self.tick;
        for p in &mut self.players {
            if !p.afoot() || p.team != team || (p.feet() - pos).length() > AMMO_PACK_REACH {
                continue;
            }
            for g in &mut p.guns {
                let s = g.weapon.stats();
                if s.shot == weapons::ShotKind::Bolt || g.weapon == weapons::Weapon::RepairGun {
                    let full = s.reserve + s.mag;
                    let have = g.reserve + g.mag;
                    if have < full {
                        g.reserve += (s.reserve / 10).max(1).min(full - have);
                    }
                }
            }
            if tick.is_multiple_of(5 * 60) && p.grenades < weapons::GRENADES {
                p.grenades += 1;
            }
        }
    }

    /// Damage to deployable `di` (from someone of `team`).
    pub(super) fn damage_deployable(&mut self, di: usize, amount: f32, by: PlayerId) {
        let d = &mut self.deployables[di];
        d.health -= amount;
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

    /// The deployable a shot from `a` to `b` meets first (not `team`'s): the fraction along
    /// and its index.
    pub(super) fn hit_deployable(&self, a: Vec3, b: Vec3, team: Team) -> Option<(f32, usize)> {
        let seg = b - a;
        let len2 = seg.length_squared().max(1e-6);
        self.deployables
            .iter()
            .enumerate()
            .filter(|(_, d)| d.team != team && d.health > 0.0)
            .filter_map(|(i, d)| {
                let c = d.center();
                let k = ((c - a).dot(seg) / len2).clamp(0.0, 1.0);
                ((a + seg * k - c).length() < d.radius()).then_some((k, i))
            })
            .min_by(|x, y| x.0.total_cmp(&y.0))
    }

    /// The repair gun mends the faction's deployables along the beam: true if it met one.
    pub(super) fn repair_deployable(
        &mut self,
        team: Team,
        from: Vec3,
        dir: Vec3,
        reach: f32,
        repair: f32,
    ) -> Option<f32> {
        for d in &mut self.deployables {
            if d.team != team || d.health >= d.max_health() {
                continue;
            }
            let c = d.center();
            let k = (c - from).dot(dir).clamp(0.0, reach);
            if (from + dir * k - c).length() < d.radius() + 0.4 {
                let before = d.health;
                d.health = (d.health + repair).min(d.max_health());
                return Some(d.health - before);
            }
        }
        None
    }

    /// The deployables `viewer` (of `team`, at `eye`) is sent: near ones, and an enemy's
    /// mines only close.
    pub(super) fn deploy_snaps(&self, team: Option<Team>, eye: Vec3) -> Vec<DeploySnap> {
        self.deployables
            .iter()
            .filter(|d| {
                let dist = (d.pos - eye).length();
                dist < 400.0 && (d.kind != Kit::Mines || Some(d.team) == team || dist < MINE_SEEN)
            })
            .map(Deployable::snap)
            .collect()
    }
}

impl ContinentSim {
    /// An EMP, smoke or flash grenade (`by`, of `team`) going off at `at`.
    pub fn utility_burst(&mut self, k: weapons::GrenadeKind, at: Vec3, by: PlayerId, team: Team) {
        use weapons::GrenadeKind;
        match k {
            GrenadeKind::Frag => {}
            GrenadeKind::Smoke => self.smokes.push((at, weapons::SMOKE_TIME)),
            GrenadeKind::Emp => {
                for p in &mut self.players {
                    if p.afoot() && p.team != team && (p.feet() - at).length() < weapons::EMP_RADIUS
                    {
                        p.armor = (p.armor - weapons::EMP_ARMOR).max(0.0);
                        p.cloaked = false;
                        p.cloak = 0.0;
                    }
                }
                for d in &mut self.deployables {
                    if d.team != team && (d.center() - at).length() < weapons::EMP_RADIUS {
                        d.health -= d.max_health() * 0.5;
                    }
                }
            }
            GrenadeKind::Flash => {
                let hit: Vec<(PlayerId, f32)> = self
                    .players
                    .iter()
                    .filter(|p| p.afoot() && p.team != team)
                    .filter_map(|p| {
                        let d = (p.eye() - at).length();
                        (d < weapons::FLASH_RADIUS
                            && self.world.ray(at + Vec3::Y * 0.2, p.eye()).is_none())
                        .then(|| {
                            (
                                p.id,
                                weapons::FLASH_TIME * (1.0 - 0.5 * d / weapons::FLASH_RADIUS),
                            )
                        })
                    })
                    .collect();
                for (id, secs) in hit {
                    if let Some(i) = self.pidx(id) {
                        self.players[i].flashed = self.players[i].flashed.max(secs);
                    }
                    self.say(Audience::Player(id), SimEvent::Flashed { player: id, secs });
                }
            }
        }
        let _ = by;
    }

    /// Whether a smoke cloud stands between `a` and `b`.
    pub fn smoked(&self, a: Vec3, b: Vec3) -> bool {
        smoked_in(&self.smokes, a, b)
    }
}

/// Whether one of `smokes` stands between `a` and `b`.
pub fn smoked_in(smokes: &[(Vec3, f32)], a: Vec3, b: Vec3) -> bool {
    let seg = b - a;
    let len2 = seg.length_squared().max(1e-6);
    smokes.iter().any(|(c, _)| {
        let k = ((*c - a).dot(seg) / len2).clamp(0.0, 1.0);
        (a + seg * k - *c).length() < weapons::SMOKE_RADIUS
    })
}
