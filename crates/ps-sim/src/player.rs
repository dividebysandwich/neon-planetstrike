//! Infantry: how a body moves (the sim and the client's prediction step it the same way) and
//! where it can be hit.

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use crate::collide::{BODY_RADIUS, World};
use crate::types::PlayerCmd;

pub const WALK_SPEED: f32 = 5.2;
pub const SPRINT_FACTOR: f32 = 1.55;
pub const CROUCH_FACTOR: f32 = 0.45;
pub const JUMP_SPEED: f32 = 5.2;
pub const EYE: f32 = 1.65;
pub const CROUCH_EYE: f32 = 1.15;
/// Movement inertia (time constants, s): getting up to speed, and stopping.
pub const ACCEL_TIME: f32 = 0.12;
pub const STOP_TIME: f32 = 0.09;
/// How long a jump press waits for the ground, and how long after leaving it one still jumps.
pub const JUMP_BUFFER: f32 = 0.15;
pub const COYOTE: f32 = 0.12;

/// A walking body: feet, vertical speed, horizontal velocity and the jump bookkeeping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Body {
    pub feet: Vec3,
    pub vy: f32,
    pub vel: Vec2,
    pub jumps_seen: Option<u32>,
    pub jump_wait: f32,
    pub off_ground: f32,
    pub jumped: bool,
    /// The fastest fall since leaving the ground (m/s, positive), for fall damage.
    pub fall: f32,
    /// Jump jets (light armour): may it use them, and the fuel left (0..1).
    pub can_jet: bool,
    pub fuel: f32,
    /// Jetting this step.
    pub jetting: bool,
    /// Falling under a chute (bailed out of an aircraft): slowly, until it lands.
    pub chute: bool,
}

/// Jump jets: the push up (m/s²), the fastest climb, seconds of thrust from full, how fast the
/// fuel fills again on the ground (per second), and how long after leaving the ground they
/// start (the jump itself first).
pub const JET_ACCEL: f32 = 22.0;
pub const JET_CLIMB: f32 = 6.0;
pub const JET_TIME: f32 = 1.4;
pub const JET_REFILL: f32 = 0.35;
pub const JET_AFTER: f32 = 0.15;
/// The fastest a chute falls (m/s).
pub const CHUTE_FALL: f32 = 5.0;

/// The eye height of a standing or crouching body.
pub fn eye_height(crouch: bool) -> f32 {
    if crouch { CROUCH_EYE } else { EYE }
}

/// Whether a command sprints: forward along the look, not crouching.
pub fn sprinting(cmd: &PlayerCmd) -> bool {
    if !cmd.sprint || cmd.crouch || cmd.stick.length() < 0.5 {
        return false;
    }
    let f = Vec2::new(-cmd.yaw.sin(), -cmd.yaw.cos());
    cmd.stick.normalize().dot(f) > 0.6
}

/// The speed a command walks at with `armor_speed` (the armor's factor).
pub fn move_speed(cmd: &PlayerCmd, armor_speed: f32) -> f32 {
    let k = if cmd.crouch {
        CROUCH_FACTOR
    } else if sprinting(cmd) {
        SPRINT_FACTOR
    } else {
        1.0
    };
    WALK_SPEED * armor_speed * k
}

impl Body {
    pub fn at(feet: Vec3) -> Body {
        Body {
            feet,
            fuel: 1.0,
            ..Default::default()
        }
    }

    /// In the air: the jump jets' push (the jump key held, fuel left) and a chute's brake on
    /// the vertical speed `vy`; on the ground the fuel fills again. (The server follows a
    /// client's steps with this too.)
    pub fn air(&mut self, cmd: &PlayerCmd, vy: f32, dt: f32) -> f32 {
        let mut vy = vy;
        self.jetting = false;
        if self.on_ground() {
            self.fuel = (self.fuel + JET_REFILL * dt).min(1.0);
        } else if self.can_jet && cmd.jet && self.fuel > 0.0 && self.off_ground > JET_AFTER {
            vy = (vy + JET_ACCEL * dt).min(JET_CLIMB.max(vy));
            self.fuel = (self.fuel - dt / JET_TIME).max(0.0);
            self.jetting = true;
            // (A climb starts the fall's count again.)
            self.fall = 0.0;
        }
        if self.chute {
            vy = vy.max(-CHUTE_FALL);
        }
        vy
    }

    pub fn on_ground(&self) -> bool {
        self.vy == 0.0
    }

    /// One step of `dt` walking by `cmd` at `speed`. Returns the fall speed if it just landed.
    pub fn step(&mut self, world: &World, cmd: &PlayerCmd, speed: f32, dt: f32) -> Option<f32> {
        let want = cmd.stick.clamp_length_max(1.0) * speed;
        let tau = if want.length_squared() >= self.vel.length_squared() {
            ACCEL_TIME
        } else {
            STOP_TIME
        };
        // In the air, little control.
        let control = if self.on_ground() { 1.0 } else { 0.15 };
        self.vel = self.vel.lerp(want, (1.0 - (-dt / tau).exp()) * control);
        if self.vel.length() < 0.01 && want == Vec2::ZERO {
            self.vel = Vec2::ZERO;
        }
        // Jumps: a press waits a moment for the ground.
        if let Some(seen) = self.jumps_seen
            && cmd.jumps != seen
        {
            self.jump_wait = JUMP_BUFFER;
        }
        self.jumps_seen = Some(cmd.jumps);
        if self.on_ground() {
            self.off_ground = 0.0;
            self.jumped = false;
        } else {
            self.off_ground += dt;
        }
        let mut vy = self.vy;
        if self.jump_wait > 0.0 {
            self.jump_wait -= dt;
            if !self.jumped && self.off_ground <= COYOTE && !cmd.crouch {
                self.jump_wait = 0.0;
                self.jumped = true;
                vy = JUMP_SPEED;
            }
        }
        let vy = self.air(cmd, vy, dt);
        let before = self.feet;
        let (feet, vy2) = world.move_body(self.feet, self.vel * dt, vy, dt);
        // Walking into a wall stops the movement into it.
        if dt > 0.0 && self.on_ground() {
            let moved = Vec2::new(feet.x - before.x, feet.z - before.z) / dt;
            if moved.length() < self.vel.length() * 0.5 {
                self.vel = self.vel.lerp(moved, 0.5);
            }
        }
        self.feet = crate::map::ContinentMap::clamp(feet);
        let landed = vy2 == 0.0 && self.vy != 0.0;
        if vy2 < 0.0 {
            self.fall = self.fall.max(-vy2);
        }
        self.vy = vy2;
        if landed {
            self.chute = false;
            return Some(std::mem::take(&mut self.fall));
        }
        if vy2 == 0.0 {
            self.fall = 0.0;
        }
        None
    }
}

