//! Squads: up to `SQUAD_MAX` players of a faction, across continents, a human leading. The hub
//! keeps them; each faction is told of its own (`ToClient::Squads`) whenever they change. The
//! leader sets the squad's waypoint and its members may spawn on it. Bots of the faction on the
//! leader's continent fill a squad up to one short of full (`fill_bots`): a human may always
//! join, and a bot then makes room.

use ps_net::protocol::{SQUAD_MAX, SquadCmd, SquadId, SquadInfo};
use ps_sim::{ContinentId, PlayerId, Team};

#[derive(Clone, Debug, PartialEq)]
pub struct Squad {
    pub id: SquadId,
    pub team: Team,
    /// The leader first.
    pub members: Vec<PlayerId>,
    pub waypoint: Option<(ContinentId, glam::Vec3)>,
}

impl Squad {
    pub fn leader(&self) -> PlayerId {
        self.members[0]
    }

    fn info(&self) -> SquadInfo {
        SquadInfo {
            id: self.id,
            leader: self.leader(),
            members: self.members.clone(),
            waypoint: self.waypoint,
        }
    }
}

#[derive(Default)]
pub struct Squads {
    list: Vec<Squad>,
    next: SquadId,
}

impl Squads {
    /// The squad player `p` is in.
    pub fn of(&self, p: PlayerId) -> Option<&Squad> {
        self.list.iter().find(|s| s.members.contains(&p))
    }

    /// Faction `team`'s squads, as its players are told of them.
    pub fn info(&self, team: Team) -> Vec<SquadInfo> {
        self.list
            .iter()
            .filter(|s| s.team == team)
            .map(Squad::info)
            .collect()
    }

    /// Takes `p` out of its squad (the next member leads; an empty squad goes). The faction
    /// whose squads changed, if any.
    pub fn leave(&mut self, p: PlayerId) -> Option<Team> {
        let i = self.list.iter().position(|s| s.members.contains(&p))?;
        let s = &mut self.list[i];
        let team = s.team;
        let was_leader = s.leader() == p;
        s.members.retain(|&m| m != p);
        if s.members.is_empty() {
            self.list.remove(i);
        } else if was_leader {
            // (A new leader's squad starts without the old one's waypoint.)
            s.waypoint = None;
        }
        Some(team)
    }

    /// Player `p` of faction `team` does `cmd`; `continents` bounds a waypoint's continent.
    /// The faction whose squads changed, if any.
    pub fn apply(
        &mut self,
        p: PlayerId,
        team: Team,
        cmd: SquadCmd,
        continents: usize,
    ) -> Option<Team> {
        let leads = self.of(p).is_some_and(|s| s.leader() == p);
        match cmd {
            SquadCmd::Create => {
                if leads && self.of(p).is_some_and(|s| s.members.len() == 1) {
                    return None;
                }
                self.leave(p);
                self.next = self.next.wrapping_add(1);
                while self.list.iter().any(|s| s.id == self.next) {
                    self.next = self.next.wrapping_add(1);
                }
                self.list.push(Squad {
                    id: self.next,
                    team,
                    members: vec![p],
                    waypoint: None,
                });
                Some(team)
            }
            SquadCmd::Join(id) => {
                let s = self.list.iter().find(|s| s.id == id)?;
                if s.team != team || s.members.len() >= SQUAD_MAX || s.members.contains(&p) {
                    return None;
                }
                // (A bot makes room for a human, if it must, at the next `fill_bots`.)
                self.leave(p);
                // (Leaving may have emptied nothing of the one joined: it has others in it.)
                let s = self.list.iter_mut().find(|s| s.id == id)?;
                s.members.push(p);
                Some(team)
            }
            SquadCmd::Leave => self.leave(p),
            SquadCmd::Waypoint(w) => {
                if !leads {
                    return None;
                }
                let w = w.filter(|(c, at)| (*c as usize) < continents && at.is_finite());
                let s = self.list.iter_mut().find(|s| s.members.contains(&p))?;
                s.waypoint = w;
                Some(team)
            }
            SquadCmd::Promote(to) => {
                let s = self.list.iter_mut().find(|s| s.members.contains(&p))?;
                let i = s.members.iter().position(|&m| m == to)?;
                if !leads || to == p {
                    return None;
                }
                // (To the front; the others keep their order, the next in line to lead.)
                let to = s.members.remove(i);
                s.members.insert(0, to);
                Some(team)
            }
            SquadCmd::Kick(who) => {
                if !leads || who == p || self.of(who).is_none_or(|s| !s.members.contains(&p)) {
                    return None;
                }
                self.leave(who)
            }
        }
    }
}

