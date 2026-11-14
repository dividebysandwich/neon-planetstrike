//! The flat screen's feel (from Neon Lasertag): the view bobs while walking, the gun sways
//! (a figure of eight walking, lagging behind the look turning) and kicks when fired, and the
//! view is jolted: shaken by hits and explosions, dipped on landing. All of it only moves what
//! is drawn: the aim stays on the crosshair. `Settings::head_bob` turns it all off.

use std::f32::consts::{E, PI, TAU};

use bevy::prelude::*;
use ps_sim::SimEvent;
use ps_sim::weapons::{Ammo, Weapon};

use crate::desktop::{Look, MainCamera};
use crate::fx::{Boom, FxSet, LocalShots};
use crate::game::{DrawSet, Game, GameEvent, LocalCmd, Mover, PredictSet};
use crate::settings::Settings;

/// How far a step carries (m), and how much of a step's bob a step makes (a full one a step
/// felt jittery).
const STRIDE: f32 = 0.8;
const BOB_RATE: f32 = 0.3;
/// How stiff the kick's, the gun's and the dip's springs are (rad/s; critically damped, so they
/// peak after 1/this s and are settled after about five times that).
const KICK_SPRING: f32 = 40.0;
const DIP_SPRING: f32 = 22.0;
/// The most a shake turns the view (rad), and how long a full one lasts (s).
const SHAKE: f32 = 0.025;
const SHAKE_TIME: f32 = 0.45;
/// Landing slower than this (m/s) doesn't dip the view.
const LAND_SPEED: f32 = 3.0;

pub struct ViewKickPlugin;

impl Plugin for ViewKickPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Sway {
            // (A fixed test view isn't jolted.)
            fixed_view: std::env::var("PS_VIEW").is_ok(),
            ..default()
        })
        .add_systems(
            Update,
            (update_sway, jolt_camera)
                .chain()
                .after(PredictSet)
                .after(FxSet)
                .before(DrawSet)
                .run_if(resource_exists::<Game>),
        );
    }
}

/// Head bob and gun sway, worked out each frame from the movement and the look.
#[derive(Resource, Default)]
pub struct Sway {
    /// Walking: 0 standing still, 1 at full speed (eased).
    amount: f32,
    /// Advances by half a turn a step, times `BOB_RATE`.
    phase: f32,
    /// Turning speed (rad/s: yaw, pitch), eased; and the look last frame.
    turn: Vec2,
    last_look: Option<Vec2>,
    /// The camera's offset (turned with the yaw only) and turn (in view space).
    camera: Vec3,
    view: Quat,
    /// The gun's offset and turn in view space (on foot).
    gun_offset: Vec3,
    gun_rot: Quat,
    /// The gun raised (m) to the camera's eye while the crouch eases.
    gun_lift: f32,
    /// On foot, alive, first person: the camera and gun are moved.
    pub on_foot: bool,
    fixed_view: bool,
    punch: Punch,
}

impl Sway {
    /// The gun in the hands moved (relative to the feet) and turned (about the grip, in the
    /// world) from where the look holds it: with the camera's bob and dip, swaying and kicking.
    pub fn gun_shift(&self, yaw: f32, pitch: f32) -> (Vec3, Quat) {
        let view = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch);
        let at = Quat::from_rotation_y(yaw) * self.camera
            + Vec3::Y * self.gun_lift
            + view * self.gun_offset;
        (at, view * self.gun_rot * view.inverse())
    }

    /// Eases the walking and turning, and gives the gun's offset and turn in view space
    /// (`turned`: the view's yaw and pitch change this frame).
    fn update(&mut self, dt: f32, turned: Vec2, speed: f32, max_speed: f32) -> (Vec3, Quat) {
        let k = |tau: f32| 1.0 - (-dt / tau).exp();
        self.amount += ((speed / max_speed.max(0.1)).min(1.0) - self.amount) * k(0.15);
        self.phase = (self.phase + speed * dt / STRIDE * BOB_RATE * PI) % TAU;
        self.turn = self.turn.lerp(
            (turned / dt).clamp(Vec2::splat(-12.0), Vec2::splat(12.0)),
            k(0.07),
        );
        let (a, p) = (self.amount, self.phase);
        // The head dips at each step and sways a little sideways every other.
        self.camera = Vec3::new(p.sin() * 0.012, (2.0 * p).cos() * 0.016, 0.0) * a;
        // The gun: a figure of eight while walking, lagging behind when the view turns.
        let walk = Vec3::new(p.sin() * 0.008, (2.0 * p).cos() * 0.006 - 0.004, 0.0) * a;
        let t = self.turn;
        let lag =
            Vec3::new(t.x * 0.006, -t.y * 0.006, 0.0).clamp(Vec3::splat(-0.03), Vec3::splat(0.03));
        let rot = Quat::from_euler(
            EulerRot::YXZ,
            (-t.x * 0.025).clamp(-0.1, 0.1),
            (-t.y * 0.025).clamp(-0.08, 0.08),
            (-t.x * 0.02).clamp(-0.08, 0.08) + p.sin() * 0.015 * a,
        );
        (walk + lag, rot)
    }
}

