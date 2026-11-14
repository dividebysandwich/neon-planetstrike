//! Playing in VR, whichever renderer drives the headset (bevy_mod_openxr in src/xr.rs, or the
//! GL renderer in src/glvr): raw controller and head input in `XrInput` becomes the local
//! player's command, and the tracking space is placed where the player is.
//!
//! On foot: the left stick walks (where the head looks), the right stick turns. The weapons
//! are holstered on the body (src/holster.rs): the rifle slot over the gun side's shoulder,
//! the sidearm on the gun side's hip, the special on the other hip (the gun side is the right,
//! the left with `--left-handed` or the setting). Gripping near one draws it, letting go puts
//! it back; the drawn gun is turned nose down in the hand by the settings' gun tilt and its
//! trigger fires. A jumps, B reloads, X throws a grenade, Y (tapped) spots the enemy looked at
//! (seated too; held, the command wheel: src/wheel.rs), the right stick pulled back cloaks (an infiltrator) or puts the engineer's kit
//! down, the other hand's trigger uses (enter
//! vehicles, terminals, consoles; their panels: src/vrui.rs).
//!
//! In a vehicle or a wall turret the player sits in its seat, the view turning with it
//! (src/seatview.rs: with a tank's or a turret's barrel, a bus's or a harvester's hull, the
//! VTOL's cockpit): the left stick drives, the right stick aims the gun (a tank's, a turret's;
//! the trigger fires), X deploys the bus, the left trigger gets out. The VTOL: the right stick
//! pitches and yaws, the left stick speeds up and slows down (and rolls), the right grip (or
//! A) climbs and the left grip (or B) sinks. The right stick's click, held, fires a seat's
//! second gun (the interceptor's missiles, once locked) and, on foot, locks the Striker on.

use bevy::prelude::*;
use ps_sim::{Hand, Pose, VrInput, look_rot};

use crate::desktop::UsePressed;
use crate::game::{Game, InputSet, LocalCmd, Mover, PredictSet};
use crate::settings::Settings;

/// How long the backends vibrate for each frame's amplitude: a little over a frame.
pub const HAPTIC_NANOS: i64 = 22_000_000;
/// The Steam Frame's controllers' interaction profile.
pub const FRAME_EXT: &[u8] = b"XR_VALVE_frame_controller_interaction\0";
pub const FRAME_PROFILE: &str = "/interaction_profiles/valve/frame_controller";

/// One controller as the runtime reports it (poses in the tracking space).
#[derive(Clone, Copy, Debug, Default)]
pub struct HandInput {
    pub grip: Option<Transform>,
    pub aim: Option<Transform>,
    pub squeeze: f32,
    pub trigger: f32,
    /// A / X
    pub primary: bool,
    /// B / Y
    pub secondary: bool,
    pub menu: bool,
    pub stick: Vec2,
    pub stick_click: bool,
}

/// Filled in by the XR backend before `InputSet`; `haptic` is read back after `Update`.
#[derive(Resource, Default)]
pub struct XrInput {
    pub hands: [HandInput; 2],
    pub head: Option<Transform>,
    pub haptic: [f32; 2],
}

/// The entity whose transform places the tracking space in the world.
#[derive(Resource, Clone, Copy)]
pub struct XrRig(pub Entity);

#[derive(Resource, Default)]
pub struct VrState {
    pub yaw: f32,
    pub rig: Transform,
    held: [[bool; 3]; 2],
    trigger_held: bool,
    snapped: bool,
    /// The body's facing in the tracking space (yaw), easing after the head's: the holsters
    /// turn with it.
    body_yaw: Option<f32>,
    /// The gun drawn: the hand holding it (0 left, 1 right) and its holster (the slot).
    pub drawn: Option<(usize, u8)>,
    /// Each hand: the holster within its reach, and gripping (with hysteresis).
    pub near: [Option<u8>; 2],
    gripping: [bool; 2],
    /// The holsters in the tracking space, and in the world (`place_rig`).
    holsters: [Option<Vec3>; 3],
    pub holsters_world: [Option<Transform>; 3],
    /// Seated at a gun one aims: which seat (a vehicle's, or a turret's) and the aim the right
    /// stick turns (world yaw, pitch).
    aim: Option<(Seat, f32, f32)>,
    /// How long Y has been held (none: up). A tap spots, on letting go; held longer it opens
    /// the command wheel (src/vrui.rs).
    spot_held: Option<f32>,
    /// The wheel open: where the left stick points (picking a slot; it moves nothing meanwhile).
    pub wheel_stick: Vec2,
    /// The wheel let go of (Y up after a hold): the left stick then (centred: nothing picked).
    pub wheel_pick: Option<Vec2>,
    wheel_was_open: bool,
    /// The right stick pulled back (on foot: the deploy key), with hysteresis.
    pulled: bool,
}

