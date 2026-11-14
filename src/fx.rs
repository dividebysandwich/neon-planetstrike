//! Shots, impacts and explosions on screen. Everyone else's projectiles come as `Fx::Shot`
//! and fly here until their `Fx::Impact` (or the end of their life); one's own the server
//! doesn't send back: they're fired here at once when the trigger is pulled (`LocalShots`),
//! so they leave the gun without a round trip's delay. They also end here, on what they meet
//! as this client sees it (the land, a wall, a forcefield, an enemy body, vehicle or turret),
//! with their impact at once (`LocalImpacts`); the server's word on that impact, a round trip
//! and the playback delay later, is then passed over. (An impact the client didn't foresee
//! still shows when the server's word comes.)
//!
//! Impacts make particles (`Particle`): a bullet kicks up dust from the ground and sparks in
//! its faction's colour, rings hot sparks off armour, flashes on a forcefield; whatever
//! explodes makes a flash, a fireball, smoke rising, embers flying and (on the ground) dust
//! blown outwards. Flashes, fire, smoke and dust are soft particles (`ParticleMaterial`) on
//! quads turned to the viewer: alpha blended smoke and dust, additive fire and flashes.

use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;
use ps_sim::snapshot::{Fx, ImpactOn};
use ps_sim::vehicle::VehicleKind;
use ps_sim::weapons::{Ammo, ShotKind, Weapon};
use ps_sim::{PlayerId, Team, VehicleId, look_dir};

use crate::game::{FxEvent, Game, LocalCmd, Mover};
use crate::materials::{AccentMaterial, GlowMaterial, NeonUniform, ParticleMaterial, team_color};

/// A missile homing on a vehicle (as the server has it fly).
#[derive(Component)]
struct Homing(VehicleId);

#[derive(Component)]
struct Projectile {
    id: u32,
    ammo: Ammo,
    vel: Vec3,
    gravity: f32,
    left: f32,
}

#[derive(Component)]
struct Blast {
    age: f32,
    life: f32,
    size: f32,
}

/// A particle: it flies (slowed by drag, pulled down by gravity), grows or shrinks from
/// `size.0` to `size.1` over its life, and fades through its `Fade` materials.
#[derive(Component)]
struct Particle {
    vel: Vec3,
    gravity: f32,
    drag: f32,
    age: f32,
    life: f32,
    size: (f32, f32),
}

/// A soft particle on a quad, turned to face the viewer each frame (rolled by this).
#[derive(Component)]
struct Billboard(f32);

/// A particle fading through materials: smoke (`FxAssets::puff`) or fire (`FxAssets::flame`).
#[derive(Component, Clone, Copy, PartialEq)]
enum Fade {
    Smoke,
    Dust,
    Fire,
}

const FADE_STEPS: usize = 6;

#[derive(Resource)]
struct FxAssets {
    tracer: Handle<Mesh>,
    shell: Handle<Mesh>,
    grenade: Handle<Mesh>,
    /// The soft particles' quad (2 m across: scaled by a particle's radius).
    quad: Handle<Mesh>,
    chip: Handle<Mesh>,
    /// A blast's fireball and its smoke.
    fire: Handle<ParticleMaterial>,
    smoke: Handle<ParticleMaterial>,
    spark: [Handle<AccentMaterial>; 4],
    /// A muzzle flash in each faction's colour (and none's).
    muzzle: [Handle<ParticleMaterial>; 4],
    /// Smoke and dust, thinner and thinner; fire, dimmer and dimmer.
    puff: [Handle<ParticleMaterial>; FADE_STEPS],
    dust: [Handle<ParticleMaterial>; FADE_STEPS],
    flame: [Handle<ParticleMaterial>; FADE_STEPS],
    /// Hot debris and sparks (chips), and white-hot flashes.
    hot: Handle<GlowMaterial>,
    flash: Handle<ParticleMaterial>,
    shield: Handle<ParticleMaterial>,
}

/// The local player's guns, as this client reckons them (for firing at once).
#[derive(Resource, Default)]
pub struct LocalShots {
    cooldown: f32,
    prev_fire: bool,
    next_id: u32,
    /// Shots fired here this frame (for the sounds): ammo and where.
    pub fired: Vec<(Ammo, Vec3)>,
    barrel: u8,
    /// The seat's second gun (missiles).
    alt_cooldown: f32,
    alt_barrel: u8,
}

/// The projectiles flying (by id), and their shooters' factions.
#[derive(Resource, Default)]
struct Flying(HashMap<u32, (Entity, Option<Team>)>);

/// One's own shots' impacts shown here before the server's word: where, what, when (s).
#[derive(Resource, Default)]
struct LocalImpacts(VecDeque<(Vec3, Ammo, f32)>);

/// How long (s) and how far (m, besides a blast's radius) the server's word on an impact
/// may come after and away from one shown here, to be that one.
const MATCH_TIME: f32 = 1.5;
const MATCH_DIST: f32 = 3.0;

/// Effects for the audio (src/audio.rs) and the HUD: what hit where.
#[derive(Message, Clone, Copy, Debug)]
pub struct Boom {
    pub at: Vec3,
    pub ammo: Option<Ammo>,
    pub on: ImpactOn,
    pub wreck: Option<VehicleKind>,
}

/// The effects' systems (the audio follows them).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FxSet;

pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LocalShots>()
            .init_resource::<Flying>()
            .init_resource::<LocalImpacts>()
            .add_message::<Boom>()
            .add_systems(Startup, make_assets)
            .add_systems(
                Update,
                (
                    local_fire,
                    receive,
                    fly,
                    blasts,
                    particles,
                    face_viewer,
                    flares,
                    jets_and_chutes,
                )
                    .chain()
                    .in_set(FxSet)
                    .after(crate::game::PredictSet)
                    .run_if(resource_exists::<Game>),
            );
    }
}

