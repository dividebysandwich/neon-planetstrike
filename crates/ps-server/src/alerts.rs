//! Alerts: now and then (`PS_ALERT_EVERY` seconds, 30 minutes by default) one of the
//! contested continents (the busiest) gets an alert for `PS_ALERT_LENGTH` seconds (15 minutes):
//! whichever faction holds the most of its bases (outposts and the center) when the time is up
//! wins it, and its players earn `ALERT_XP` (those on the continent) or `ALERT_XP_AWAY`.

use std::time::{Duration, Instant};

use ps_net::protocol::AlertInfo;
use ps_sim::worldgen::BaseKind;
use ps_sim::{BaseIdx, ContinentId, Team, WorldDef, WorldState};

pub struct Alerts {
    pub active: Option<(ContinentId, Instant)>,
    next: Instant,
    every: Duration,
    length: Duration,
}

/// What happened to the alerts this tick.
#[derive(Debug, PartialEq)]
pub enum AlertEvent {
    Started(ContinentId),
    Ended(ContinentId, Option<Team>),
}

fn secs(var: &str, default: u64) -> Duration {
    Duration::from_secs(
        std::env::var(var)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default),
    )
}

impl Alerts {
    pub fn new(now: Instant) -> Alerts {
        let every = secs("PS_ALERT_EVERY", 1800);
        Alerts {
            active: None,
            // (The first a third of the way in.)
            next: now + secs("PS_ALERT_FIRST", every.as_secs() / 3),
            every,
            length: secs("PS_ALERT_LENGTH", 900),
        }
    }

    pub fn info(&self, now: Instant) -> Option<AlertInfo> {
        self.active.map(|(continent, ends)| AlertInfo {
            continent,
            left: ends.saturating_duration_since(now).as_secs() as u32,
        })
    }

    /// Starts or ends an alert when it's time (`population`: everyone on each continent).
    pub fn tick(
        &mut self,
        now: Instant,
        def: &WorldDef,
        state: &WorldState,
        population: &[[u16; 3]],
    ) -> Option<AlertEvent> {
        if let Some((c, ends)) = self.active {
            if now < ends {
                return None;
            }
            self.active = None;
            self.next = now + self.every;
            return Some(AlertEvent::Ended(c, winner(holdings(def, state, c))));
        }
        if now < self.next {
            return None;
        }
        let busiest = (0..def.continents.len() as ContinentId)
            .filter(|&c| state.contested(def, c))
            .max_by_key(|&c| {
                population
                    .get(c as usize)
                    .map_or(0, |p| p.iter().map(|&n| n as u32).sum::<u32>())
            });
        match busiest {
            Some(c) => {
                self.active = Some((c, now + self.length));
                Some(AlertEvent::Started(c))
            }
            None => {
                // (Nothing contested: look again in a minute.)
                self.next = now + Duration::from_secs(60);
                None
            }
        }
    }
}

/// The outposts and center each faction holds on continent `c`.
pub fn holdings(def: &WorldDef, state: &WorldState, c: ContinentId) -> [u16; 3] {
    let mut out = [0u16; 3];
    for (b, site) in def.continents[c as usize].bases.iter().enumerate() {
        if !matches!(site.kind, BaseKind::Outpost | BaseKind::Center) {
            continue;
        }
        if let Some(t) = state.base(c, b as BaseIdx).owner {
            out[t.index()] += 1;
        }
    }
    out
}

/// The faction holding the most (none if two tie at the top, or nobody holds any).
pub fn winner(h: [u16; 3]) -> Option<Team> {
    let top = *h.iter().max()?;
    let mut leaders = Team::ALL.into_iter().filter(|t| h[t.index()] == top);
    let first = leaders.next()?;
    (top > 0 && leaders.next().is_none()).then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps_sim::WorldParams;

    #[test]
    fn the_most_bases_win_and_ties_draw() {
        assert_eq!(winner([3, 5, 1]), Some(Team::Blue));
        assert_eq!(winner([4, 4, 1]), None);
        assert_eq!(winner([0, 0, 0]), None);
    }

    #[test]
    fn alerts_start_on_a_contested_continent_and_end_in_time() {
        let def = WorldDef::generate(WorldParams {
            seed: 11,
            contested: 6,
        });
        let state = WorldState::initial(&def);
        let t0 = Instant::now();
        let mut a = Alerts {
            active: None,
            next: t0,
            every: Duration::from_secs(100),
            length: Duration::from_secs(10),
        };
        let pop = vec![[1, 1, 1]; def.continents.len()];
        let Some(AlertEvent::Started(c)) = a.tick(t0, &def, &state, &pop) else {
            panic!("no alert");
        };
        assert!(state.contested(&def, c));
        assert_eq!(a.info(t0).unwrap().left, 10);
        assert_eq!(
            a.tick(t0 + Duration::from_secs(5), &def, &state, &pop),
            None
        );
        let end = a.tick(t0 + Duration::from_secs(11), &def, &state, &pop);
        assert_eq!(
            end,
            Some(AlertEvent::Ended(c, winner(holdings(&def, &state, c))))
        );
        assert!(a.active.is_none());
    }
}
