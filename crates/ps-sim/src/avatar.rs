//! How the avatars' floating parts hang together (for drawing only; hits use
//! `player::hit_body`): the head on its neck on the chest.

use glam::{EulerRot, Quat, Vec3};

/// How far below the eyes the floating chest's top is, looking straight ahead.
pub const CHEST_DROP: f32 = 0.3;
/// Where the eyes are from the top of the neck, looking straight ahead.
const NECK_TO_EYE: Vec3 = Vec3::new(0.0, 0.1, -0.08);
/// The neck's length, from the chest's top up to the head.
const NECK: f32 = CHEST_DROP - NECK_TO_EYE.y;
/// How far the neck tilts (pitch and roll, radians); the head bends the rest at its top.
const NECK_BEND: f32 = 0.7;

/// The neck's rotation under a head turned to `rot`.
pub fn neck_rot(rot: Quat) -> Quat {
    let (yaw, pitch, roll) = rot.to_euler(EulerRot::YXZ);
    Quat::from_euler(
        EulerRot::YXZ,
        yaw,
        pitch.clamp(-NECK_BEND, NECK_BEND),
        roll.clamp(-NECK_BEND, NECK_BEND),
    )
}

/// The eyes of a head turned to `rot` on its neck over the body's middle: `level` is the
/// height of the eyes looking straight ahead.
pub fn eye_on_neck(level: Vec3, rot: Quat) -> Vec3 {
    let (yaw, pitch, _) = rot.to_euler(EulerRot::YXZ);
    let rot = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch);
    level - Vec3::Y * CHEST_DROP + neck_rot(rot) * Vec3::Y * NECK + rot * NECK_TO_EYE
}

/// The top of the neck under the eyes at `eye` of a head turned to `rot`.
pub fn neck_of(eye: Vec3, rot: Quat) -> Vec3 {
    eye - rot * NECK_TO_EYE
}

/// The top of the floating chest under the eyes at `eye` turned to `rot`.
pub fn chest_top(eye: Vec3, rot: Quat) -> Vec3 {
    neck_of(eye, rot) - neck_rot(rot) * Vec3::Y * NECK
}
