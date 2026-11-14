//! Progress: experience (XP) for what a player does for its faction, Battle Rank from it, and
//! the certification points each rank brings. The continents award XP as things happen
//! (`ContinentSim::awards`, and `SimEvent::Xp` to the player); the server keeps each player's
//! total. Certification points are counted but don't lock anything yet.

use serde::{Deserialize, Serialize};

use crate::vehicle::VehicleKind;

/// What XP is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Award {
    Kill,
    /// On top of a kill, for a headshot.
    Headshot,
    /// Damage done to someone a team mate then killed.
    Assist,
    VehicleKill(VehicleKind),
    Revive,
    Heal,
    Repair,
    /// Solving an enemy base's console (starting a hack).
    Hack,
    /// Solving one's own base's console (stopping a hack), or taking one's own capture point
    /// back during a capture.
    Defend,
    /// A harvester's load into a silo.
    Delivery,
    /// Someone spawned on one's spawn bus, or on one as squad leader.
    Spawn,
    /// An enemy structure destroyed.
    Structure,
    /// One's structure built.
    Build,
    /// An enemy one spotted killed by a team mate.
    Spot,
    /// One's faction locked a continent one was on.
    Lock,
    /// One's faction won an alert (more for those on the continent).
    Alert,
    /// Taking a capture point (standing in it).
    Capture,
}

impl Award {
    pub fn name(self) -> &'static str {
        match self {
            Award::Kill => "KILL",
            Award::Headshot => "HEADSHOT",
            Award::Assist => "ASSIST",
            Award::VehicleKill(_) => "VEHICLE DESTROYED",
            Award::Revive => "REVIVE",
            Award::Heal => "HEAL",
            Award::Repair => "REPAIR",
            Award::Hack => "HACK",
            Award::Defend => "DEFEND",
            Award::Delivery => "DELIVERY",
            Award::Spawn => "SPAWN",
            Award::Structure => "STRUCTURE DESTROYED",
            Award::Build => "BUILT",
            Award::Spot => "SPOT BONUS",
            Award::Lock => "CONTINENT LOCKED",
            Award::Alert => "ALERT WON",
            Award::Capture => "POINT CAPTURED",
        }
    }
}

pub const KILL_XP: u32 = 100;
pub const HEADSHOT_XP: u32 = 25;
pub const ASSIST_XP: u32 = 50;
pub const REVIVE_XP: u32 = 75;
pub const HACK_XP: u32 = 250;
/// A capture point taken (for each one in it).
pub const CAPTURE_XP: u32 = 100;
pub const SPAWN_XP: u32 = 10;
pub const STRUCTURE_XP: u32 = 100;
pub const BUILD_XP: u32 = 25;
pub const SPOT_XP: u32 = 25;
pub const LOCK_XP: u32 = 500;
/// An alert won: for those on its continent at the end, and the rest of the faction.
pub const ALERT_XP: u32 = 1000;
pub const ALERT_XP_AWAY: u32 = 250;
/// Per point of health healed, of vehicle (turret, generator) health repaired, of resources
/// delivered.
pub const HEAL_XP: f32 = 0.5;
pub const REPAIR_XP: f32 = 1.0;
pub const DELIVERY_XP: f32 = 1.0;
/// Heal, repair and delivery XP is given in lumps of at least this.
pub const LUMP: f32 = 10.0;
/// Damage (health points) within `ASSIST_TICKS` that makes an assist.
pub const ASSIST_DAMAGE: f32 = 20.0;
pub const ASSIST_TICKS: u64 = 600;

pub fn vehicle_xp(k: VehicleKind) -> u32 {
    match k {
        VehicleKind::Rover => 75,
        VehicleKind::LightTank | VehicleKind::LightAaa | VehicleKind::Harvester => 150,
        VehicleKind::Interceptor => 200,
        VehicleKind::Dropship => 350,
        VehicleKind::Vtol => 200,
        VehicleKind::HeavyTank => 250,
        VehicleKind::SpawnBus => 300,
    }
}

pub const MAX_BR: u8 = 40;

/// XP a Battle Rank takes (in all): BR 2 at 1000, 5 at 10 000, 10 at 45 000, 20 at 190 000.
pub fn xp_for(br: u8) -> u64 {
    let n = br.clamp(1, MAX_BR) as u64;
    500 * (n - 1) * n
}

/// The Battle Rank `xp` makes.
pub fn battle_rank(xp: u64) -> u8 {
    (1..=MAX_BR).rev().find(|&b| xp >= xp_for(b)).unwrap_or(1)
}

/// How far (0..1) `xp` is from its rank to the next.
pub fn rank_progress(xp: u64) -> f32 {
    let br = battle_rank(xp);
    if br >= MAX_BR {
        return 1.0;
    }
    let (a, b) = (xp_for(br), xp_for(br + 1));
    (xp - a) as f32 / (b - a) as f32
}

/// Certification points at a Battle Rank.
pub fn cert_points(br: u8) -> u32 {
    3 + br as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_follow_xp() {
        assert_eq!(battle_rank(0), 1);
        assert_eq!(battle_rank(999), 1);
        assert_eq!(battle_rank(1000), 2);
        assert_eq!(battle_rank(10_000), 5);
        assert_eq!(battle_rank(u64::MAX / 2), MAX_BR);
        for br in 1..MAX_BR {
            assert!(xp_for(br + 1) > xp_for(br));
            assert_eq!(battle_rank(xp_for(br)), br);
        }
        assert!((rank_progress(2000) - 0.5).abs() < 1e-6);
        assert_eq!(cert_points(1), 4);
    }
}