/// Y held at least this long (s) isn't a tap: it opens the command wheel instead of spotting.
pub const SPOT_TAP: f32 = 0.35;

impl VrState {
    /// The command wheel is up (Y held past a tap).
    pub fn wheel_open(&self) -> bool {
        self.spot_held.is_some_and(|t| t >= SPOT_TAP)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Seat {
    Vehicle(ps_sim::VehicleId, u8),
    Turret(u16),
}

/// How fast the right stick turns a gun (rad/s): across and up and down.
const AIM_YAW_RATE: f32 = 1.8;
const AIM_PITCH_RATE: f32 = 1.0;

/// The renderer can't draw the comfort vignette (src/glvr).
#[derive(Resource)]
pub struct NoVignette;

#[derive(Component)]
pub struct Vignette;

pub struct VrPlugin;

impl Plugin for VrPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<XrInput>()
            .init_resource::<VrState>()
            .init_resource::<Rumble>()
            .add_message::<UsePressed>()
            .add_plugins(crate::vrui::VrUiPlugin)
            .add_systems(
                Update,
                read_controls
                    .in_set(InputSet)
                    .run_if(resource_exists::<Game>),
            )
            .add_systems(
                Update,
                place_rig
                    .after(PredictSet)
                    .after(crate::seatview::SeatViewSet)
                    .run_if(resource_exists::<Game>),
            )
            .add_systems(
                Update,
                (haptics.run_if(resource_exists::<Game>), play_rumble)
                    .chain()
                    .after(place_rig),
            )
            .add_plugins(crate::holster::HolsterPlugin);
    }
}