/// The view's jolts: a kick when firing, a shake when hit or near a blast and a dip on landing.
#[derive(Default)]
struct Punch {
    /// The view's kick (rad: pitch up, yaw) and its rate, springing back.
    kick: Vec2,
    kick_vel: Vec2,
    /// The gun pushed back towards the eye (m), and its rate.
    gun: f32,
    gun_vel: f32,
    /// The eyes' dip (m, down), and its rate.
    dip: f32,
    dip_vel: f32,
    /// How shaken (0 to 1), wearing off.
    trauma: f32,
    time: f32,
}

/// A critically damped spring's step (exact, at any `dt`).
fn spring(x: &mut f32, v: &mut f32, w: f32, dt: f32) {
    let (e, c) = ((-w * dt).exp(), *v + w * *x);
    *x = (*x + c * dt) * e;
    *v = (*v - w * c * dt) * e;
}

impl Punch {
    /// Kicks the view up by about `deg` at its peak (and a little sideways), and the gun back.
    fn fire(&mut self, deg: f32) {
        let peak = deg.to_radians();
        // (Peaking at `impulse / (w e)`.)
        let impulse = peak * KICK_SPRING * E;
        let side = (self.time * 91.7).sin() * 0.35;
        self.kick_vel += Vec2::new(impulse, impulse * side);
        self.gun_vel += deg * 0.02 * KICK_SPRING * E;
    }

    fn shake(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).min(1.0);
    }

    /// A dip as deep as the fall was fast (at most 8 cm).
    fn land(&mut self, speed: f32) {
        if speed > LAND_SPEED {
            let depth = (speed * 0.012).min(0.08);
            self.dip_vel += depth * DIP_SPRING * E;
            self.shake(((speed - LAND_SPEED) * 0.04).min(0.5));
        }
    }

    fn step(&mut self, dt: f32) {
        self.time += dt;
        spring(&mut self.kick.x, &mut self.kick_vel.x, KICK_SPRING, dt);
        spring(&mut self.kick.y, &mut self.kick_vel.y, KICK_SPRING, dt);
        spring(&mut self.gun, &mut self.gun_vel, KICK_SPRING, dt);
        spring(&mut self.dip, &mut self.dip_vel, DIP_SPRING, dt);
        self.trauma = (self.trauma - dt / SHAKE_TIME).max(0.0);
    }

    /// The view's turn from it all (in view space).
    fn view(&self) -> Quat {
        let s = self.trauma * self.trauma * SHAKE;
        let t = self.time;
        let shake = Vec3::new(
            (t * 41.0).sin() + 0.5 * (t * 97.0 + 1.3).sin(),
            (t * 37.0 + 2.1).sin() + 0.5 * (t * 83.0).sin(),
            (t * 29.0 + 0.7).sin(),
        ) * s;
        Quat::from_euler(
            EulerRot::YXZ,
            self.kick.y + shake.y,
            self.kick.x + shake.x,
            shake.z,
        )
    }
}

/// How much a gun kicks the view (degrees at the peak).
fn kick_of(w: Weapon) -> f32 {
    match w {
        Weapon::Rifle | Weapon::Volt => 0.4,
        Weapon::Ember => 0.55,
        Weapon::Hive => 0.32,
        Weapon::Pistol => 0.5,
        Weapon::Striker | Weapon::Skyhook => 1.0,
        Weapon::Scatter => 1.2,
        Weapon::Lance => 1.5,
        Weapon::Medgun | Weapon::RepairGun => 0.0,
        Weapon::Pounder => 0.3,
        Weapon::Mauler => 1.3,
        Weapon::Burster => 0.6,
    }
}

