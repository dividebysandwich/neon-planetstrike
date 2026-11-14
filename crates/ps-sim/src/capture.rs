//! Capture points (`Capture::Points` bases): three zones, A, B and C, each held by a faction.
//! Standing in a zone that isn't one's faction's takes it over `POINT_TIME` (faster with more
//! there, up to `MAX_RATE`); a rival's progress there drains first; two factions in it hold it
//! as it is (contested); its holders alone in it drain the takers' progress, and an empty
//! zone's progress fades slowly.
//!
//! A faction holding two of the three, and allowed to take the base (the lattice), captures
//! it when its capture (`POINT_CAPTURE_TIME`) runs out: the continent reports who holds the
//! majority (`ContinentSim::majority`) and the server runs the capture like a hack
//! (`WorldState::points_held`). Its owners taking a point back stops it.

use crate::types::Team;

/// Seconds one player takes to turn a point.
pub const POINT_TIME: f32 = 24.0;
/// How much faster a crowd turns one (at most).
pub const MAX_RATE: f32 = 2.5;
/// An empty point's progress fades this much slower than one player turns it.
pub const EMPTY_FADE: f32 = 0.25;
/// Seconds a capture (two points held) takes to take the base.
pub const POINT_CAPTURE_TIME: f32 = 150.0;
/// How far above or below a point's floor one still counts in it (not the storey above).
pub const POINT_DY: f32 = 3.5;

/// A point's state on the continent.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PointRt {
    pub owner: Option<Team>,
    /// Who is taking it, and how far they are (0..1).
    pub by: Option<Team>,
    pub progress: f32,
    /// Two factions in it: frozen.
    pub contested: bool,
}

/// How fast `n` players turn a point, relative to one.
pub fn rate(n: u8) -> f32 {
    (1.0 + 0.5 * (n.max(1) - 1) as f32).min(MAX_RATE)
}

impl PointRt {
    pub fn held_by(owner: Option<Team>) -> PointRt {
        PointRt {
            owner,
            ..Default::default()
        }
    }
}

/// Steps a point by `dt` with `present` players of each faction in it (only the factions
/// `eligible` count: the base's owners and those who may take it). The faction that took it,
/// if it just changed hands.
pub fn step_point(
    pt: &mut PointRt,
    present: [u8; 3],
    eligible: [bool; 3],
    dt: f32,
) -> Option<Team> {
    let here: Vec<Team> = Team::ALL
        .into_iter()
        .filter(|t| present[t.index()] > 0 && eligible[t.index()])
        .collect();
    pt.contested = here.len() > 1;
    if pt.contested {
        return None;
    }
    let fade = |pt: &mut PointRt, by: f32| {
        pt.progress -= by;
        if pt.progress <= 0.0 {
            pt.progress = 0.0;
            pt.by = None;
        }
    };
    let Some(&t) = here.first() else {
        fade(pt, EMPTY_FADE * dt / POINT_TIME);
        return None;
    };
    let step = rate(present[t.index()]) * dt / POINT_TIME;
    if pt.owner == Some(t) {
        fade(pt, step);
        return None;
    }
    if pt.by.is_some_and(|b| b != t) {
        fade(pt, step);
        return None;
    }
    pt.by = Some(t);
    pt.progress += step;
    if pt.progress >= 1.0 {
        *pt = PointRt::held_by(Some(t));
        return Some(t);
    }
    None
}

/// The faction holding two or more of the points.
pub fn majority(points: &[PointRt]) -> Option<Team> {
    Team::ALL
        .into_iter()
        .find(|&t| points.iter().filter(|p| p.owner == Some(t)).count() * 2 > points.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 30.0;

    fn time_to_take(pt: &mut PointRt, present: [u8; 3]) -> Option<f32> {
        let all = [true; 3];
        for i in 0..100_000 {
            if step_point(pt, present, all, DT).is_some() {
                return Some(i as f32 * DT);
            }
        }
        None
    }

    #[test]
    fn points_turn_faster_with_more_and_drain_first() {
        let red = [1, 0, 0];
        let mut p = PointRt::held_by(Some(Team::Blue));
        let one = time_to_take(&mut p, red).unwrap();
        assert!((one - POINT_TIME).abs() < 0.2, "{one}");
        assert_eq!(p.owner, Some(Team::Red));
        let mut p = PointRt::held_by(Some(Team::Blue));
        let two = time_to_take(&mut p, [2, 0, 0]).unwrap();
        assert!((two - POINT_TIME / 1.5).abs() < 0.2, "{two}");
        let mut p = PointRt::held_by(None);
        let five = time_to_take(&mut p, [5, 0, 0]).unwrap();
        assert!((five - POINT_TIME / MAX_RATE).abs() < 0.2, "{five}");
        // Halfway by Red, then Green: Red's half drains before Green fills it.
        let mut p = PointRt::held_by(Some(Team::Blue));
        let all = [true; 3];
        for _ in 0..(POINT_TIME / 2.0 / DT) as usize {
            step_point(&mut p, red, all, DT);
        }
        assert_eq!(p.by, Some(Team::Red));
        let green = time_to_take(&mut p, [0, 0, 1]).unwrap();
        assert!((green - POINT_TIME * 1.5).abs() < 0.3, "{green}");
        assert_eq!(p.owner, Some(Team::Green));
    }

    #[test]
    fn contested_points_hold_and_empty_ones_fade() {
        let all = [true; 3];
        let mut p = PointRt::held_by(Some(Team::Blue));
        for _ in 0..300 {
            step_point(&mut p, [1, 0, 0], all, DT);
        }
        let half = p.progress;
        assert!(half > 0.3);
        for _ in 0..3000 {
            assert_eq!(step_point(&mut p, [1, 1, 0], all, DT), None);
        }
        assert!(p.contested && p.progress == half);
        // Someone not allowed to take it doesn't count: no contest.
        step_point(&mut p, [1, 0, 3], [true, true, false], DT);
        assert!(!p.contested && p.progress > half);
        // The owners alone drain it.
        for _ in 0..600 {
            step_point(&mut p, [0, 1, 0], all, DT);
        }
        assert_eq!((p.by, p.progress, p.owner), (None, 0.0, Some(Team::Blue)));
        // Left alone, progress fades slowly.
        for _ in 0..300 {
            step_point(&mut p, [1, 0, 0], all, DT);
        }
        let was = p.progress;
        for _ in 0..300 {
            step_point(&mut p, [0, 0, 0], all, DT);
        }
        assert!(
            p.progress < was && p.progress > was * 0.6,
            "{was} {}",
            p.progress
        );
    }

    #[test]
    fn two_of_three_is_a_majority() {
        let p = |t: Option<Team>| PointRt::held_by(t);
        let r = Some(Team::Red);
        assert_eq!(majority(&[p(r), p(r), p(None)]), r);
        assert_eq!(
            majority(&[p(r), p(Some(Team::Blue)), p(Some(Team::Green))]),
            None
        );
        assert_eq!(majority(&[p(None), p(None), p(r)]), None);
    }
}