fn make_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut glows: ResMut<Assets<GlowMaterial>>,
    mut accents: ResMut<Assets<AccentMaterial>>,
    mut parts: ResMut<Assets<ParticleMaterial>>,
) {
    let spark = [Some(Team::Red), Some(Team::Blue), Some(Team::Green), None].map(|t| {
        accents.add(AccentMaterial {
            neon: NeonUniform::fixed(team_color(t), 9.0),
            overlay: false,
        })
    });
    let fade = |k: usize| 1.0 - k as f32 / FADE_STEPS as f32;
    let smoke_of = |c: Vec3, a: f32, parts: &mut Assets<ParticleMaterial>| {
        std::array::from_fn(|k| parts.add(ParticleMaterial::puff(c, a * fade(k), k as f32 * 3.7)))
    };
    let puff = smoke_of(Vec3::new(0.025, 0.025, 0.03), 0.8, &mut parts);
    let dust = smoke_of(Vec3::new(0.11, 0.1, 0.09), 0.6, &mut parts);
    let flame = std::array::from_fn(|k| {
        parts.add(ParticleMaterial::glow(
            Vec3::new(1.0, 0.38, 0.06) * 3.0,
            fade(k) * fade(k),
        ))
    });
    let muzzle = [Some(Team::Red), Some(Team::Blue), Some(Team::Green), None]
        .map(|t| parts.add(ParticleMaterial::glow(team_color(t) * 4.0, 1.0)));
    commands.insert_resource(FxAssets {
        tracer: meshes.add(Cuboid::new(0.05, 0.05, 1.6)),
        shell: meshes.add(Sphere::new(0.18).mesh().ico(1).unwrap()),
        grenade: meshes.add(Sphere::new(0.09).mesh().ico(2).unwrap()),
        quad: meshes.add(Rectangle::new(2.0, 2.0)),
        chip: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        fire: parts.add(ParticleMaterial::glow(Vec3::new(1.0, 0.45, 0.1) * 3.0, 1.0)),
        smoke: parts.add(ParticleMaterial::puff(
            Vec3::new(0.025, 0.025, 0.03),
            0.7,
            11.0,
        )),
        spark,
        muzzle,
        puff,
        dust,
        flame,
        hot: glows.add(GlowMaterial {
            neon: NeonUniform::fixed(Vec3::new(1.0, 0.85, 0.55), 8.0),
            overlay: false,
        }),
        flash: parts.add(ParticleMaterial::glow(Vec3::new(1.0, 0.8, 0.5) * 4.0, 1.0)),
        shield: parts.add(ParticleMaterial::glow(Vec3::new(0.4, 0.8, 1.0) * 2.5, 0.8)),
    });
}

fn team_of(game: &Game, id: PlayerId) -> Option<Team> {
    game.snap
        .players
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.team)
        .or_else(|| game.names.get(&id).map(|n| n.1))
}

fn spawn_projectile(
    commands: &mut Commands,
    fx: &FxAssets,
    id: u32,
    ammo: Ammo,
    from: Vec3,
    vel: Vec3,
    team: Option<Team>,
) -> Entity {
    let b = ammo.ballistics();
    let mesh = match ammo {
        Ammo::Grenade | Ammo::Utility(_) => fx.grenade.clone(),
        Ammo::LightShell
        | Ammo::HeavyShell
        | Ammo::Rocket
        | Ammo::Missile
        | Ammo::Gun(Weapon::Skyhook)
        | Ammo::Gun(Weapon::Striker) => fx.shell.clone(),
        _ => fx.tracer.clone(),
    };
    let mat = fx.spark[team.map_or(3, |t| t.index())].clone();
    // (Bolts stretched with their speed; a grenade a ball.)
    let len = if b.bounces {
        1.0
    } else {
        (vel.length() / 140.0).clamp(0.4, 4.0)
    };
    commands
        .spawn((
            Projectile {
                id,
                ammo,
                vel,
                gravity: b.gravity,
                left: b.life,
            },
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(from)
                .looking_to(vel.normalize_or(Vec3::NEG_Z), Vec3::Y)
                .with_scale(Vec3::new(1.0, 1.0, len)),
        ))
        .id()
}

/// A gun's muzzle flash: a brief spark of the shooter's colour.
fn spawn_flash(commands: &mut Commands, fx: &FxAssets, at: Vec3, team: Option<Team>) {
    commands.spawn((
        Blast {
            age: 0.0,
            life: 0.06,
            size: 0.22,
        },
        Billboard(rnd(0.0, 6.3)),
        Mesh3d(fx.quad.clone()),
        MeshMaterial3d(fx.muzzle[team.map_or(3, |t| t.index())].clone()),
        Transform::from_translation(at).with_scale(Vec3::splat(0.05)),
    ));
}

