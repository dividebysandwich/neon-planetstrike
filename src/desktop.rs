//! Playing on a flat screen: mouse look, WASD, Shift sprints, Ctrl crouches, Space jumps, left
//! button fires, right button aims (the lance's scope), R reloads, G throws a grenade, 1/2
//! switch weapons, E uses (enter/leave vehicles, terminals, consoles: src/interact.rs), M the
//! map. In vehicles: W/S drive (the VTOL: speed up / slow down), A/D turn (the VTOL: roll),
//! the mouse aims the turret (the VTOL: yaw and pitch), Space/Ctrl hover up/down, B deploys
//! the spawn bus, F changes seats, V the camera. Esc opens the in-game menu; while an overlay
//! is up the mouse is free and the game ignores the input.

use bevy::camera::Hdr;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use ps_sim::vehicle::MotionKind;
use ps_sim::weapons::Weapon;
use ps_sim::{look_dir, wrap_angle};

use crate::game::{Game, InputSet, LocalCmd, Mover, PredictSet};
use crate::settings::{Action, Inputs, Settings, WheelState};

#[derive(Component)]
pub struct MainCamera;

#[derive(Resource, Default)]
pub struct Look {
    pub yaw: f32,
    pub pitch: f32,
    pub crouch: f32,
    pub zoom: f32,
    /// Vehicles: seen from behind (else from the seat).
    pub third_person: bool,
    /// The VTOL: the camera's smoothed orientation.
    cam_rot: Quat,
}

/// Overlays that take the mouse (and keep the game from the input).
#[derive(Resource, Default)]
pub struct Overlays {
    pub menu: bool,
    pub map: bool,
    pub terminal: bool,
    pub chat: bool,
    pub options: bool,
    /// The squad panel (src/squad.rs).
    pub squad: bool,
    /// The build panel, and placing a structure (src/build.rs: no firing meanwhile).
    pub build: bool,
    /// Logging in to the server (src/login.rs).
    pub login: bool,
    /// One's stats (src/stats.rs).
    pub stats: bool,
    pub placing: bool,
    /// The click that closed an overlay doesn't also fire.
    pub hold_fire: bool,
}

impl Overlays {
    pub fn any(&self) -> bool {
        self.menu
            || self.map
            || self.terminal
            || self.chat
            || self.options
            || self.squad
            || self.build
            || self.login
            || self.stats
    }
}

/// The use key was pressed (src/interact.rs decides what it does).
#[derive(Message, Clone, Copy, Debug)]
pub struct UsePressed;

pub struct DesktopPlugin;

impl Plugin for DesktopPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Look {
            // Testing: PS_INSIDE=1 starts in vehicles and turrets with the view from inside.
            third_person: std::env::var("PS_INSIDE").is_err(),
            ..default()
        })
        .init_resource::<Overlays>()
        .add_message::<UsePressed>()
        .add_systems(Startup, spawn_camera)
        .add_systems(Update, (grab_cursor, read_input.in_set(InputSet)).chain())
        .add_systems(Update, face_spawn.before(InputSet))
        .add_systems(
            Update,
            place_camera
                .after(PredictSet)
                .after(crate::seatview::SeatViewSet)
                .before(crate::fx::FxSet),
        )
        .add_systems(Update, apply_fov);
    }
}

pub fn camera_bundle() -> impl Bundle {
    (
        Camera3d::default(),
        Hdr,
        Tonemapping::AcesFitted,
        Bloom {
            intensity: 0.22,
            ..Bloom::NATURAL
        },
        Projection::Perspective(PerspectiveProjection {
            fov: 80f32.to_radians(),
            near: 0.05,
            far: 8000.0,
            ..default()
        }),
        Msaa::Sample4,
    )
}

fn spawn_camera(mut commands: Commands) {
    let mut cam = commands.spawn((
        MainCamera,
        camera_bundle(),
        Transform::from_xyz(0.0, 300.0, 600.0).looking_at(Vec3::ZERO, Vec3::Y),
        crate::audio::listener(),
    ));
    // (The soft particles read the depth: src/fx.rs, assets/shaders/particle.wgsl.)
    if !crate::WEBGL2 {
        cam.insert(bevy::core_pipeline::prepass::DepthPrepass);
    }
}