/// Hit volumes: the head (a sphere) and the body (a capsule from the knees to the chest).
pub const HEAD_RADIUS: f32 = 0.24;
pub const BODY_HIT_RADIUS: f32 = BODY_RADIUS + 0.04;

/// Distance along the ray `o` + t·`d` (`d` unit length) to a sphere, within `len`.
fn ray_sphere(o: Vec3, d: Vec3, c: Vec3, r: f32, len: f32) -> Option<f32> {
    let m = o - c;
    let b = m.dot(d);
    let cc = m.length_squared() - r * r;
    if cc > 0.0 && b > 0.0 {
        return None;
    }
    let disc = b * b - cc;
    if disc < 0.0 {
        return None;
    }
    let t = (-b - disc.sqrt()).max(0.0);
    (t <= len).then_some(t)
}

/// The first hit of the segment `a`..`b` on a body with its feet at `feet` (crouching or
/// not): the fraction along the segment, and whether it was the head.
pub fn hit_body(feet: Vec3, crouch: bool, a: Vec3, b: Vec3) -> Option<(f32, bool)> {
    let len = (b - a).length();
    if len < 1e-6 {
        return None;
    }
    let d = (b - a) / len;
    let eye = feet + Vec3::Y * eye_height(crouch);
    // Quick reject.
    let mid = feet + Vec3::Y * 0.9;
    let m = a - mid;
    let t = (-m.dot(d)).clamp(0.0, len);
    if (m + d * t).length_squared() > 1.6 * 1.6 {
        return None;
    }
    let head = ray_sphere(a, d, eye + Vec3::Y * 0.05, HEAD_RADIUS, len);
    // The capsule: closest approach of the ray to its axis, sampled as spheres along it.
    let (lo, hi) = (feet.y + 0.3, eye.y - 0.3);
    let mut body: Option<f32> = None;
    let n = 5;
    for i in 0..n {
        let y = lo + (hi - lo) * i as f32 / (n - 1) as f32;
        if let Some(t) = ray_sphere(a, d, Vec3::new(feet.x, y, feet.z), BODY_HIT_RADIUS, len)
            && body.is_none_or(|b| t < b)
        {
            body = Some(t);
        }
    }
    match (head, body) {
        (Some(h), Some(b)) if h <= b => Some((h / len, true)),
        (Some(h), None) => Some((h / len, true)),
        (_, Some(b)) => Some((b / len, false)),
        (None, None) => None,
    }
}

/// Where a body's shots leave from on a flat screen: the eye (the crosshair's line).
pub fn eye_of(feet: Vec3, crouch: bool) -> Vec3 {
    feet + Vec3::Y * eye_height(crouch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collide::Collider;

    #[test]
    fn heads_and_bodies_are_hit() {
        let feet = Vec3::new(0.0, 0.0, 0.0);
        let (f, head) = hit_body(
            feet,
            false,
            Vec3::new(0.0, EYE + 0.05, 10.0),
            Vec3::new(0.0, EYE + 0.05, -10.0),
        )
        .unwrap();
        assert!(head && (f - 0.5).abs() < 0.03, "{f}");
        let (_, head) = hit_body(
            feet,
            false,
            Vec3::new(0.0, 1.0, 10.0),
            Vec3::new(0.0, 1.0, -10.0),
        )
        .unwrap();
        assert!(!head);
        assert!(
            hit_body(
                feet,
                false,
                Vec3::new(2.0, 1.0, 10.0),
                Vec3::new(2.0, 1.0, -10.0)
            )
            .is_none()
        );
    }

    #[test]
    fn bodies_walk_jump_and_land() {
        let floor = Collider::Box {
            min: Vec3::new(-100.0, -1.0, -100.0),
            max: Vec3::new(100.0, 0.0, 100.0),
        };
        let w = World::new(vec![floor], None);
        let mut b = Body::at(Vec3::ZERO);
        let mut cmd = PlayerCmd {
            stick: Vec2::new(0.0, -1.0),
            ..Default::default()
        };
        for _ in 0..60 {
            b.step(&w, &cmd, WALK_SPEED, 1.0 / 60.0);
        }
        assert!(b.feet.z < -4.0 && b.feet.z > -5.3, "{}", b.feet);
        cmd.jumps = 1;
        let mut peak: f32 = 0.0;
        let mut landed = None;
        for _ in 0..120 {
            if let Some(f) = b.step(&w, &cmd, WALK_SPEED, 1.0 / 60.0) {
                landed = Some(f);
            }
            peak = peak.max(b.feet.y);
        }
        assert!(peak > 1.0 && peak < 1.6, "{peak}");
        assert!(landed.is_some_and(|f| f > 4.0 && f < 6.0), "{landed:?}");
    }
}