fn spawn_blast(commands: &mut Commands, fx: &FxAssets, at: Vec3, size: f32, life: f32) {
    commands.spawn((
        Blast {
            age: 0.0,
            life,
            size,
        },
        Billboard(rnd(0.0, 6.3)),
        Mesh3d(fx.quad.clone()),
        MeshMaterial3d(fx.fire.clone()),
        Transform::from_translation(at).with_scale(Vec3::splat(0.1)),
    ));
    if size > 2.0 {
        commands.spawn((
            Blast {
                age: 0.0,
                life: life * 3.0,
                size: size * 1.4,
            },
            Billboard(rnd(0.0, 6.3)),
            Mesh3d(fx.quad.clone()),
            MeshMaterial3d(fx.smoke.clone()),
            Transform::from_translation(at + Vec3::Y * size * 0.5).with_scale(Vec3::splat(0.1)),
        ));
        commands.spawn((
            Blast {
                age: 0.0,
                life: life * 0.6,
                size: 0.0,
            },
            PointLight {
                color: Color::linear_rgb(1.0, 0.5, 0.15),
                intensity: 4_000_000.0 * size,
                range: size * 8.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(at + Vec3::Y),
        ));
    }
}

/// A random unit vector (`up`: in the upper half).
fn random_dir(up: bool) -> Vec3 {
    let v = Vec3::new(
        rand::random::<f32>() * 2.0 - 1.0,
        rand::random::<f32>() * 2.0 - 1.0,
        rand::random::<f32>() * 2.0 - 1.0,
    )
    .normalize_or(Vec3::Y);
    if up {
        Vec3::new(v.x, v.y.abs(), v.z)
    } else {
        v
    }
}

fn rnd(lo: f32, hi: f32) -> f32 {
    lo + rand::random::<f32>() * (hi - lo)
}

#[allow(clippy::too_many_arguments)]
fn particle<'a>(
    commands: &'a mut Commands,
    mesh: &Handle<Mesh>,
    at: Vec3,
    vel: Vec3,
    gravity: f32,
    drag: f32,
    life: f32,
    size: (f32, f32),
) -> EntityCommands<'a> {
    let mut e = commands.spawn((
        Particle {
            vel,
            gravity,
            drag,
            age: 0.0,
            life,
            size,
        },
        Mesh3d(mesh.clone()),
        Transform::from_translation(at)
            .with_rotation(Quat::from_euler(
                EulerRot::XYZ,
                rnd(0.0, 6.3),
                rnd(0.0, 6.3),
                rnd(0.0, 6.3),
            ))
            .with_scale(Vec3::splat(size.0.max(0.001))),
    ));
    e.insert(Visibility::default());
    e
}

/// A soft particle (`FxAssets::quad`, facing the viewer) of `size` radius growing to the second.
#[allow(clippy::too_many_arguments)]
fn soft<'a>(
    commands: &'a mut Commands,
    fx: &FxAssets,
    mat: &Handle<ParticleMaterial>,
    at: Vec3,
    vel: Vec3,
    gravity: f32,
    drag: f32,
    life: f32,
    size: (f32, f32),
) -> EntityCommands<'a> {
    let mut e = particle(commands, &fx.quad, at, vel, gravity, drag, life, size);
    e.insert((Billboard(rnd(0.0, 6.3)), MeshMaterial3d(mat.clone())));
    e
}

/// Sparks flying from `at` (up, off the ground, when `up`).
fn sparks(
    commands: &mut Commands,
    fx: &FxAssets,
    at: Vec3,
    n: usize,
    speed: f32,
    mat: Option<&Handle<AccentMaterial>>,
    up: bool,
) {
    for _ in 0..n {
        let v = random_dir(up) * speed * rnd(0.5, 1.2);
        let size = rnd(0.04, 0.08);
        let mut e = particle(
            commands,
            &fx.chip,
            at,
            v,
            9.8,
            1.5,
            rnd(0.2, 0.45),
            (size, 0.0),
        );
        match mat {
            Some(m) => e.insert(MeshMaterial3d(m.clone())),
            None => e.insert(MeshMaterial3d(fx.hot.clone())),
        };
    }
}

/// Smoke or dust: puffs round `at`, rising and spreading.
#[allow(clippy::too_many_arguments)]
fn puffs(
    commands: &mut Commands,
    fx: &FxAssets,
    at: Vec3,
    n: usize,
    size: f32,
    life: f32,
    rise: f32,
    kind: Fade,
) {
    let mats = if kind == Fade::Dust {
        &fx.dust
    } else {
        &fx.puff
    };
    for _ in 0..n {
        let off = random_dir(true) * size * 0.4;
        let v = Vec3::Y * rise * rnd(0.6, 1.3) + random_dir(false) * size * 0.6;
        soft(
            commands,
            fx,
            &mats[0],
            at + off,
            v,
            -0.3,
            1.2,
            life * rnd(0.75, 1.25),
            (size * 0.45, size * rnd(1.1, 1.5)),
        )
        .insert(kind);
    }
}

/// An explosion of `size` (its splash radius) at `at`: a white flash, a fireball, debris,
/// smoke rising and (on the ground) dust blown out round it.
fn explosion(commands: &mut Commands, fx: &FxAssets, at: Vec3, size: f32, ground: bool) {
    let s = size.max(0.8);
    soft(
        commands,
        fx,
        &fx.flash,
        at,
        Vec3::ZERO,
        0.0,
        0.0,
        0.14,
        (s * 0.35, s * 0.8),
    );
    for _ in 0..5 {
        let off = random_dir(true) * s * 0.2;
        soft(
            commands,
            fx,
            &fx.flame[0],
            at + off,
            Vec3::Y * s * 0.6 + random_dir(false) * s * 0.5,
            -1.0,
            2.0,
            rnd(0.35, 0.6),
            (s * 0.25, s * rnd(0.6, 0.85)),
        )
        .insert(Fade::Fire);
    }
    let debris = (6.0 + s * 2.0).min(22.0) as usize;
    for _ in 0..debris {
        let v = random_dir(ground) * rnd(6.0, 14.0) * s.sqrt();
        // (Embers: glowing, fading as they fly.)
        let size = rnd(0.06, 0.14) * s.sqrt();
        soft(
            commands,
            fx,
            &fx.flame[0],
            at,
            v,
            14.0,
            0.6,
            rnd(0.5, 1.1),
            (size, size * 0.4),
        )
        .insert(Fade::Fire);
    }
    puffs(
        commands,
        fx,
        at + Vec3::Y * s * 0.2,
        (2.0 + s * 0.8).min(8.0) as usize,
        s * 0.9,
        rnd(1.6, 2.4),
        s * 0.5,
        Fade::Smoke,
    );
    if ground {
        for k in 0..8 {
            let a = k as f32 / 8.0 * std::f32::consts::TAU + rnd(0.0, 0.5);
            let out = Vec3::new(a.cos(), 0.0, a.sin());
            soft(
                commands,
                fx,
                &fx.dust[0],
                at + out * s * 0.3,
                out * s * rnd(2.0, 3.0) + Vec3::Y * 0.6,
                0.0,
                2.2,
                rnd(0.9, 1.4),
                (s * 0.3, s * 0.7),
            )
            .insert(Fade::Dust);
        }
    }
}