fn grab_cursor(
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut overlays: ResMut<Overlays>,
    game: Option<Res<Game>>,
) {
    let Ok(mut c) = cursor.single_mut() else {
        return;
    };
    let playing = game.as_ref().is_some_and(|g| g.alive());
    if overlays.any() || !playing {
        if c.grab_mode != CursorGrabMode::None {
            set_grab(&mut c, false);
        }
        overlays.hold_fire = mouse.pressed(MouseButton::Left);
        return;
    }
    if !mouse.pressed(MouseButton::Left) {
        overlays.hold_fire = false;
    }
    if c.grab_mode == CursorGrabMode::None {
        set_grab(&mut c, true);
        overlays.hold_fire = true;
    }
}

pub fn set_grab(c: &mut CursorOptions, grab: bool) {
    c.grab_mode = if grab {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    c.visible = !grab;
}

#[allow(clippy::too_many_arguments)]
fn read_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    settings: Res<Settings>,
    mut wheel: Local<WheelState>,
    mut look: ResMut<Look>,
    mut local: ResMut<LocalCmd>,
    overlays: Res<Overlays>,
    game: Option<Res<Game>>,
    mut uses: MessageWriter<UsePressed>,
    time: Res<Time>,
    mover: Res<Mover>,
) {
    let Some(game) = game else { return };
    let i = Inputs {
        keys: &keys,
        mouse: &mouse,
        wheel: *wheel,
    };
    *wheel = WheelState::default();
    let active = !overlays.any();
    local.active = active;
    let pressed = |a: Action| active && settings.pressed(a, &i);
    let just = |a: Action| active && settings.just_pressed(a, &i);
    let sens = settings.sensitivity * 0.0025;
    let delta = if active {
        motion.delta * sens
    } else {
        Vec2::ZERO
    };
    let seat = game.seat();
    let in_turret = game.mine().is_some_and(|m| m.turret.is_some());
    let vehicle = seat.and_then(|(v, _)| game.vehicle(v)).map(|v| v.kind);
    let flying = vehicle.is_some_and(|k| k.aircraft()) && seat.is_some_and(|s| s.1 == 0);
    let cmd = &mut local.cmd;
    if flying {
        // The mouse flies the craft; the look follows its nose.
        let inv = if settings.invert_flight_y { -1.0 } else { 1.0 };
        cmd.drive.turn += Vec2::new(-delta.x, -delta.y * inv);
    } else {
        // (Vehicles' guns and turrets have their own up and down.)
        let invert = if seat.is_some() || in_turret {
            settings.invert_vehicle_y
        } else {
            settings.invert_y
        };
        look.yaw = wrap_angle(look.yaw - delta.x);
        look.pitch = (look.pitch - delta.y * if invert { -1.0 } else { 1.0 }).clamp(-1.5, 1.5);
    }
    if flying && let Some(m) = drive_motion(&mover) {
        let f = m.rot * Vec3::NEG_Z;
        look.yaw = ps_sim::yaw_of(f);
        look.pitch = f.y.clamp(-1.0, 1.0).asin();
    }
    cmd.yaw = look.yaw;
    cmd.pitch = look.pitch;
    let mut axis = Vec2::ZERO;
    if pressed(Action::Forward) {
        axis.y += 1.0;
    }
    if pressed(Action::Back) {
        axis.y -= 1.0;
    }
    if pressed(Action::Left) {
        axis.x -= 1.0;
    }
    if pressed(Action::Right) {
        axis.x += 1.0;
    }
    // Testing: PS_FIRE=1 holds the trigger.
    let fire = (pressed(Action::Fire) && !overlays.hold_fire && !overlays.placing)
        || std::env::var("PS_FIRE").is_ok_and(|v| v == "1");
    if fire && settings.just_pressed(Action::Fire, &i) {
        local.fire_latch = true;
    }
    let cmd = &mut local.cmd;
    cmd.fire = fire;
    // Testing: PS_ALT=1 holds the aim / second-gun button once locked on.
    let locked = game.mine().and_then(|m| m.lock).is_some_and(|l| l.1 >= 1.0);
    cmd.alt = pressed(Action::Aim) || (locked && std::env::var("PS_ALT").is_ok_and(|v| v == "1"));
    match seat {
        None => {
            let (s, c) = cmd.yaw.sin_cos();
            let fwd = Vec2::new(-s, -c);
            let right = Vec2::new(c, -s);
            cmd.stick = (fwd * axis.y + right * axis.x).clamp_length_max(1.0);
            cmd.crouch = pressed(Action::Crouch);
            cmd.sprint = pressed(Action::Sprint);
            cmd.drive = default();
            if just(Action::Jump) {
                cmd.jumps += 1;
            }
            // (Held: light armour's jump jets.)
            cmd.jet = pressed(Action::Jump);
            // On foot: the infiltrator's cloak, the engineer's deployable.
            if just(Action::Deploy) {
                cmd.deploys += 1;
            }
        }
        Some((_, s)) => {
            cmd.stick = Vec2::ZERO;
            cmd.crouch = false;
            cmd.sprint = false;
            if s == 0 {
                // Testing: PS_DEV_DRIVE=throttle,steer,lift drives by itself.
                if let Ok(v) = std::env::var("PS_DEV_DRIVE") {
                    let n: Vec<f32> = v.split(',').filter_map(|t| t.parse().ok()).collect();
                    axis = Vec2::new(
                        n.get(1).copied().unwrap_or(0.0),
                        n.first().copied().unwrap_or(0.0),
                    );
                    cmd.drive.lift = n.get(2).copied().unwrap_or(0.0);
                }
                cmd.drive.throttle = axis.y;
                cmd.drive.steer = axis.x;
                cmd.drive.lift = if std::env::var("PS_DEV_DRIVE").is_ok() {
                    cmd.drive.lift
                } else if pressed(Action::Jump) {
                    1.0
                } else if pressed(Action::Crouch) {
                    -1.0
                } else {
                    0.0
                };
            } else {
                cmd.drive = default();
            }
            if just(Action::Deploy) {
                cmd.deploys += 1;
            }
            if just(Action::Seat)
                && let Some((vid, s)) = seat
                && let Some(v) = game.vehicle(vid)
            {
                let n = v.seats.len();
                if let Some(next) = (1..n)
                    .map(|k| (s as usize + k) % n)
                    .find(|&k| v.seats[k].is_none())
                {
                    cmd.seat = next as u8;
                    cmd.seat_switches += 1;
                }
            }
        }
    }
    if just(Action::View) && (seat.is_some() || in_turret) {
        look.third_person = !look.third_person;
    }
    if just(Action::Reload) {
        cmd.reloads += 1;
    }
    if just(Action::Grenade) {
        cmd.throws += 1;
    }
    if just(Action::Primary) {
        cmd.slot = 0;
    }
    if just(Action::Secondary) {
        cmd.slot = 1;
    }
    if just(Action::Special) {
        cmd.slot = 2;
    }
    // (A MAX has only its arms.)
    if game.mine().is_some_and(|m| !m.loadout.usable(cmd.slot)) {
        cmd.slot = 0;
    }
    if just(Action::Use) {
        uses.write(UsePressed);
    }
    let k = 1.0 - (-time.delta_secs() / 0.08).exp();
    look.crouch += (if cmd.crouch { 1.0 } else { 0.0 } - look.crouch) * k;
    look.zoom += (if cmd.alt && seat.is_none() { 1.0 } else { 0.0 } - look.zoom) * k;
}