#[allow(clippy::too_many_arguments)]
fn update_sway(
    real: Res<Time<Real>>,
    game: Res<Game>,
    look: Res<Look>,
    local: Res<LocalCmd>,
    mut mover: ResMut<Mover>,
    shots: Res<LocalShots>,
    mut booms: MessageReader<Boom>,
    mut events: MessageReader<GameEvent>,
    settings: Res<Settings>,
    cams: Query<&GlobalTransform, With<MainCamera>>,
    mut sway: ResMut<Sway>,
) {
    let dt = real.delta_secs().clamp(1e-4, 0.05);
    let sway = &mut *sway;
    let me = game.mine();
    let cam = cams.single().map_or(mover.body.feet, |c| c.translation());
    for GameEvent(e) in events.read() {
        if let SimEvent::Hurt { victim, amount, .. } = e
            && Some(*victim) == game.me
        {
            sway.punch.shake((amount / 40.0).clamp(0.2, 0.8));
        }
    }
    for b in booms.read() {
        let size = match (b.wreck, b.ammo) {
            (Some(_), _) => 6.0,
            (_, Some(a)) => a.ballistics().splash,
            _ => 0.0,
        };
        if size <= 0.0 {
            continue;
        }
        let reach = size * 6.0 + 6.0;
        let d = b.at.distance(cam);
        if d < reach {
            sway.punch
                .shake((size / 5.0).min(0.9) * (1.0 - d / reach).powi(2));
        }
    }
    for (ammo, _) in &shots.fired {
        match ammo {
            Ammo::Gun(w) => sway.punch.fire(kick_of(*w)),
            Ammo::LightShell => sway.punch.shake(0.3),
            Ammo::HeavyShell => sway.punch.shake(0.45),
            _ => {}
        }
    }
    if let Some(v) = mover.landed.take() {
        sway.punch.land(v);
    }
    sway.punch.step(dt);
    // The look's turn this frame (the yaw wrapped).
    let now = Vec2::new(look.yaw, look.pitch);
    let turned = sway.last_look.map_or(Vec2::ZERO, |l| {
        Vec2::new(ps_sim::wrap_angle(now.x - l.x), now.y - l.y)
    });
    sway.last_look = Some(now);
    sway.on_foot = mover.valid
        && game.alive()
        && game.ready()
        && game.seat().is_none()
        && me.is_some_and(|m| m.turret.is_none());
    let speed = if sway.on_foot && mover.body.on_ground() {
        mover.body.vel.length()
    } else {
        0.0
    };
    let max = me.map_or(ps_sim::player::WALK_SPEED, |m| {
        ps_sim::player::move_speed(&ps_sim::PlayerCmd::default(), m.loadout.armor.speed())
    });
    let (mut offset, mut rot) = sway.update(dt, turned, speed, max);
    let kick = sway.punch.gun;
    offset += Vec3::new(0.0, kick * 0.25, kick);
    rot *= Quat::from_rotation_x(kick * 2.0);
    sway.camera -= Vec3::Y * sway.punch.dip;
    sway.view = sway.punch.view();
    // The look's eye (the gun's) eases with crouching as the camera's does.
    let eased = ps_sim::player::EYE - 0.5 * look.crouch;
    let crouch_fix = eased - ps_sim::player::eye_height(local.cmd.crouch);
    if settings.head_bob {
        sway.gun_offset = offset;
        sway.gun_rot = rot;
    } else {
        sway.camera = Vec3::ZERO;
        sway.view = Quat::IDENTITY;
        sway.gun_offset = Vec3::ZERO;
        sway.gun_rot = Quat::IDENTITY;
    }
    sway.gun_lift = crouch_fix;
}

/// Jolts the camera `place_camera` placed: turned by the kick and the shake, and on foot moved
/// by the bob and the dip.
fn jolt_camera(
    game: Res<Game>,
    sway: Res<Sway>,
    look: Res<Look>,
    free: Option<Res<crate::freecam::FreeCam>>,
    mut cams: Query<&mut Transform, With<MainCamera>>,
) {
    if sway.fixed_view || free.is_some_and(|f| f.on) || !game.alive() || !game.ready() {
        return;
    }
    let Ok(mut cam) = cams.single_mut() else {
        return;
    };
    cam.rotation *= sway.view;
    if sway.on_foot {
        cam.translation += Quat::from_rotation_y(look.yaw) * sway.camera;
    }
}

/// Testing: `PS_FEEL_TEST=1` fakes being hit (from round about) and hitting (on the head, a
/// kill, a vehicle, plain, in turn) every second and a half, and a grenade going off ahead,
/// to see and feel the feedback.
pub fn feel_test(
    time: Res<Time>,
    game: Option<Res<Game>>,
    local: Res<LocalCmd>,
    mut events: MessageWriter<GameEvent>,
    mut fx: MessageWriter<crate::game::FxEvent>,
    mut next: Local<f32>,
    mut n: Local<u32>,
) {
    let Some(game) = game else { return };
    let (Some(me), Some(mine)) = (game.me, game.mine()) else {
        return;
    };
    let now = time.elapsed_secs();
    if !game.alive() || now < *next {
        return;
    }
    *next = now + 1.5;
    *n += 1;
    let a = *n as f32 * 2.1;
    events.write(GameEvent(SimEvent::Hurt {
        victim: me,
        by: me,
        amount: 18.0,
        from: mine.feet + Vec3::new(a.sin() * 20.0, 1.0, a.cos() * 20.0),
    }));
    let ahead = ps_sim::look_dir(local.cmd.yaw, 0.0) * 3.5;
    fx.write(crate::game::FxEvent(ps_sim::snapshot::Fx::Impact {
        id: 0,
        ammo: Ammo::Grenade,
        at: mine.feet + ahead + Vec3::Y * 0.3,
        what: ps_sim::snapshot::ImpactOn::Ground,
    }));
    events.write(GameEvent(SimEvent::HitConfirm {
        by: me,
        vehicle: *n % 4 == 3,
        headshot: *n % 4 == 1,
        kill: *n % 4 == 2,
    }));
}