/// What a projectile does where it hits.
fn impact_fx(
    commands: &mut Commands,
    fx: &FxAssets,
    at: Vec3,
    ammo: Ammo,
    what: ImpactOn,
    team: Option<Team>,
) {
    let b = ammo.ballistics();
    let spark = &fx.spark[team.map_or(3, |t| t.index())];
    if b.splash > 0.0 {
        explosion(commands, fx, at, b.splash * 0.7, what == ImpactOn::Ground);
        return;
    }
    // (A heavier gun, a bigger hit.)
    let k = (b.damage / 12.0).clamp(0.6, 2.5);
    match what {
        ImpactOn::Ground => {
            puffs(commands, fx, at, 2, 0.6 * k, 0.6, 0.8, Fade::Dust);
            sparks(commands, fx, at, 4, 5.0, Some(spark), true);
        }
        ImpactOn::Body => sparks(commands, fx, at, 7, 4.0, Some(spark), false),
        ImpactOn::Vehicle => {
            sparks(commands, fx, at, 8, 7.0, None, false);
            soft(
                commands,
                fx,
                &fx.flash,
                at,
                Vec3::ZERO,
                0.0,
                0.0,
                0.08,
                (0.2, 0.5 * k),
            );
        }
        ImpactOn::Shield => {
            soft(
                commands,
                fx,
                &fx.shield,
                at,
                Vec3::ZERO,
                0.0,
                0.0,
                0.25,
                (0.3, 1.4 * k),
            );
            sparks(commands, fx, at, 4, 3.0, Some(spark), false);
        }
        ImpactOn::Air => {
            puffs(commands, fx, at, 2, 0.8 * k, 0.8, 0.3, Fade::Smoke);
        }
    }
}

/// Turns the soft particles to the viewer: the camera, or in VR the head.
fn face_viewer(
    cams: Query<&GlobalTransform, With<crate::desktop::MainCamera>>,
    vr: Option<Res<crate::vr::VrState>>,
    xr: Option<Res<crate::vr::XrInput>>,
    mut q: Query<(&mut Transform, &Billboard)>,
) {
    let eye = match (vr, xr) {
        (Some(vr), Some(xr)) => (vr.rig * xr.head.unwrap_or_default()).translation,
        _ => match cams.single() {
            Ok(c) => c.translation(),
            Err(_) => return,
        },
    };
    for (mut t, b) in &mut q {
        let to = t.translation - eye;
        if to.length_squared() > 1e-6 {
            // (+Z, the quad's face, towards the eye.)
            t.rotation =
                Transform::IDENTITY.looking_to(to, Vec3::Y).rotation * Quat::from_rotation_z(b.0);
        }
    }
}

/// Particles fly, grow or shrink, fade and go.
fn particles(
    mut commands: Commands,
    time: Res<Time>,
    fx: Option<Res<FxAssets>>,
    mut q: Query<(
        Entity,
        &mut Particle,
        &mut Transform,
        Option<&Fade>,
        Option<&mut MeshMaterial3d<ParticleMaterial>>,
    )>,
) {
    let Some(fx) = fx else { return };
    let dt = time.delta_secs();
    for (e, mut p, mut t, fade, mat) in &mut q {
        p.age += dt;
        if p.age >= p.life {
            commands.entity(e).despawn();
            continue;
        }
        let k = p.age / p.life;
        let drag = (1.0 - p.drag * dt).max(0.0);
        p.vel *= drag;
        p.vel.y -= p.gravity * dt;
        t.translation += p.vel * dt;
        t.scale = Vec3::splat((p.size.0 + (p.size.1 - p.size.0) * k.sqrt()).max(0.001));
        if let (Some(&fade), Some(mut m)) = (fade, mat) {
            let step = ((k * FADE_STEPS as f32) as usize).min(FADE_STEPS - 1);
            let want = match fade {
                Fade::Smoke => &fx.puff[step],
                Fade::Dust => &fx.dust[step],
                Fade::Fire => &fx.flame[step],
            };
            if m.0 != *want {
                m.0 = want.clone();
            }
        }
    }
}