impl Squads {
    /// Bots in squads: those gone from the leader's continent (or the game) out, a human to the
    /// lead (a squad with none left goes), and squads filled with free bots of their faction on
    /// their leader's continent up to one short of `SQUAD_MAX` (bots out to make that room for
    /// humans who joined). `human`: whether a player is a human, and on which continent;
    /// `bots`: the bots, their faction and continent. The factions whose squads changed.
    pub fn fill_bots(
        &mut self,
        human: impl Fn(PlayerId) -> Option<Option<ContinentId>>,
        bots: &[(PlayerId, Team, ContinentId)],
    ) -> Vec<Team> {
        let mut changed: Vec<Team> = Vec::new();
        let mark = |t: Team, changed: &mut Vec<Team>| {
            if !changed.contains(&t) {
                changed.push(t);
            }
        };
        let bot_at = |id: PlayerId| bots.iter().find(|b| b.0 == id).map(|b| b.2);
        // A human first; squads without any go.
        let before = self.list.len();
        self.list
            .retain(|s| s.members.iter().any(|&m| human(m).is_some()));
        if self.list.len() != before {
            // (Which faction's went isn't kept: tell all.)
            for t in Team::ALL {
                mark(t, &mut changed);
            }
        }
        for s in &mut self.list {
            if human(s.leader()).is_none()
                && let Some(i) = s.members.iter().position(|&m| human(m).is_some())
            {
                let h = s.members.remove(i);
                s.members.insert(0, h);
                s.waypoint = None;
                mark(s.team, &mut changed);
            }
            let at = human(s.leader()).flatten();
            let n = s.members.len();
            s.members
                .retain(|&m| human(m).is_some() || (bot_at(m).is_some() && bot_at(m) == at));
            if s.members.len() != n {
                mark(s.team, &mut changed);
            }
            // Room for one more human.
            while s.members.len() > SQUAD_MAX - 1 {
                let Some(i) = s.members.iter().rposition(|&m| human(m).is_none()) else {
                    break;
                };
                s.members.remove(i);
                mark(s.team, &mut changed);
            }
        }
        // Free bots in.
        let taken: Vec<PlayerId> = self.list.iter().flat_map(|s| s.members.clone()).collect();
        let mut free: Vec<&(PlayerId, Team, ContinentId)> =
            bots.iter().filter(|b| !taken.contains(&b.0)).collect();
        for s in &mut self.list {
            let Some(at) = human(s.leader()).flatten() else {
                continue;
            };
            while s.members.len() < SQUAD_MAX - 1 {
                let Some(i) = free.iter().position(|b| b.1 == s.team && b.2 == at) else {
                    break;
                };
                s.members.push(free.remove(i).0);
                mark(s.team, &mut changed);
            }
        }
        changed
    }