/// The driven vehicle's predicted motion, if driving.
pub fn drive_motion(mover: &Mover) -> Option<ps_sim::vehicle::Motion> {
    mover.drive.as_ref().map(|d| d.2)
}

#[allow(clippy::too_many_arguments)]
fn place_camera(
    game: Option<Res<Game>>,
    mover: Res<Mover>,
    mut look: ResMut<Look>,
    time: Res<Time>,
    real: Res<Time<Real>>,
    mut cams: Query<&mut Transform, With<MainCamera>>,
    free: Option<Res<crate::freecam::FreeCam>>,
    view: Res<crate::seatview::SeatView>,
    mut local: ResMut<LocalCmd>,
) {
    if free.is_some_and(|f| f.on) {
        return;
    }
    let Some(game) = game else { return };
    let Ok(mut cam) = cams.single_mut() else {
        return;
    };
    // Testing: PS_VIEW=x,y,z,tx,ty,tz fixes the camera (relative to the player when PS_VIEW_REL
    // is set).
    if let Ok(v) = std::env::var("PS_VIEW") {
        let n: Vec<f32> = v.split(',').filter_map(|t| t.parse().ok()).collect();
        if n.len() == 6 {
            let base = if std::env::var("PS_VIEW_REL").is_ok() {
                game.mine().map_or(Vec3::ZERO, |m| m.feet)
            } else {
                Vec3::ZERO
            };
            *cam = Transform::from_translation(base + Vec3::new(n[0], n[1], n[2]))
                .looking_at(base + Vec3::new(n[3], n[4], n[5]), Vec3::Y);
            return;
        }
    }
    let rot = Quat::from_rotation_y(look.yaw) * Quat::from_rotation_x(look.pitch);
    let dt = time.delta_secs();
    // Killed: the killcam, from where one fell to the killer.
    if let Some((_, Some(killer))) = game.killcam(real.elapsed_secs())
        && let Some(m) = game.mine()
    {
        let eye = m.feet + Vec3::Y * 1.2;
        let back = (eye - killer).normalize_or(Vec3::Z);
        let from = eye + back * 1.5 + Vec3::Y * 0.6;
        let want = Transform::from_translation(from).looking_at(killer, Vec3::Y);
        let k = (dt * 4.0).min(1.0);
        cam.translation = cam.translation.lerp(want.translation, k);
        cam.rotation = cam.rotation.slerp(want.rotation, k);
        return;
    }
    // Dead or arriving: high above the continent's centre, or where one fell.
    if !game.alive() || !game.ready() {
        let at = game.mine().map_or(Vec3::new(0.0, 0.0, 0.0), |m| m.feet);
        let want =
            Transform::from_translation(at + Vec3::new(0.0, 60.0, 80.0)).looking_at(at, Vec3::Y);
        cam.translation = cam.translation.lerp(want.translation, (dt * 2.0).min(1.0));
        cam.rotation = cam.rotation.slerp(want.rotation, (dt * 2.0).min(1.0));
        return;
    }
    // Seated (a vehicle, a wall turret): from inside, the seat's view (src/seatview.rs).
    if view.active && !look.third_person {
        cam.translation = view.eye;
        cam.rotation = view.rot;
        return;
    }
    // Manning a turret, seen from behind: the gun aims at what the crosshair is on.
    if view.turret.is_some() {
        let at = view.eye + rot * Vec3::new(0.0, 2.0, 8.0);
        cam.translation = keep_clear(&game.world, view.eye, at);
        cam.rotation = rot;
        aim_at_crosshair(&game, &view, &cam, &mut local);
        return;
    }
    if let Some((vid, seat)) = game.seat() {
        let Some(v) = game.vehicle(vid) else { return };
        let spec = v.kind.spec();
        let (pos, vrot) = match drive_motion(&mover) {
            Some(m) if seat == 0 => (m.pos, m.rot),
            _ => (v.pos, v.rot),
        };
        let size = spec.half.z.max(spec.half.x);
        if spec.motion == MotionKind::Flying && seat == 0 {
            // Chasing the craft, smoothed.
            look.cam_rot = look.cam_rot.slerp(vrot, (dt * 5.0).min(1.0));
            let back = look.cam_rot * Vec3::new(0.0, size * 0.55, size * 2.4);
            let pivot = pos + Vec3::Y * spec.half.y;
            cam.translation = keep_clear(&game.world, pivot, pivot + back);
            cam.rotation = look.cam_rot * Quat::from_rotation_x(-0.12);
        } else {
            let pivot = pos + Vec3::Y * (spec.half.y * 2.0 + 1.0);
            let back = rot * Vec3::new(0.0, 0.0, size * 2.2 + 4.0);
            let at = pivot + back + Vec3::Y * 1.5;
            cam.translation = keep_clear(&game.world, pivot, at);
            cam.rotation = rot;
            aim_at_crosshair(&game, &view, &cam, &mut local);
        }
        return;
    }
    if mover.valid {
        let eye = ps_sim::player::EYE - 0.5 * look.crouch;
        cam.translation = mover.body.feet + Vec3::Y * eye;
        cam.rotation = rot;
    } else if let Some(m) = game.mine().filter(|m| m.zip.is_some()) {
        // On a zipline: carried along by the server, followed smoothly.
        let want = m.feet + Vec3::Y * ps_sim::player::EYE;
        let k = if (cam.translation - want).length() > 8.0 {
            1.0
        } else {
            (dt * 12.0).min(1.0)
        };
        cam.translation = cam.translation.lerp(want, k);
        cam.rotation = rot;
    }
}