#[allow(clippy::too_many_arguments)]
fn read_controls(
    input: Res<XrInput>,
    mut vr: ResMut<VrState>,
    mut local: ResMut<LocalCmd>,
    game: Res<Game>,
    settings: Res<Settings>,
    args: Res<crate::Args>,
    time: Res<Time>,
    mut uses: MessageWriter<UsePressed>,
    mut spots: MessageWriter<crate::game::SpotPressed>,
    view: Res<crate::seatview::SeatView>,
    mut rumble: ResMut<Rumble>,
) {
    let dt = time.delta_secs();
    let gun = gun_side(&args, &settings);
    // (The use trigger is the hand without the gun drawn.)
    let off = vr.drawn.map_or(1 - gun, |(h, _)| 1 - h);
    let (l, r) = (input.hands[0], input.hands[1]);
    let pressed = |h: usize, k: usize| match k {
        0 => input.hands[h].primary,
        1 => input.hands[h].secondary,
        _ => input.hands[h].stick_click,
    };
    let mut edge = [[false; 3]; 2];
    #[allow(clippy::needless_range_loop)]
    for h in 0..2 {
        for k in 0..3 {
            let now = pressed(h, k);
            edge[h][k] = now && !vr.held[h][k];
            vr.held[h][k] = now;
        }
    }
    let use_now = input.hands[off].trigger > 0.6;
    if use_now && !vr.trigger_held {
        uses.write(UsePressed);
    }
    vr.trigger_held = use_now;
    // Y, tapped: spot what one looks at (on foot and seated alike; let go within `SPOT_TAP`).
    match (input.hands[0].secondary, vr.spot_held) {
        (true, held) => vr.spot_held = Some(held.unwrap_or(0.0) + dt),
        (false, Some(held)) => {
            vr.spot_held = None;
            if held < SPOT_TAP {
                spots.write(crate::game::SpotPressed);
                rumble.play(0, 0.0, 0.02, 0.5);
            } else {
                vr.wheel_pick = Some(vr.wheel_stick);
            }
        }
        (false, None) => {}
    }
    // The wheel up: the left stick picks on it (and doesn't walk or drive meanwhile).
    let wheel = vr.wheel_open();
    if wheel && !vr.wheel_was_open {
        // (A tick as it opens.)
        rumble.play(0, 0.0, 0.02, 0.3);
    }
    vr.wheel_was_open = wheel;
    vr.wheel_stick = if wheel { l.stick } else { Vec2::ZERO };
    let l = if wheel {
        HandInput {
            stick: Vec2::ZERO,
            stick_click: false,
            ..l
        }
    } else {
        l
    };
    local.active = true;
    let head_yaw = input
        .head
        .map_or(0.0, |h| ps_sim::yaw_of(h.rotation * Vec3::NEG_Z))
        + vr.yaw;
    let seat = game.seat();
    let turret = game.mine().and_then(|m| m.turret);
    let seated = seat.is_some() || turret.is_some();
    let flying =
        seat.is_some_and(|(v, s)| s == 0 && game.vehicle(v).is_some_and(|v| v.kind.aircraft()));
    // Turning (the right stick), snapped or smooth by the settings (seated, the view turns
    // with the seat, src/seatview.rs).
    if !seated {
        let x = r.stick.x;
        if settings.snap_turn > 0.0 {
            if x.abs() > 0.7 && !vr.snapped {
                vr.yaw -= x.signum() * settings.snap_turn.to_radians();
                vr.snapped = true;
            } else if x.abs() < 0.3 {
                vr.snapped = false;
            }
        } else if x.abs() > 0.15 {
            vr.yaw -= x * 2.5 * dt;
        }
    }
    // Seated at a gun one aims (a tank's, a turret's): the right stick turns it.
    let aims = match (seat, turret) {
        (_, Some(t)) => Some(Seat::Turret(t)),
        (Some((v, s)), None) => game.vehicle(v).and_then(|veh| {
            let spec = veh.kind.spec();
            let gi = spec.seats.get(s as usize)?.gun?;
            (!spec.guns[gi].fixed && veh.kind != ps_sim::vehicle::VehicleKind::Harvester)
                .then_some(Seat::Vehicle(v, s))
        }),
        _ => None,
    };
    match aims {
        Some(key) => {
            let (mut yaw, mut pitch) = match vr.aim {
                Some((k, y, p)) if k == key => (y, p),
                // (Taking the seat: from where the gun points.)
                _ if view.active => {
                    let d = view.rot * Vec3::NEG_Z;
                    (ps_sim::yaw_of(d), d.y.clamp(-1.0, 1.0).asin())
                }
                _ => (head_yaw, 0.0),
            };
            let dead = |x: f32| if x.abs() < 0.15 { 0.0 } else { x };
            let inv = if settings.invert_vehicle_y { -1.0 } else { 1.0 };
            yaw = ps_sim::wrap_angle(yaw - dead(r.stick.x) * AIM_YAW_RATE * dt);
            pitch = (pitch + dead(r.stick.y) * inv * AIM_PITCH_RATE * dt).clamp(-0.9, 1.3);
            vr.aim = Some((key, yaw, pitch));
        }
        None => vr.aim = None,
    }
    let cmd = &mut local.cmd;
    // The look: the head's (at a gun: the stick's aim).
    if let Some((_, yaw, pitch)) = vr.aim {
        cmd.yaw = yaw;
        cmd.pitch = pitch;
    } else if let Some(h) = input.head {
        let rig_rot = if seated {
            view.level
        } else {
            Quat::from_rotation_y(vr.yaw)
        };
        let d = rig_rot * h.rotation * Vec3::NEG_Z;
        cmd.yaw = ps_sim::yaw_of(d);
        cmd.pitch = d.y.clamp(-1.0, 1.0).asin();
    }
    // Seated: the gun side's trigger fires the seat's gun; on foot, the drawn gun's.
    cmd.fire = if seated {
        input.hands[gun].trigger > 0.6
    } else {
        vr.drawn.is_some_and(|(h, _)| input.hands[h].trigger > 0.6)
    };
    // The right stick, clicked and held (the left's click is the map's): a seat's second gun
    // (an interceptor's missiles); on foot, the Striker's lock-on.
    cmd.alt = r.stick_click && (seated || vr.drawn.is_some());
    match seat {
        None if turret.is_some() => {
            cmd.stick = Vec2::ZERO;
            cmd.drive = default();
        }
        None => {
            let (s, c) = head_yaw.sin_cos();
            let fwd = Vec2::new(-s, -c);
            let right = Vec2::new(c, -s);
            cmd.stick = (fwd * l.stick.y + right * l.stick.x).clamp_length_max(1.0);
            cmd.sprint = l.stick_click || l.stick.length() > 0.95;
            if edge[1][0] {
                cmd.jumps += 1;
            }
            // (A held: light armour's jump jets.)
            cmd.jet = r.primary;
            if edge[1][1] {
                cmd.reloads += 1;
            }
            if edge[0][0] {
                cmd.throws += 1;
            }
            // The right stick pulled back (not turning): the infiltrator's cloak, the
            // engineer's deployable.
            let back = r.stick.y < -0.75 && r.stick.x.abs() < 0.5;
            if back && !vr.pulled {
                cmd.deploys += 1;
            }
            vr.pulled = if vr.pulled { r.stick.y < -0.4 } else { back };
            cmd.drive = default();
        }
        Some((_, s)) => {
            cmd.stick = Vec2::ZERO;
            if s == 0 {
                cmd.drive.throttle = l.stick.y;
                cmd.drive.steer = l.stick.x;
                cmd.drive.lift = 0.0;
                if flying {
                    // The right stick pitches and yaws; the grips (or A / B) climb and sink.
                    let dead = |x: f32| if x.abs() < 0.12 { 0.0 } else { x };
                    let inv = if settings.invert_flight_y { -1.0 } else { 1.0 };
                    cmd.drive.turn +=
                        Vec2::new(-dead(r.stick.x) * 1.3, dead(r.stick.y) * inv * 1.0) * dt;
                    // (The left stick across rolls only gently.)
                    cmd.drive.steer = dead(l.stick.x) * 0.6;
                    cmd.drive.lift = if r.squeeze > 0.6 || r.primary {
                        1.0
                    } else if l.squeeze > 0.6 || r.secondary {
                        -1.0
                    } else {
                        0.0
                    };
                }
            }
            if edge[0][0] {
                cmd.deploys += 1;
            }
        }
    }
    holsters(
        &input,
        &mut vr,
        &mut local.cmd,
        &mut rumble,
        &settings,
        gun,
        dt,
        seated || !game.alive(),
    );
    // (A MAX has only its arms, whichever holster it drew from.)
    if game
        .mine()
        .is_some_and(|m| !m.loadout.usable(local.cmd.slot))
    {
        local.cmd.slot = 0;
    }
}