    pub fn all(&self) -> &[Squad] {
        &self.list
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn squads_are_made_joined_led_and_left() {
        let mut sq = Squads::default();
        assert_eq!(sq.apply(1, Team::Red, SquadCmd::Create, 9), Some(Team::Red));
        let id = sq.of(1).unwrap().id;
        assert_eq!(sq.of(1).unwrap().leader(), 1);
        // Joined by its faction only, up to the limit.
        assert_eq!(sq.apply(9, Team::Blue, SquadCmd::Join(id), 9), None);
        for p in 2..=SQUAD_MAX as u16 {
            assert!(sq.apply(p, Team::Red, SquadCmd::Join(id), 9).is_some());
        }
        assert_eq!(sq.of(1).unwrap().members.len(), SQUAD_MAX);
        assert_eq!(sq.apply(50, Team::Red, SquadCmd::Join(id), 9), None);
        // Only the leader sets the waypoint, on a continent that exists.
        let w = Some((4, Vec3::new(10.0, 2.0, -30.0)));
        assert_eq!(sq.apply(2, Team::Red, SquadCmd::Waypoint(w), 9), None);
        assert!(sq.apply(1, Team::Red, SquadCmd::Waypoint(w), 9).is_some());
        assert_eq!(sq.of(3).unwrap().waypoint, w);
        sq.apply(1, Team::Red, SquadCmd::Waypoint(Some((40, Vec3::ZERO))), 9);
        assert_eq!(sq.of(3).unwrap().waypoint, None);
        // Kicking and promoting are the leader's.
        assert_eq!(sq.apply(2, Team::Red, SquadCmd::Kick(3), 9), None);
        assert!(sq.apply(1, Team::Red, SquadCmd::Kick(3), 9).is_some());
        assert!(sq.of(3).is_none());
        assert!(sq.apply(1, Team::Red, SquadCmd::Promote(4), 9).is_some());
        assert_eq!(sq.of(1).unwrap().leader(), 4);
        // The leader leaving hands the lead on; the last one out ends the squad.
        sq.apply(4, Team::Red, SquadCmd::Leave, 9);
        assert_eq!(sq.of(1).unwrap().leader(), 1);
        for p in 1..=SQUAD_MAX as u16 {
            sq.leave(p);
        }
        assert!(sq.info(Team::Red).is_empty());
    }

    #[test]
    fn bots_fill_squads_leaving_room_for_a_human() {
        let mut sq = Squads::default();
        sq.apply(1, Team::Red, SquadCmd::Create, 9);
        let id = sq.of(1).unwrap().id;
        // Humans 1 (on continent 4) and 2 (on 5); bots of both factions on 4 and 5.
        let human = |p: PlayerId| match p {
            1 => Some(Some(4)),
            2 => Some(Some(5)),
            _ => None,
        };
        let mut bots: Vec<(PlayerId, Team, ContinentId)> =
            (100..120).map(|b| (b, Team::Red, 4)).collect();
        bots.push((200, Team::Blue, 4));
        bots.push((300, Team::Red, 5));
        assert_eq!(sq.fill_bots(human, &bots), vec![Team::Red]);
        let m = &sq.of(1).unwrap().members;
        assert_eq!(m.len(), SQUAD_MAX - 1);
        assert!(m.iter().all(|&b| b == 1 || (100..120).contains(&b)));
        // A human joins: a bot makes room again.
        assert!(sq.apply(2, Team::Red, SquadCmd::Join(id), 9).is_some());
        sq.fill_bots(human, &bots);
        let m = &sq.of(1).unwrap().members;
        assert_eq!(m.len(), SQUAD_MAX - 1);
        assert!(m.contains(&2));
        // A bot gone from the game leaves the squad (another takes its place).
        let gone = m[2];
        bots.retain(|b| b.0 != gone);
        sq.fill_bots(human, &bots);
        assert!(!sq.of(1).unwrap().members.contains(&gone));
        // The human leader leaves: the other human leads; both gone, the squad goes.
        sq.leave(1);
        sq.fill_bots(human, &bots);
        assert_eq!(sq.of(2).unwrap().leader(), 2);
        sq.leave(2);
        sq.fill_bots(human, &bots);
        assert!(sq.info(Team::Red).is_empty());
    }

    #[test]
    fn joining_or_creating_leaves_the_old_squad() {
        let mut sq = Squads::default();
        sq.apply(1, Team::Green, SquadCmd::Create, 9);
        sq.apply(2, Team::Green, SquadCmd::Create, 9);
        let b = sq.of(2).unwrap().id;
        sq.apply(1, Team::Green, SquadCmd::Join(b), 9);
        // 1's own squad went with it; 1 is in 2's.
        assert_eq!(sq.info(Team::Green).len(), 1);
        assert_eq!(sq.of(1).unwrap().members, vec![2, 1]);
        sq.apply(1, Team::Green, SquadCmd::Create, 9);
        assert_eq!(sq.info(Team::Green).len(), 2);
        assert_eq!(sq.of(2).unwrap().members, vec![2]);
    }
}