/// A camera behind `pivot` at `at`, pulled in short of the land between them (a canyon's
/// wall), under a roof (a cave's), and not into the ground.
fn keep_clear(world: &ps_sim::collide::World, pivot: Vec3, at: Vec3) -> Vec3 {
    let mut at = at;
    if let Some(t) = &world.terrain
        && let Some((_, p)) = t.ray(pivot, at)
        && !world.in_hole(p.x, p.z)
    {
        let len = (at - pivot).length().max(1e-3);
        let hit = (p - pivot).length();
        at = pivot.lerp(at, ((hit - 0.4) / len).max(0.0));
    }
    if let Some(c) = world.ceiling(at.x, at.z, pivot.y.min(at.y))
        && at.y > c - 0.5
    {
        at.y = (c - 0.5).max(pivot.y.min(c - 0.5));
    }
    if let Some(g) = world.land(at.x, at.z) {
        at.y = at.y.max(g + 1.0);
    }
    at
}

/// Seen from behind, a gun aims from its pivot at what lies under the crosshair (the
/// camera's line), not along the camera's own direction beside it.
fn aim_at_crosshair(
    game: &Game,
    view: &crate::seatview::SeatView,
    cam: &Transform,
    local: &mut LocalCmd,
) {
    if view.want.is_none() {
        return;
    }
    let fwd = cam.rotation * Vec3::NEG_Z;
    // (From past the vehicle or turret itself.)
    let from = cam.translation + fwd * (cam.translation - view.pivot).length();
    let far = cam.translation + fwd * 800.0;
    let at = game.world.ray(from, far).map_or(far, |h| h.1);
    let d = (at - view.pivot).normalize_or(fwd);
    local.cmd.yaw = ps_sim::yaw_of(d);
    local.cmd.pitch = d.y.clamp(-1.0, 1.0).asin();
}