/// Where each holster sits (tracking space, from the eyes along the body's facing; x towards
/// the gun side): the rifle slot over the shoulder, the sidearm on the gun side's hip, the
/// special on the other. `PS_HOLSTER_<slot>=x,y,z` moves one (testing, tuning).
fn holster_offset(slot: usize) -> Vec3 {
    static ENV: std::sync::OnceLock<[Option<Vec3>; 3]> = std::sync::OnceLock::new();
    let env = ENV.get_or_init(|| {
        std::array::from_fn(|i| {
            let v = std::env::var(format!("PS_HOLSTER_{i}")).ok()?;
            let n: Vec<f32> = v.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            (n.len() == 3).then(|| Vec3::new(n[0], n[1], n[2]))
        })
    });
    env[slot].unwrap_or(HOLSTERS[slot])
}

/// The holsters' default places (see `holster_offset`).
const HOLSTERS: [Vec3; 3] = [
    Vec3::new(0.17, -0.12, 0.14),
    Vec3::new(0.23, -0.58, -0.07),
    Vec3::new(-0.23, -0.58, -0.07),
];
/// How near a hand has to be to a holster to draw from it (m): the shoulder's a reach back.
const HOLSTER_REACH: [f32; 3] = [0.22, 0.2, 0.2];

/// The hand the gun side is (0 left, 1 right).
pub fn gun_side(args: &crate::Args, settings: &Settings) -> usize {
    if args.left_handed || settings.left_handed {
        0
    } else {
        1
    }
}

