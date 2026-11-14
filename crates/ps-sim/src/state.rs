//! Who holds what: the owner of every base and continent, the hacks under way, and the rules
//! that follow from them.
//!
//! - A base can be hacked by a faction that doesn't own it when one of its lattice neighbours
//!   is that faction's base, or a warp gate whose far side's continent that faction owns.
//! - A hack runs `HACK_TIME`; the base's owner stops it by solving the console again.
//! - Holding a continent's center base holds the continent: all its bases flip.
//! - A continent is contested (bots fight there) when its bases have different owners or a
//!   continent linked by a warp gate has another owner. Sanctuaries never are.
//!
//! Kept on disk (RON) by the server, for the same seed and world size.

use serde::{Deserialize, Serialize};

use crate::types::Team;
use crate::worldgen::{BaseIdx, BaseKind, ContinentId, Node, WorldDef, WorldParams};

/// Seconds a hack takes to capture a base.
pub const HACK_TIME: f32 = 300.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hack {
    pub team: Team,
    /// Seconds left.
    pub left: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BaseState {
    pub owner: Option<Team>,
    pub hack: Option<Hack>,
    /// The silo's resources (0..`SILO_CAP`): turrets and vehicles use them up; at none the
    /// base goes neutral, and nobody spawns there until there are some again.
    #[serde(default = "start_resources")]
    pub resources: f32,
}

pub const SILO_CAP: f32 = 1000.0;
pub const START_RESOURCES: f32 = 600.0;

fn start_resources() -> f32 {
    START_RESOURCES
}

impl BaseState {
    pub fn new(owner: Option<Team>) -> BaseState {
        BaseState {
            owner,
            hack: None,
            resources: START_RESOURCES,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContinentState {
    pub owner: Option<Team>,
    pub bases: Vec<BaseState>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldState {
    pub params: WorldParams,
    pub continents: Vec<ContinentState>,
}

/// A change of ownership (for events and the log).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Change {
    BaseCaptured {
        continent: ContinentId,
        base: BaseIdx,
        team: Team,
        from: Option<Team>,
    },
    ContinentCaptured {
        continent: ContinentId,
        team: Team,
        from: Option<Team>,
    },
    HackStarted {
        continent: ContinentId,
        base: BaseIdx,
        team: Team,
    },
    HackStopped {
        continent: ContinentId,
        base: BaseIdx,
        by: Team,
    },
    /// Its silo ran dry: the base went neutral.
    BaseDrained {
        continent: ContinentId,
        base: BaseIdx,
        from: Option<Team>,
    },
}

impl WorldState {
    /// The world at its start: sanctuaries to their factions, every other continent (and its
    /// bases) to the faction whose sanctuary is nearest through the gates (none on a tie).
    pub fn initial(def: &WorldDef) -> WorldState {
        let n = def.continents.len();
        let mut dist = vec![[usize::MAX; 3]; n];
        for t in Team::ALL {
            let s = t.index();
            let mut queue = std::collections::VecDeque::from([(s, 0usize)]);
            while let Some((c, d)) = queue.pop_front() {
                if dist[c][t.index()] <= d {
                    continue;
                }
                dist[c][t.index()] = d;
                for nb in def.neighbours(c as ContinentId) {
                    queue.push_back((nb as usize, d + 1));
                }
            }
        }
        let continents = def
            .continents
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let owner = c.sanctuary.or_else(|| {
                    let best = *dist[i].iter().min().unwrap();
                    let mut who = Team::ALL.iter().filter(|t| dist[i][t.index()] == best);
                    let first = who.next().copied();
                    if who.next().is_some() { None } else { first }
                });
                ContinentState {
                    owner,
                    bases: c.bases.iter().map(|_| BaseState::new(owner)).collect(),
                }
            })
            .collect();
        WorldState {
            params: def.params,
            continents,
        }
    }

    pub fn base(&self, c: ContinentId, b: BaseIdx) -> &BaseState {
        &self.continents[c as usize].bases[b as usize]
    }

    /// Whether `team` may hack base `b` on continent `c` now.
    pub fn can_hack(&self, def: &WorldDef, c: ContinentId, b: BaseIdx, team: Team) -> bool {
        let cd = &def.continents[c as usize];
        let base = &cd.bases[b as usize];
        if base.kind == BaseKind::Home || self.base(c, b).owner == Some(team) {
            return false;
        }
        // Towers stand outside the lattice: anyone may take them.
        if base.kind == BaseKind::Tower {
            return true;
        }
        cd.neighbours(Node::Base(b)).any(|n| match n {
            Node::Base(o) => self.base(c, o).owner == Some(team),
            Node::Gate(g) => {
                let far = cd.gates[g as usize].to;
                self.continents[far as usize].owner == Some(team)
            }
        })
    }

    /// Whether players of `team` may warp to continent `to`: anywhere but an enemy sanctuary.
    pub fn can_warp(def: &WorldDef, to: ContinentId, team: Team) -> bool {
        def.continents[to as usize]
            .sanctuary
            .is_none_or(|s| s == team)
    }

    /// Whether bots fight on continent `c`.
    pub fn contested(&self, def: &WorldDef, c: ContinentId) -> bool {
        let cd = &def.continents[c as usize];
        if cd.sanctuary.is_some() {
            return false;
        }
        let cs = &self.continents[c as usize];
        let first = cs.bases.first().map(|b| b.owner);
        if cs.bases.iter().any(|b| Some(b.owner) != first) {
            return true;
        }
        cd.gates.iter().any(|g| {
            let o = self.continents[g.to as usize].owner;
            o.is_some() && o != cs.owner
        })
    }

    /// The factions with a stake in continent `c`: owners of its bases, and owners of the
    /// continents its gates lead to (who may attack it).
    pub fn factions_on(&self, def: &WorldDef, c: ContinentId) -> Vec<Team> {
        let mut out: Vec<Team> = Vec::new();
        let mut add = |t: Option<Team>| {
            if let Some(t) = t
                && !out.contains(&t)
            {
                out.push(t);
            }
        };
        for b in &self.continents[c as usize].bases {
            add(b.owner);
        }
        for g in &def.continents[c as usize].gates {
            add(self.continents[g.to as usize].owner);
        }
        out.sort();
        out
    }

    /// A hack of base `b` by `team` (its console solved). The owner solving it stops a hack
    /// under way; anyone else who may hack it starts (or takes over) one.
    pub fn console_solved(
        &mut self,
        def: &WorldDef,
        c: ContinentId,
        b: BaseIdx,
        team: Team,
    ) -> Vec<Change> {
        let st = self.continents[c as usize].bases[b as usize];
        if st.owner == Some(team) {
            if st.hack.is_some() {
                self.continents[c as usize].bases[b as usize].hack = None;
                return vec![Change::HackStopped {
                    continent: c,
                    base: b,
                    by: team,
                }];
            }
            return Vec::new();
        }
        if st.hack.is_some_and(|h| h.team == team) || !self.can_hack(def, c, b, team) {
            return Vec::new();
        }
        // A tower is taken at once.
        if def.continents[c as usize].bases[b as usize].kind == BaseKind::Tower {
            return self.capture(def, c, b, team);
        }
        self.continents[c as usize].bases[b as usize].hack = Some(Hack {
            team,
            left: HACK_TIME,
        });
        vec![Change::HackStarted {
            continent: c,
            base: b,
            team,
        }]
    }

    /// Base `b`'s silo ran dry: it goes neutral (its hack, if any, ends).
    pub fn drain(&mut self, c: ContinentId, b: BaseIdx) -> Option<Change> {
        let st = self
            .continents
            .get_mut(c as usize)?
            .bases
            .get_mut(b as usize)?;
        st.resources = 0.0;
        let from = Some(st.owner.take()?);
        st.hack = None;
        Some(Change::BaseDrained {
            continent: c,
            base: b,
            from,
        })
    }

    /// Runs the hacks on continent `c` for `dt` seconds: the captures that follow.
    pub fn tick_hacks(&mut self, def: &WorldDef, c: ContinentId, dt: f32) -> Vec<Change> {
        let mut done = Vec::new();
        for (i, b) in self.continents[c as usize].bases.iter_mut().enumerate() {
            if let Some(h) = &mut b.hack {
                h.left -= dt;
                if h.left <= 0.0 {
                    done.push((i as BaseIdx, h.team));
                }
            }
        }
        let mut out = Vec::new();
        for (b, team) in done {
            out.extend(self.capture(def, c, b, team));
        }
        out
    }

    /// Base `b` goes to `team` (and with the center base, the whole continent).
    pub fn capture(
        &mut self,
        def: &WorldDef,
        c: ContinentId,
        b: BaseIdx,
        team: Team,
    ) -> Vec<Change> {
        let mut out = Vec::new();
        let cs = &mut self.continents[c as usize];
        let from = cs.bases[b as usize].owner;
        cs.bases[b as usize].owner = Some(team);
        cs.bases[b as usize].hack = None;
        out.push(Change::BaseCaptured {
            continent: c,
            base: b,
            team,
            from,
        });
        if def.continents[c as usize].bases[b as usize].kind == BaseKind::Center {
            let was = cs.owner;
            cs.owner = Some(team);
            for (i, base) in cs.bases.iter_mut().enumerate() {
                if i != b as usize && base.owner != Some(team) {
                    let from = base.owner;
                    base.owner = Some(team);
                    base.hack = None;
                    out.push(Change::BaseCaptured {
                        continent: c,
                        base: i as BaseIdx,
                        team,
                        from,
                    });
                }
            }
            out.push(Change::ContinentCaptured {
                continent: c,
                team,
                from: was,
            });
        }
        out
    }

    /// Applies a change the server reported (a client's copy of the state).
    pub fn apply(&mut self, def: &WorldDef, ch: &Change) {
        match *ch {
            Change::BaseDrained {
                continent, base, ..
            } => {
                if let Some(b) = self
                    .continents
                    .get_mut(continent as usize)
                    .and_then(|c| c.bases.get_mut(base as usize))
                {
                    b.owner = None;
                    b.hack = None;
                    b.resources = 0.0;
                }
            }
            Change::BaseCaptured {
                continent,
                base,
                team,
                ..
            } => {
                if let Some(b) = self
                    .continents
                    .get_mut(continent as usize)
                    .and_then(|c| c.bases.get_mut(base as usize))
                {
                    b.owner = Some(team);
                    b.hack = None;
                }
            }
            Change::ContinentCaptured {
                continent, team, ..
            } => {
                if let Some(c) = self.continents.get_mut(continent as usize) {
                    c.owner = Some(team);
                }
            }
            Change::HackStarted {
                continent,
                base,
                team,
            } => {
                if let Some(b) = self
                    .continents
                    .get_mut(continent as usize)
                    .and_then(|c| c.bases.get_mut(base as usize))
                {
                    b.hack = Some(Hack {
                        team,
                        left: HACK_TIME,
                    });
                }
            }
            Change::HackStopped {
                continent, base, ..
            } => {
                if let Some(b) = self
                    .continents
                    .get_mut(continent as usize)
                    .and_then(|c| c.bases.get_mut(base as usize))
                {
                    b.hack = None;
                }
            }
        }
        let _ = def;
    }

    /// Counts the hacks down (between the server's updates).
    pub fn count_down(&mut self, dt: f32) {
        for c in &mut self.continents {
            for b in &mut c.bases {
                if let Some(h) = &mut b.hack {
                    h.left = (h.left - dt).max(0.0);
                }
            }
        }
    }

    /// Whether this state belongs to the world `def` (same seed and size).
    pub fn fits(&self, def: &WorldDef) -> bool {
        self.params == def.params
            && self.continents.len() == def.continents.len()
            && self
                .continents
                .iter()
                .zip(&def.continents)
                .all(|(s, d)| s.bases.len() == d.bases.len())
    }

    pub fn to_ron(&self) -> String {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap_or_default()
    }

    pub fn from_ron(s: &str) -> Option<WorldState> {
        ron::from_str(s).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> WorldDef {
        WorldDef::generate(WorldParams {
            seed: 11,
            contested: 6,
        })
    }

    #[test]
    fn sanctuaries_start_owned_and_unhackable() {
        let def = world();
        let st = WorldState::initial(&def);
        for t in Team::ALL {
            assert_eq!(st.continents[t.index()].owner, Some(t));
            for other in Team::ALL {
                assert!(!st.can_hack(&def, t.index() as ContinentId, 0, other));
            }
        }
        // Enemy sanctuaries are off limits.
        assert!(!WorldState::can_warp(&def, 0, Team::Blue));
        assert!(WorldState::can_warp(&def, 0, Team::Red));
        assert!(WorldState::can_warp(&def, 4, Team::Green));
    }

    /// A continent next to one owned by `attacker` while it is owned by someone else.
    fn front(def: &WorldDef, st: &WorldState, attacker: Team) -> Option<(ContinentId, u8)> {
        for c in &def.continents {
            if c.sanctuary.is_some() || st.continents[c.id as usize].owner == Some(attacker) {
                continue;
            }
            for (gi, g) in c.gates.iter().enumerate() {
                if st.continents[g.to as usize].owner == Some(attacker) {
                    return Some((c.id, gi as u8));
                }
            }
        }
        None
    }

    #[test]
    fn hacks_follow_the_lattice_and_the_center_flips_the_continent() {
        let def = world();
        let mut st = WorldState::initial(&def);
        let attacker = Team::ALL
            .into_iter()
            .find(|&t| front(&def, &st, t).is_some())
            .unwrap();
        let (c, gate) = front(&def, &st, attacker).unwrap();
        let cd = &def.continents[c as usize];
        assert!(st.contested(&def, c));
        let next_to_gate: Vec<BaseIdx> = cd
            .neighbours(Node::Gate(gate))
            .filter_map(|n| match n {
                Node::Base(b) => Some(b),
                _ => None,
            })
            .collect();
        assert!(!next_to_gate.is_empty());
        for (b, site) in cd.bases.iter().enumerate() {
            let b = b as BaseIdx;
            if site.kind == BaseKind::Tower {
                // (Anyone may take a tower.)
                assert_eq!(
                    st.can_hack(&def, c, b, attacker),
                    st.base(c, b).owner != Some(attacker)
                );
                continue;
            }
            let linked = cd.neighbours(Node::Base(b)).any(|n| match n {
                Node::Gate(g) => {
                    st.continents[cd.gates[g as usize].to as usize].owner == Some(attacker)
                }
                Node::Base(o) => st.base(c, o).owner == Some(attacker),
            });
            assert_eq!(st.can_hack(&def, c, b, attacker), linked, "base {b}");
        }
        // Hack the gate's base: the owner stops it, the attacker hacks again and holds out.
        let b = next_to_gate[0];
        let owner = st.base(c, b).owner;
        assert!(matches!(
            st.console_solved(&def, c, b, attacker).first(),
            Some(Change::HackStarted { .. })
        ));
        if let Some(o) = owner {
            assert!(matches!(
                st.console_solved(&def, c, b, o).first(),
                Some(Change::HackStopped { .. })
            ));
            st.console_solved(&def, c, b, attacker);
        }
        assert!(st.tick_hacks(&def, c, HACK_TIME - 1.0).is_empty());
        let ch = st.tick_hacks(&def, c, 2.0);
        assert!(matches!(ch[0], Change::BaseCaptured { team, .. } if team == attacker));
        assert_eq!(st.base(c, b).owner, Some(attacker));
        // Walk the lattice to the center, capturing each base on the way.
        let center = cd.center_base().unwrap();
        let mut guard = 0;
        while st.base(c, center).owner != Some(attacker) {
            guard += 1;
            assert!(guard < 50);
            let next = (0..cd.bases.len() as BaseIdx)
                .find(|&b| st.can_hack(&def, c, b, attacker))
                .expect("something hackable");
            st.capture(&def, c, next, attacker);
        }
        assert_eq!(st.continents[c as usize].owner, Some(attacker));
        assert!(
            st.continents[c as usize]
                .bases
                .iter()
                .all(|b| b.owner == Some(attacker))
        );
    }

    #[test]
    fn state_round_trips_through_ron() {
        let def = world();
        let mut st = WorldState::initial(&def);
        st.continents[4].bases[1].hack = Some(Hack {
            team: Team::Green,
            left: 12.5,
        });
        let back = WorldState::from_ron(&st.to_ron()).unwrap();
        assert_eq!(back, st);
        assert!(back.fits(&def));
    }
}
