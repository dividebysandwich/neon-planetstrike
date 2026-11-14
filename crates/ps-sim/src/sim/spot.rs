//! Spotting: a player marks the enemy it aims at (the spot key) for its faction for
//! `SPOT_TIME`: the faction's snapshots flag it (`flags::SPOTTED`, `vflags::SPOTTED`), its
//! minimaps and screens show it, and if a team mate kills it the spotter earns `SPOT_XP`.

use glam::Vec3;

use super::ContinentSim;
use crate::events::{Audience, SpotTarget};
use crate::progress::{self, Award};
use crate::types::{PlayerId, Team};
use crate::{SimEvent, sim::TICK};

/// Seconds a spot lasts, and between a player's spots.
pub const SPOT_TIME: f32 = 10.0;
pub const SPOT_WAIT: f32 = 1.0;
/// How far one spots infantry and vehicles, and how far off the aim (rad) they may be.
pub const SPOT_RANGE: f32 = 400.0;
pub const SPOT_RANGE_VEHICLE: f32 = 700.0;
pub const SPOT_CONE: f32 = 0.07;

/// A spot: what, for which faction, by whom, until which tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub what: SpotTarget,
    pub team: Team,
    pub by: PlayerId,
    pub until: u64,
}

impl ContinentSim {
    /// Player `pi` spots what it aims at.
    pub(super) fn spot(&mut self, pi: usize) {
        let p = &self.players[pi];
        if !p.alive() || self.tick < p.spot_ready {
            return;
        }
        let (id, team) = (p.id, p.team);
        let (eye, look) = match p.seat.and_then(|(v, s)| self.vehicle(v).map(|v| (v, s))) {
            Some((v, s)) => (v.seat_eye(s as usize), p.cmd.look()),
            None => (p.eye(), p.cmd.look()),
        };
        let in_cone = |c: Vec3, range: f32| {
            let d = c - eye;
            let dist = d.length();
            let off = d.normalize_or(Vec3::Y).angle_between(look);
            // (Near ones: a little wider, a body is wide close up.)
            (dist < range && (off < SPOT_CONE || d.cross(look).length() < 1.2)).then_some(off)
        };
        let seen = |c: Vec3| {
            self.world
                .ray_team(eye, c, Some(team))
                .is_none_or(|h| (h.1 - c).length() < 4.0)
        };
        let mut best: Option<(f32, SpotTarget)> = None;
        for q in &self.players {
            if !q.afoot() || q.team == team || q.hidden_from(eye) {
                continue;
            }
            let c = q.feet() + Vec3::Y;
            if let Some(off) = in_cone(c, SPOT_RANGE)
                && best.is_none_or(|b| off < b.0)
                && seen(c)
            {
                best = Some((off, SpotTarget::Player(q.id)));
            }
        }
        for v in &self.vehicles {
            if v.team == team {
                continue;
            }
            let c = v.center();
            if let Some(off) = in_cone(c, SPOT_RANGE_VEHICLE)
                && best.is_none_or(|b| off < b.0)
                && seen(c)
            {
                best = Some((off, SpotTarget::Vehicle(v.id)));
            }
        }
        self.players[pi].spot_ready = self.tick + (SPOT_WAIT / TICK) as u64;
        let Some((_, what)) = best else { return };
        let until = self.tick + (SPOT_TIME / TICK) as u64;
        match self
            .spots
            .iter_mut()
            .find(|s| s.what == what && s.team == team)
        {
            Some(s) => {
                s.until = until;
                s.by = id;
            }
            None => self.spots.push(Spot {
                what,
                team,
                by: id,
                until,
            }),
        }
        self.say(Audience::Team(team), SimEvent::Spotted { by: id, what });
    }

    pub(super) fn step_spots(&mut self) {
        let tick = self.tick;
        let players = &self.players;
        let vehicles = &self.vehicles;
        self.spots.retain(|s| {
            s.until > tick
                && match s.what {
                    SpotTarget::Player(p) => players.iter().any(|q| q.id == p && q.alive()),
                    SpotTarget::Vehicle(v) => vehicles.iter().any(|q| q.id == v),
                }
        });
    }

    /// Whether `team` has `what` spotted.
    pub fn spotted(&self, team: Team, what: SpotTarget) -> bool {
        self.spots.iter().any(|s| s.team == team && s.what == what)
    }

    /// `what` was killed by `by`: its spotter of `by`'s faction (another player) earns its XP.
    pub(super) fn spot_kill(&mut self, what: SpotTarget, by: PlayerId) {
        let Some(team) = self.player(by).map(|p| p.team) else {
            return;
        };
        let spotter = self
            .spots
            .iter()
            .find(|s| s.what == what && s.team == team && s.by != by)
            .map(|s| s.by);
        if let Some(s) = spotter {
            self.award(s, Award::Spot, progress::SPOT_XP);
        }
    }
}