/// Moves angle `a` towards `b` by `k` (0..1) of the way.
fn approach_angle(a: f32, b: f32, k: f32) -> f32 {
    a + ps_sim::wrap_angle(b - a) * k
}

/// The yaw a head faces, flattened (looking straight down: where the top of the head points).
fn flat_yaw(rot: Quat) -> f32 {
    let f = rot * Vec3::NEG_Z;
    let d = if Vec2::new(f.x, f.z).length() > 0.3 {
        f
    } else {
        rot * Vec3::Y * -f.y.signum()
    };
    ps_sim::yaw_of(d)
}

/// The holsters following the body, and the hands drawing from them and letting go.
#[allow(clippy::too_many_arguments)]
fn holsters(
    input: &XrInput,
    vr: &mut VrState,
    cmd: &mut ps_sim::PlayerCmd,
    rumble: &mut Rumble,
    settings: &Settings,
    gun: usize,
    dt: f32,
    away: bool,
) {
    if let Some(h) = input.head {
        let head = flat_yaw(h.rotation);
        let body = vr
            .body_yaw
            .map_or(head, |b| approach_angle(b, head, 1.0 - (-dt * 3.0).exp()));
        vr.body_yaw = Some(body);
        let side = if gun == 0 { -1.0 } else { 1.0 };
        let turn = Quat::from_rotation_y(body);
        for (n, slot) in vr.holsters.iter_mut().enumerate() {
            let at = holster_offset(n);
            let dy = if n == 0 { 0.0 } else { settings.holster_dy };
            *slot = Some(
                h.translation + turn * Vec3::new(at.x * side, 0.0, at.z) + Vec3::Y * (at.y + dy),
            );
        }
    }
    // Seated, dead: nothing in hand, nothing to draw.
    if away {
        vr.drawn = None;
        vr.near = [None; 2];
        return;
    }
    // Letting go: back in its holster.
    if let Some((h, _)) = vr.drawn
        && input.hands[h].squeeze < 0.4
    {
        vr.drawn = None;
        rumble.play(h, 0.0, 0.035, 0.85);
    }
    for h in [gun, 1 - gun] {
        let hand = &input.hands[h];
        let holding = vr.drawn.is_some_and(|(d, _)| d == h);
        // The nearest holster in reach (not the one the other hand has emptied).
        let near = hand.grip.and_then(|g| {
            (0..3u8)
                .filter(|&n| vr.drawn.is_none_or(|(_, s)| s != n))
                .filter_map(|n| {
                    let d = g.translation.distance(vr.holsters[n as usize]?);
                    (d < HOLSTER_REACH[n as usize]).then_some((n, d))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(n, _)| n)
        });
        let near = near.filter(|_| !holding);
        if near.is_some() && near != vr.near[h] {
            rumble.play(h, 0.0, 0.015, 0.25);
        }
        let gripping = hand.squeeze > 0.6;
        if let Some(n) = near
            && gripping
            && !vr.gripping[h]
        {
            // (A gun in the other hand goes back: one at a time.)
            if let Some((other, _)) = vr.drawn {
                rumble.play(other, 0.0, 0.02, 0.4);
            }
            vr.drawn = Some((h, n));
            cmd.slot = n;
            rumble.play(h, 0.0, 0.035, 0.85);
        }
        vr.near[h] = near;
        if hand.squeeze < 0.4 || gripping {
            vr.gripping[h] = gripping;
        }
    }
}

/// Places the tracking space so the head is over the player's feet (or in its seat), and
/// fills in the head and hands for the others to see.
#[allow(clippy::too_many_arguments)]
pub(crate) fn place_rig(
    input: Res<XrInput>,
    mut vr: ResMut<VrState>,
    mut local: ResMut<LocalCmd>,
    mover: Res<Mover>,
    game: Res<Game>,
    models: Res<crate::assets::Models>,
    rig: Option<Res<XrRig>>,
    args: Res<crate::Args>,
    settings: Res<Settings>,
    view: Res<crate::seatview::SeatView>,
    mut transforms: Query<&mut Transform>,
) {
    let head = input
        .head
        .unwrap_or(Transform::from_xyz(0.0, ps_sim::player::EYE, 0.0));
    let h = head.translation;
    // Seated (a vehicle, a turret): the seat's eyes are where the head is, and the tracking
    // space turns with the seat's view (src/seatview.rs).
    let seated = view.active.then_some(view.eye);
    let rot = if view.active {
        view.level
    } else {
        Quat::from_rotation_y(vr.yaw)
    };
    let feet = if mover.valid {
        mover.body.feet
    } else {
        game.mine().map_or(Vec3::ZERO, |m| m.feet)
    };
    // Dead (or not spawned yet): over where one fell, or the continent's first base, to look
    // round while choosing a spawn point.
    let feet = if game.alive() {
        feet
    } else {
        let at = if feet.length_squared() > 1.0 {
            feet
        } else {
            game.map
                .as_ref()
                .and_then(|m| m.bases.first())
                .map_or(feet, |b| b.center)
        };
        at + Vec3::Y * 12.0
    };
    vr.rig = match seated {
        Some(eye) => Transform::from_translation(eye - rot * h).with_rotation(rot),
        None => {
            Transform::from_translation(feet - rot * Vec3::new(h.x, 0.0, h.z)).with_rotation(rot)
        }
    };
    if let Some(r) = rig
        && let Ok(mut t) = transforms.get_mut(r.0)
    {
        *t = vr.rig;
    }
    let rig_t = vr.rig;
    let world = |t: &Transform| rig_t * *t;
    let head_w = world(&head);
    // The gun in the hand that drew it (seated: the gun side's, for the seat's gun).
    let drawn = if seated.is_some() {
        Some((gun_side(&args, &settings), local.cmd.slot))
    } else {
        vr.drawn
    };
    let mut v = VrInput {
        head: Pose {
            pos: head_w.translation - feet,
            rot: head_w.rotation,
        },
        gun_hand: drawn.map(|(h, _)| if h == 0 { Hand::Left } else { Hand::Right }),
        ..default()
    };
    for (i, hand) in input.hands.iter().enumerate() {
        let Some(grip) = hand.grip else { continue };
        let g = world(&grip);
        let rot = hand.aim.map_or(g.rotation, |a| world(&a).rotation);
        v.hands[i] = Pose {
            pos: g.translation - feet,
            rot,
        };
    }
    // The gun held with its handle more upright in the hand: turned nose down about the grip
    // from the controller's aim (it fires along its barrel), as in Neon Lasertag.
    if let Some((gun, slot)) = drawn {
        v.hands[gun].rot *= Quat::from_rotation_x(-gun_tilt(&settings));
        let p = v.hands[gun];
        let w = game
            .mine()
            .map_or(ps_sim::weapons::Weapon::Rifle, |m| m.loadout.slot(slot));
        v.muzzle = Pose {
            pos: p.pos + p.rot * crate::actors::muzzle_of(&models, w),
            rot: look_rot(p.rot * Vec3::NEG_Z),
        };
    }
    // The holsters in the world: hip ones muzzle down, tipped forward; the shoulder's muzzle
    // up behind the shoulder, slanting across the back.
    let side = if gun_side(&args, &settings) == 0 {
        -1.0
    } else {
        1.0
    };
    let body = vr.body_yaw;
    vr.holsters_world = std::array::from_fn(|n| {
        let (at, body) = (vr.holsters[n]?, body?);
        let rot = if n == 0 {
            Quat::from_rotation_y(body)
                * Quat::from_rotation_z(-side * 0.5)
                * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 - 0.25)
        } else {
            Quat::from_rotation_y(body) * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2 + 0.3)
        };
        Some(rig_t * Transform::from_translation(at).with_rotation(rot))
    });
    local.cmd.vr = Some(v);
    local.cmd.crouch =
        head_w.translation.y - feet.y < 0.75 * ps_sim::player::EYE && seated.is_none();
}

