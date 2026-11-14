//! Types shared by the simulation, the client, the bots and the network protocol.

use glam::{Quat, Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// A player, unique across the whole world (the server hands them out).
pub type PlayerId = u16;
/// A vehicle, unique on its continent.
pub type VehicleId = u16;

/// What dealt damage when no player did (a fall, the map's edge).
pub const HAZARD: PlayerId = u16::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Team {
    Red,
    Blue,
    Green,
}

impl Team {
    pub const ALL: [Team; 3] = [Team::Red, Team::Blue, Team::Green];

    pub fn index(self) -> usize {
        match self {
            Team::Red => 0,
            Team::Blue => 1,
            Team::Green => 2,
        }
    }

    pub fn from_index(i: usize) -> Option<Team> {
        Team::ALL.get(i).copied()
    }

    pub fn name(self) -> &'static str {
        match self {
            Team::Red => "Red",
            Team::Blue => "Blue",
            Team::Green => "Green",
        }
    }

    pub fn upper(self) -> &'static str {
        match self {
            Team::Red => "RED",
            Team::Blue => "BLUE",
            Team::Green => "GREEN",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hand {
    Left,
    Right,
}

impl Hand {
    pub fn index(self) -> usize {
        match self {
            Hand::Left => 0,
            Hand::Right => 1,
        }
    }
}

/// A position and orientation. Forward is -Z (Bevy's convention).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pose {
    pub pos: Vec3,
    pub rot: Quat,
}

impl Default for Pose {
    fn default() -> Self {
        Pose {
            pos: Vec3::ZERO,
            rot: Quat::IDENTITY,
        }
    }
}

impl Pose {
    pub fn forward(&self) -> Vec3 {
        self.rot * Vec3::NEG_Z
    }
}

/// A VR player's head and hands relative to the feet, and the hand holding the gun with its
/// muzzle (shots leave from there).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VrInput {
    pub head: Pose,
    pub hands: [Pose; 2],
    pub gun_hand: Option<Hand>,
    pub muzzle: Pose,
}

/// Driving, flying or gunning a vehicle this tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DriveCmd {
    /// Forward/back (W/S), -1..1.
    pub throttle: f32,
    /// Turning (A/D on the ground, roll in the air), -1..1.
    pub steer: f32,
    /// Hover thrust (Space) and down (Ctrl), -1..1.
    pub lift: f32,
    /// The mouse's turn this tick (radians): yaw and pitch for aircraft.
    pub turn: Vec2,
}

/// One tick of a player's intent: the same for the local player (desktop or VR), a remote
/// player (from the network) and a bot. Positions are relative to the player's feet in world
/// orientation. Presses are counted (`jumps`, `uses`, ...) so a lost command loses none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerCmd {
    /// Locomotion in world XZ, length at most 1 (scaled by the move speed).
    pub stick: Vec2,
    /// Where the client's prediction has the feet: the sim moves there as far as its speed
    /// limit and the walls allow.
    pub feet: Option<Vec3>,
    /// Where the player looks (radians; yaw 0 = facing -Z, pitch up positive).
    pub yaw: f32,
    pub pitch: f32,
    /// VR: the head and hands (flat screen: none; drawn from the look).
    pub vr: Option<VrInput>,
    pub fire: bool,
    /// Aim down sights / zoom (the lance's scope), or the vehicle's second weapon.
    pub alt: bool,
    pub crouch: bool,
    pub sprint: bool,
    /// The weapon slot in hand (0 primary, 1 secondary, 2 tool).
    pub slot: u8,
    pub jumps: u32,
    /// The jump key held (light armour's jump jets, in the air).
    pub jet: bool,
    pub reloads: u32,
    /// Grenades thrown.
    pub throws: u32,
    /// The use key (enter or leave a vehicle, use a terminal or console).
    pub uses: u32,
    /// Vehicles: deploy (the spawn bus) and switching seats (to `seat`).
    pub deploys: u32,
    pub seat_switches: u32,
    pub seat: u8,
    pub drive: DriveCmd,
    /// The vehicle's pose as the driver's prediction has it: position, rotation, velocity.
    pub vehicle: Option<(Vec3, Quat, Vec3)>,
    /// The sim tick the client was showing when it made this command (lag compensation);
    /// 0: unknown.
    pub view_tick: u64,
}

impl PlayerCmd {
    /// The look direction.
    pub fn look(&self) -> Vec3 {
        look_dir(self.yaw, self.pitch)
    }
}

/// The unit vector looking at `yaw`, `pitch`.
pub fn look_dir(yaw: f32, pitch: f32) -> Vec3 {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    Vec3::new(-sy * cp, sp, -cy * cp)
}

/// The yaw of a direction (0 = -Z).
pub fn yaw_of(d: Vec3) -> f32 {
    (-d.x).atan2(-d.z)
}

/// The rotation looking along `dir`.
pub fn look_rot(dir: Vec3) -> Quat {
    let d = dir.normalize_or(Vec3::NEG_Z);
    let yaw = yaw_of(d);
    let pitch = d.y.clamp(-1.0, 1.0).asin();
    Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch)
}

/// Wraps an angle into -π..π.
pub fn wrap_angle(a: f32) -> f32 {
    (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn look_and_yaw_agree() {
        for yaw in [-2.0f32, -0.5, 0.0, 1.0, 3.0] {
            let d = look_dir(yaw, 0.3);
            assert!((wrap_angle(yaw_of(d) - yaw)).abs() < 1e-4);
            let q = look_rot(d);
            assert!((q * Vec3::NEG_Z - d).length() < 1e-4);
        }
    }
}