/// One's own shots, fired here at once.
#[allow(clippy::too_many_arguments)]
fn local_fire(
    mut commands: Commands,
    game: Res<Game>,
    local: Res<LocalCmd>,
    mover: Res<Mover>,
    fx: Option<Res<FxAssets>>,
    mut shots: ResMut<LocalShots>,
    time: Res<Time>,
    models: Res<crate::assets::Models>,
    view: Res<crate::seatview::SeatView>,
) {
    shots.fired.clear();
    let Some(fx) = fx else { return };
    let dt = time.delta_secs();
    shots.cooldown = (shots.cooldown - dt).max(0.0);
    let fire = local.cmd.fire && local.active;
    let edge = fire && !shots.prev_fire;
    shots.prev_fire = fire;
    let Some(me) = game.mine().cloned() else {
        return;
    };
    // A locked target, for the missile about to go.
    let locked = me.lock.filter(|l| l.1 >= 1.0).map(|l| l.0);
    // The seat's second gun (the interceptor's missiles) on the other trigger.
    shots.alt_cooldown = (shots.alt_cooldown - dt).max(0.0);
    if local.cmd.alt
        && local.active
        && shots.alt_cooldown <= 0.0
        && me.dead.is_none()
        && game.ready()
        && me.alt_ammo.is_some_and(|a| a.0 > 0)
        && let Some((vid, seat)) = me.seat
        && let Some(v) = game.vehicle(vid)
        && let Some(gi) = v
            .kind
            .spec()
            .seats
            .get(seat as usize)
            .and_then(|s| s.alt_gun)
    {
        let gs = v.kind.gun_spec(v.loadout, gi);
        shots.alt_cooldown = gs.interval;
        shots.alt_barrel = (shots.alt_barrel + 1) % gs.barrels.max(1);
        let (pos, rot) = match &mover.drive {
            Some((id, _, m)) if *id == vid => (m.pos, m.rot),
            _ => (v.pos, v.rot),
        };
        let side = (shots.alt_barrel as f32 - (gs.barrels - 1) as f32 * 0.5) * gs.barrel_gap;
        let from = pos + rot * (gs.pivot + Vec3::new(side, 0.0, -gs.barrel));
        let vel = rot * Vec3::NEG_Z * gs.ammo.ballistics().speed + v.vel;
        shots.next_id = shots.next_id.wrapping_add(1);
        let id = u32::MAX - shots.next_id % 100_000;
        let e = spawn_projectile(&mut commands, &fx, id, gs.ammo, from, vel, Some(game.team));
        if let Some(t) = locked {
            commands.entity(e).insert(Homing(t));
        }
        shots.fired.push((gs.ammo, from));
    }
    if me.dead.is_some() || !game.ready() || !fire || shots.cooldown > 0.0 {
        return;
    }
    // (Nobody fires inside a warp gate's bubble.)
    let at = if view.active {
        view.eye
    } else {
        mover.body.feet + Vec3::Y
    };
    if game
        .map
        .as_ref()
        .is_some_and(|m| m.gates.iter().any(|g| ps_sim::base::in_gate_bubble(*g, at)))
    {
        return;
    }
    let team = Some(game.team);
    let look = look_dir(local.cmd.yaw, local.cmd.pitch);
    // A wall turret: its bolts from its barrels, turn about.
    if me.turret.is_some() && view.active {
        let powered = game
            .snap
            .turrets
            .iter()
            .find(|t| t.occupant == game.me)
            .is_some_and(|t| t.powered);
        if !powered {
            return;
        }
        shots.cooldown = ps_sim::sim::TURRET_INTERVAL;
        shots.barrel = (shots.barrel + 1) % 2;
        let d = view.rot * Vec3::NEG_Z;
        let side = view.rot * Vec3::X * if shots.barrel == 0 { 0.55 } else { -0.55 };
        let from = view.pivot + d * ps_sim::base::TURRET_BARREL + side;
        let vel = d * Ammo::TurretBolt.ballistics().speed;
        shots.next_id = shots.next_id.wrapping_add(1);
        let id = u32::MAX - shots.next_id % 100_000;
        spawn_projectile(&mut commands, &fx, id, Ammo::TurretBolt, from, vel, team);
        spawn_blast(&mut commands, &fx, from, 0.3, 0.08);
        shots.fired.push((Ammo::TurretBolt, from));
        return;
    }
    match me.seat {
        None => {
            let slot = (local.cmd.slot as usize).min(ps_sim::weapons::SLOTS - 1);
            let w = me.loadout.slot(slot as u8);
            let s = w.stats();
            let g = me.guns[slot];
            if g.mag == 0 || g.reload > 0.0 || (!s.automatic && !edge) || s.shot == ShotKind::Beam {
                return;
            }
            shots.cooldown = s.interval;
            let (muzzle, dir) = match &local.cmd.vr {
                // VR: from the gun in the hand, along its barrel.
                Some(vr) => (mover.body.feet + vr.muzzle.pos, vr.muzzle.forward()),
                None => {
                    // (From the gun's muzzle, converging on the crosshair's line.)
                    let eye =
                        mover.body.feet + Vec3::Y * ps_sim::player::eye_height(local.cmd.crouch);
                    let (_, hands) = crate::actors::flat_poses(
                        local.cmd.yaw,
                        local.cmd.pitch,
                        local.cmd.crouch,
                        w,
                    );
                    let muzzle = mover.body.feet
                        + hands[1].translation
                        + hands[1].rotation * crate::actors::muzzle_of(&models, w);
                    let aim = game
                        .world
                        .ray(eye, eye + look * 600.0)
                        .map_or(eye + look * 600.0, |h| h.1);
                    (muzzle, (aim - muzzle).normalize_or(look))
                }
            };
            let moving = mover.body.vel.length() / ps_sim::player::WALK_SPEED;
            let spread = w.spread_for(
                local.cmd.alt,
                local.cmd.crouch,
                moving,
                !mover.body.on_ground(),
            );
            for _ in 0..s.pellets.min(6) {
                let d = (dir
                    + Vec3::new(
                        rand::random::<f32>() - 0.5,
                        rand::random::<f32>() - 0.5,
                        rand::random::<f32>() - 0.5,
                    ) * spread
                        * 2.0)
                    .normalize();
                shots.next_id = shots.next_id.wrapping_add(1);
                let id = u32::MAX - shots.next_id % 100_000;
                let e = spawn_projectile(
                    &mut commands,
                    &fx,
                    id,
                    Ammo::Gun(w),
                    muzzle,
                    d * s.speed,
                    team,
                );
                if let Some(t) = locked.filter(|_| Ammo::Gun(w).lock_on().is_some()) {
                    commands.entity(e).insert(Homing(t));
                }
            }
            spawn_flash(&mut commands, &fx, muzzle, team);
            shots.fired.push((Ammo::Gun(w), muzzle));
        }
        Some((vid, seat)) => {
            let Some(v) = game.vehicle(vid) else { return };
            // (The harvester's beam is no gun: src/vehicles.rs draws it.)
            if v.kind == VehicleKind::Harvester {
                return;
            }
            let spec = v.kind.spec();
            let Some(gi) = spec.seats.get(seat as usize).and_then(|s| s.gun) else {
                return;
            };
            // (Out of rounds: the server fires nothing, so neither does the screen.)
            if me.seat_ammo.is_some_and(|a| a.0 == 0) {
                return;
            }
            let gs = v.kind.gun_spec(v.loadout, gi);
            shots.cooldown = gs.interval;
            let (pos, rot) = match &mover.drive {
                Some((id, _, m)) if *id == vid => (m.pos, m.rot),
                _ => (v.pos, v.rot),
            };
            let (yaw, pitch) = v
                .aim
                .get(gi)
                .map_or((0.0, 0.0), |a| (a.0 as f32 / 1e4, a.1 as f32 / 1e4));
            let aim = if gs.fixed {
                Quat::IDENTITY
            } else if view.want.is_some() && view.vehicle == Some(vid) {
                // (As the view has it: predicted.)
                rot.inverse() * view.rot
            } else {
                Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch)
            };
            shots.barrel = (shots.barrel + 1) % gs.barrels.max(1);
            let side = if gs.barrels > 1 {
                (shots.barrel as f32 - (gs.barrels - 1) as f32 * 0.5) * gs.barrel_gap
            } else {
                0.0
            };
            let from = pos + rot * (gs.pivot + aim * Vec3::new(side, 0.0, -gs.barrel));
            let dir = rot * aim * Vec3::NEG_Z;
            let vel = dir * gs.ammo.ballistics().speed + v.vel;
            shots.next_id = shots.next_id.wrapping_add(1);
            let id = u32::MAX - shots.next_id % 100_000;
            spawn_projectile(&mut commands, &fx, id, gs.ammo, from, vel, team);
            shots.fired.push((gs.ammo, from));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn receive(
    mut commands: Commands,
    game: Res<Game>,
    fx: Option<Res<FxAssets>>,
    mut events: MessageReader<FxEvent>,
    mut flying: ResMut<Flying>,
    mut booms: MessageWriter<Boom>,
    mut local: ResMut<LocalImpacts>,
    time: Res<Time>,
    mut shots: Query<(Entity, &mut Projectile, &mut Transform, Option<&Homing>)>,
) {
    let Some(fx) = fx else { return };
    let now = time.elapsed_secs();
    while local.0.front().is_some_and(|l| now - l.2 > MATCH_TIME) {
        local.0.pop_front();
    }
    for FxEvent(f) in events.read() {
        match *f {
            Fx::Shot {
                id,
                owner,
                ammo,
                from,
                vel,
                target,
            } => {
                let team = team_of(&game, owner);
                // A grenade's new flight after a bounce: the one flying follows it.
                if let Some(&(e, _)) = flying.0.get(&id) {
                    if let Ok((_, mut p, mut t, _)) = shots.get_mut(e) {
                        p.vel = vel;
                        t.translation = from;
                    }
                    continue;
                }
                if matches!(ammo, Ammo::Gun(_)) {
                    spawn_flash(&mut commands, &fx, from, team);
                }
                let e = spawn_projectile(&mut commands, &fx, id, ammo, from, vel, team);
                if let Some(t) = target {
                    commands.entity(e).insert(Homing(t));
                }
                flying.0.insert(id, (e, team));
            }
            Fx::Impact { id, ammo, at, what } => {
                // (A shot not seen flying is one's own: the server doesn't send those.)
                let team = match flying.0.remove(&id) {
                    Some((e, team)) => {
                        commands.entity(e).try_despawn();
                        team
                    }
                    None => {
                        // One's own: shown already if this client saw it coming.
                        let reach = MATCH_DIST + ammo.ballistics().splash;
                        if let Some(i) = local
                            .0
                            .iter()
                            .position(|l| l.1 == ammo && (l.0 - at).length() < reach)
                        {
                            let l = local.0.remove(i);
                            debug!(
                                "own impact shown {:.0} ms before the server's word",
                                l.map_or(0.0, |l| (now - l.2) * 1000.0)
                            );
                            continue;
                        }
                        debug!("impact of a shot not seen flying, or not foreseen here: {what:?}");
                        Some(game.team)
                    }
                };
                show_impact(&mut commands, &fx, &mut booms, at, ammo, what, team);
            }
            Fx::Wreck { at, kind } => {
                spawn_blast(&mut commands, &fx, at, 7.0, 0.9);
                explosion(&mut commands, &fx, at, 6.0, true);
                booms.write(Boom {
                    at,
                    ammo: None,
                    on: ImpactOn::Vehicle,
                    wreck: Some(kind),
                });
            }
            Fx::Muzzle { at, .. } => {
                spawn_blast(&mut commands, &fx, at, 0.3, 0.08);
            }
        }
    }
}

/// An impact's effects: the blast's light, the particles, the sound (`Boom`).
fn show_impact(
    commands: &mut Commands,
    fx: &FxAssets,
    booms: &mut MessageWriter<Boom>,
    at: Vec3,
    ammo: Ammo,
    what: ImpactOn,
    team: Option<Team>,
) {
    let b = ammo.ballistics();
    if let Ammo::Utility(k) = ammo {
        utility_fx(commands, fx, at, k);
    } else {
        if b.splash > 0.0 {
            // (The light of the blast.)
            spawn_blast(commands, fx, at, b.splash * 0.6, 0.5);
        }
        impact_fx(commands, fx, at, ammo, what, team);
    }
    booms.write(Boom {
        at,
        ammo: Some(ammo),
        on: what,
        wreck: None,
    });
}

/// What one's own shot meets going from `a` to `b`, as this client sees it (as the server
/// reckons it: the land and walls, forcefields but the faction's own, enemy bodies on foot,
/// enemy vehicles' hulls and the enemy's standing turrets): where and what.
fn own_hit(game: &Game, a: Vec3, b: Vec3) -> Option<(Vec3, ImpactOn)> {
    let team = game.team;
    let seg = b - a;
    let mut best: Option<(f32, ImpactOn)> = None;
    let mut take = |f: f32, on: ImpactOn| {
        if best.is_none_or(|h| f < h.0) {
            best = Some((f, on));
        }
    };
    let vehicles: Vec<(ps_sim::vehicle::Motion, Vec3, Team)> = game
        .snap
        .vehicles
        .iter()
        .map(|v| {
            let mut veh = ps_sim::sim::Vehicle::new(v.id, v.kind, v.team, v.pos, 0.0);
            veh.motion.rot = v.rot;
            (veh.motion, v.kind.spec().half, v.team)
        })
        .collect();
    if let Some((f, at, _)) = game.world.ray(a, b) {
        // (The vehicles' boxes in the client's world aren't the server's walls: their hulls
        // below are.)
        let on_vehicle = vehicles.iter().any(|(m, half, _)| {
            ps_sim::vehicle::hit_hull(
                m,
                *half + Vec3::splat(0.3),
                at - seg.normalize_or(Vec3::Y) * 0.5,
                at + seg.normalize_or(Vec3::Y) * 0.5,
            )
            .is_some()
        });
        if !on_vehicle {
            let shield = game.world.shields.iter().any(|(c, _)| {
                let (lo, hi) = c.bounds();
                at.cmpge(lo - 0.05).all() && at.cmple(hi + 0.05).all()
            });
            take(
                f,
                if shield {
                    ImpactOn::Shield
                } else {
                    ImpactOn::Ground
                },
            );
        }
    }
    for p in &game.snap.players {
        if p.team == team
            || Some(p.id) == game.me
            || p.seat.is_some()
            || p.has(ps_sim::snapshot::flags::DEAD)
        {
            continue;
        }
        let crouch = p.has(ps_sim::snapshot::flags::CROUCH);
        if let Some((f, _)) = ps_sim::player::hit_body(p.feet, crouch, a, b) {
            take(f, ImpactOn::Body);
        }
    }
    for (m, half, t) in &vehicles {
        if *t == team {
            continue;
        }
        if let Some(f) = ps_sim::vehicle::hit_hull(m, *half, a, b) {
            take(f, ImpactOn::Vehicle);
        }
    }
    if let Some(map) = &game.map {
        for t in &game.snap.turrets {
            if t.health <= 0.0 || t.team.is_none_or(|o| o == team) {
                continue;
            }
            let Some(pos) = map
                .bases
                .get(t.base as usize)
                .and_then(|l| l.turrets.get(t.index as usize))
            else {
                continue;
            };
            let c = *pos + Vec3::Y * ps_sim::base::TURRET_PIVOT;
            let k = ((c - a).dot(seg) / seg.length_squared().max(1e-6)).clamp(0.0, 1.0);
            if (a + seg * k - c).length() < 1.6 {
                take(k, ImpactOn::Vehicle);
            }
        }
    }
    best.map(|(f, on)| (a + seg * f, on))
}

#[allow(clippy::too_many_arguments)]
fn fly(
    mut commands: Commands,
    game: Res<Game>,
    time: Res<Time>,
    fx: Option<Res<FxAssets>>,
    mut flying: ResMut<Flying>,
    mut local: ResMut<LocalImpacts>,
    mut booms: MessageWriter<Boom>,
    mut q: Query<(Entity, &mut Projectile, &mut Transform, Option<&Homing>)>,
) {
    let Some(fx) = fx else { return };
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    for (e, mut p, mut t, homing) in &mut q {
        p.left -= dt;
        p.vel.y -= p.gravity * dt;
        // A missile turns towards its target (and goes on straight once flares lead it off).
        if let Some(h) = homing
            && let Some(v) = game.vehicle(h.0)
        {
            if v.flags & ps_sim::snapshot::vflags::FLARES != 0 {
                commands.entity(e).remove::<Homing>();
            } else {
                let c = v.pos + v.rot * Vec3::Y * v.kind.spec().half.y;
                p.vel = ps_sim::sim::home(p.vel, c - t.translation, dt);
            }
        }
        let mut next = t.translation + p.vel * dt;
        // A grenade bounces off the land and what stands on it (the server's word on its new
        // flight follows).
        if p.ammo.ballistics().bounces
            && let Some((_, at, n)) = game.world.ray(t.translation, next)
        {
            let vn = p.vel.dot(n);
            p.vel = (p.vel - n * vn * 1.6) * 0.55;
            next = at + n * 0.05;
        }
        // One's own end on what they meet, with their impact now (the server's word on it
        // comes later, and is passed over then).
        let own = p.id > u32::MAX - 100_001;
        let hit = if own && !p.ammo.ballistics().bounces {
            own_hit(&game, t.translation, next)
        } else {
            None
        };
        if let Some((at, what)) = hit {
            commands.entity(e).despawn();
            show_impact(
                &mut commands,
                &fx,
                &mut booms,
                at,
                p.ammo,
                what,
                Some(game.team),
            );
            local.0.push_back((at, p.ammo, now));
            continue;
        }
        if p.left <= 0.0 {
            commands.entity(e).despawn();
            flying.0.remove(&p.id);
            continue;
        }
        t.translation = next;
        if p.vel.length_squared() > 1e-4 {
            t.look_to(p.vel.normalize(), Vec3::Y);
        }
    }
}

fn blasts(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Blast, &mut Transform, Option<&mut PointLight>)>,
) {
    let dt = time.delta_secs();
    for (e, mut b, mut t, light) in &mut q {
        b.age += dt;
        let k = (b.age / b.life).min(1.0);
        if k >= 1.0 {
            commands.entity(e).despawn();
            continue;
        }
        t.scale = Vec3::splat((b.size * (0.3 + 0.7 * k.sqrt())).max(0.01));
        if let Some(mut l) = light {
            l.intensity *= 1.0 - (dt * 6.0).min(0.9);
        }
    }
}

/// Aircraft's flares: while they burn, white-hot sparks thrown out behind and falling.
fn flares(
    mut commands: Commands,
    game: Res<Game>,
    time: Res<Time>,
    fx: Option<Res<FxAssets>>,
    mut next: Local<f32>,
) {
    let Some(fx) = fx else { return };
    *next -= time.delta_secs();
    if *next > 0.0 {
        return;
    }
    *next = 0.07;
    for v in &game.snap.vehicles {
        if v.flags & ps_sim::snapshot::vflags::FLARES == 0 {
            continue;
        }
        let back = v.pos + v.rot * Vec3::new(0.0, v.kind.spec().half.y, v.kind.spec().half.z);
        for _ in 0..3 {
            let out = Vec3::new(rnd(-1.0, 1.0), rnd(-0.6, 0.4), rnd(0.2, 1.0)).normalize();
            let vel = v.vel * 0.5 + v.rot * out * rnd(10.0, 18.0);
            let size = rnd(0.25, 0.4);
            let mut e = particle(
                &mut commands,
                &fx.chip,
                back,
                vel,
                6.0,
                0.6,
                1.4,
                (size, 0.05),
            );
            e.insert(MeshMaterial3d(fx.hot.clone()));
        }
    }
}

/// The colour of a faction's shots.
pub fn shot_color(team: Option<Team>) -> Vec3 {
    team_color(team)
}

/// An EMP's blue burst, a flash's white one, a smoke grenade's cloud (puffs that hang as long
/// as the sim's cloud does).
fn utility_fx(commands: &mut Commands, fx: &FxAssets, at: Vec3, k: ps_sim::weapons::GrenadeKind) {
    use ps_sim::weapons::{GrenadeKind, SMOKE_RADIUS, SMOKE_TIME};
    let burst = |commands: &mut Commands, mat: Handle<ParticleMaterial>, size: f32, life: f32| {
        commands.spawn((
            Blast {
                age: 0.0,
                life,
                size,
            },
            Billboard(rnd(0.0, 6.3)),
            Mesh3d(fx.quad.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(at + Vec3::Y * 0.5).with_scale(Vec3::splat(0.1)),
        ));
    };
    match k {
        GrenadeKind::Frag => {}
        GrenadeKind::Emp => burst(commands, fx.shield.clone(), 7.0, 0.5),
        GrenadeKind::Flash => burst(commands, fx.flash.clone(), 9.0, 0.25),
        GrenadeKind::Smoke => {
            for i in 0..14 {
                let a = i as f32 * 2.4;
                let r = SMOKE_RADIUS * 0.55 * ((i % 4) as f32 / 3.0);
                let off = Vec3::new(a.cos() * r, 0.6 + (i % 3) as f32 * 1.1, a.sin() * r);
                commands.spawn((
                    Blast {
                        age: 0.0,
                        life: SMOKE_TIME,
                        size: SMOKE_RADIUS * 0.75,
                    },
                    Billboard(rnd(0.0, 6.3)),
                    Mesh3d(fx.quad.clone()),
                    MeshMaterial3d(fx.smoke.clone()),
                    Transform::from_translation(at + off).with_scale(Vec3::splat(0.1)),
                ));
            }
        }
    }
}

/// Players' jump jets (flames under their feet while they fire, one's own as predicted) and
/// chutes (a glowing canopy over whoever falls under one).
#[allow(clippy::too_many_arguments)]
fn jets_and_chutes(
    mut commands: Commands,
    game: Res<Game>,
    mover: Res<crate::game::Mover>,
    time: Res<Time>,
    fx: Option<Res<FxAssets>>,
    mut next: Local<f32>,
    mut canopies: Local<std::collections::HashMap<PlayerId, Entity>>,
    mut transforms: Query<&mut Transform>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut canopy_mesh: Local<Option<Handle<Mesh>>>,
) {
    use ps_sim::snapshot::flags;
    let Some(fx) = fx else { return };
    let mesh = canopy_mesh
        .get_or_insert_with(|| meshes.add(Sphere::new(1.4).mesh().ico(1).unwrap()))
        .clone();
    let me = game.me;
    let feet_of = |p: &ps_sim::snapshot::PlayerSnap| {
        if Some(p.id) == me && mover.valid {
            mover.body.feet
        } else {
            p.feet
        }
    };
    // Flames.
    *next -= time.delta_secs();
    if *next <= 0.0 {
        *next = 0.05;
        for p in &game.snap.players {
            let jetting = if Some(p.id) == me && mover.valid {
                mover.body.jetting
            } else {
                p.has(flags::JET)
            };
            if !jetting || p.seat.is_some() {
                continue;
            }
            let at = feet_of(p) + Vec3::Y * 0.9;
            for side in [-0.18f32, 0.18] {
                let back = Quat::from_rotation_y(p.yaw) * Vec3::new(side, 0.0, 0.2);
                let vel = Vec3::new(rnd(-0.6, 0.6), rnd(-7.0, -4.0), rnd(-0.6, 0.6));
                let mut e = particle(
                    &mut commands,
                    &fx.chip,
                    at + back,
                    vel,
                    0.0,
                    0.25,
                    0.2,
                    (rnd(0.06, 0.1), 0.02),
                );
                e.insert(MeshMaterial3d(fx.hot.clone()));
            }
        }
    }
    // Canopies.
    let chuting: Vec<(PlayerId, Vec3)> = game
        .snap
        .players
        .iter()
        .filter(|p| {
            if Some(p.id) == me && mover.valid {
                mover.body.chute
            } else {
                p.has(flags::CHUTE)
            }
        })
        .map(|p| (p.id, feet_of(p)))
        .collect();
    canopies.retain(|id, e| {
        let keep = chuting.iter().any(|c| c.0 == *id);
        if !keep {
            commands.entity(*e).try_despawn();
        }
        keep
    });
    for (id, feet) in chuting {
        let at = feet + Vec3::Y * 3.4;
        let e = *canopies.entry(id).or_insert_with(|| {
            commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(fx.hot.clone()),
                    Transform::from_translation(at).with_scale(Vec3::new(1.0, 0.35, 1.0)),
                ))
                .id()
        });
        if let Ok(mut t) = transforms.get_mut(e) {
            t.translation = at;
        }
    }
}