/// How far the gun is turned nose down from the controller's aim (radians): `PS_GUN_TILT`
/// (degrees), else the settings' (`Settings::gun_tilt`, 40 by default).
fn gun_tilt(settings: &Settings) -> f32 {
    static ENV: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    ENV.get_or_init(|| {
        std::env::var("PS_GUN_TILT")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
    })
    .unwrap_or(settings.gun_tilt)
    .to_radians()
}

/// Vibration patterns being played: (hand, wait, length, amplitude, fading, played).
#[derive(Resource, Default)]
pub struct Rumble {
    pulses: Vec<(usize, f32, f32, f32, bool, f32)>,
}

impl Rumble {
    /// A pulse on `hand` (0 left, 1 right) after `wait` seconds.
    pub fn play(&mut self, hand: usize, wait: f32, secs: f32, amplitude: f32) {
        self.pulses.push((hand, wait, secs, amplitude, false, 0.0));
    }

    /// A pulse fading out.
    pub fn fade(&mut self, hand: usize, wait: f32, secs: f32, amplitude: f32) {
        self.pulses.push((hand, wait, secs, amplitude, true, 0.0));
    }
}

/// Buzzes on hits taken (harder in the hand on the side they came from) and given (in the gun
/// hand: harder on the head, a double tap on a kill).
#[allow(clippy::too_many_arguments)]
fn haptics(
    mut rumble: ResMut<Rumble>,
    mut events: MessageReader<crate::game::GameEvent>,
    game: Res<Game>,
    vr: Res<VrState>,
    local: Res<LocalCmd>,
    mover: Res<Mover>,
    args: Res<crate::Args>,
    settings: Res<Settings>,
) {
    let gun = vr
        .drawn
        .map_or_else(|| gun_side(&args, &settings), |(h, _)| h);
    for e in events.read() {
        match e.0 {
            ps_sim::SimEvent::Hurt {
                victim,
                amount,
                from,
                ..
            } if Some(victim) == game.me => {
                // (From the left: positive.)
                let head = local
                    .cmd
                    .vr
                    .map_or(local.cmd.yaw, |v| ps_sim::yaw_of(v.head.rot * Vec3::NEG_Z));
                let d = from - mover.body.feet;
                let s = if Vec2::new(d.x, d.z).length() > 1.5 {
                    ps_sim::wrap_angle(ps_sim::yaw_of(d) - head).sin()
                } else {
                    0.0
                };
                let a = (0.45 + amount / 60.0).min(0.9);
                rumble.fade(0, 0.0, 0.18, a * (0.6 + 0.4 * s));
                rumble.fade(1, 0.0, 0.18, a * (0.6 - 0.4 * s));
            }
            ps_sim::SimEvent::HitConfirm {
                by, headshot, kill, ..
            } if Some(by) == game.me => {
                rumble.play(gun, 0.0, 0.03, if headshot { 0.55 } else { 0.3 });
                if kill {
                    rumble.play(gun, 0.09, 0.05, 0.8);
                }
            }
            _ => {}
        }
    }
}

/// This frame's amplitude per hand: the strongest pulse playing (times the settings').
fn play_rumble(
    mut rumble: ResMut<Rumble>,
    mut input: ResMut<XrInput>,
    time: Res<Time>,
    settings: Res<Settings>,
) {
    let dt = time.delta_secs();
    let mut amp = [0.0f32; 2];
    for (hand, wait, secs, a, fade, played) in rumble.pulses.iter_mut() {
        if *wait > 0.0 {
            *wait -= dt;
            continue;
        }
        let k = if *fade {
            (1.0 - *played / *secs).max(0.0)
        } else {
            1.0
        };
        amp[*hand & 1] = amp[*hand & 1].max(*a * k);
        *played += dt;
    }
    rumble.pulses.retain(|p| p.1 > 0.0 || p.5 < p.2);
    for (h, a) in input.haptic.iter_mut().zip(amp) {
        *h = h.max((a * settings.haptics).min(1.0));
    }
}