/// The aim's zoom (the lance's scope a lot, the others a little), from the settings' view.
fn apply_fov(
    settings: Res<Settings>,
    look: Res<Look>,
    game: Option<Res<Game>>,
    local: Res<LocalCmd>,
    mut cams: Query<&mut Projection, With<MainCamera>>,
    free: Option<Res<crate::freecam::FreeCam>>,
) {
    if free.is_some_and(|f| f.on) {
        return;
    }
    let weapon = game
        .as_ref()
        .and_then(|g| g.mine().map(|m| m.loadout.slot(local.cmd.slot)));
    let zoom = match weapon {
        Some(Weapon::Lance) => 4.0,
        Some(Weapon::Rifle) | Some(Weapon::Pistol) => 1.4,
        _ => 1.15,
    };
    let fov = settings.fov.to_radians();
    let zoomed = 2.0 * ((fov / 2.0).tan() / zoom).atan();
    let want = fov + (zoomed - fov) * look.zoom;
    for mut p in &mut cams {
        if let Projection::Perspective(pp) = &mut *p
            && (pp.fov - want).abs() > 1e-4
        {
            pp.fov = want;
        }
    }
}

/// Where the local player looks from and along (for picking what "use" means).
pub fn eye_and_look(game: &Game, mover: &Mover, local: &LocalCmd) -> Option<(Vec3, Vec3)> {
    if !mover.valid {
        return None;
    }
    let eye = mover.body.feet + Vec3::Y * ps_sim::player::eye_height(local.cmd.crouch);
    let _ = game;
    Some((eye, look_dir(local.cmd.yaw, local.cmd.pitch)))
}

/// On spawning (or leaving a vehicle, or warping), look the way one was put facing.
fn face_spawn(
    game: Option<Res<Game>>,
    mut look: ResMut<Look>,
    mut events: MessageReader<crate::game::GameEvent>,
) {
    let Some(game) = game else { return };
    for e in events.read() {
        if let ps_sim::SimEvent::Spawned { player, yaw, .. } = e.0
            && Some(player) == game.me
            && game.seat().is_none()
        {
            look.yaw = yaw;
            look.pitch = 0.0;
        }
    }
}
