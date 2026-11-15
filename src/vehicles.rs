//! Vehicles on screen, following `Game::snap` (the one the player drives, as its prediction
//! has it): hulls with their turrets and barrels turned to the gunners' aim, the bus's, the
//! harvester's and the rover's wheels, the bus's mast, the VTOL's tilting engines, a cloak
//! bubble round a deployed bus. Models from
//! vehicles.glb (stand-in boxes without it).

use std::collections::HashMap;

use bevy::prelude::*;
use ps_sim::snapshot::{VehicleSnap, vflags};
use ps_sim::vehicle::VehicleKind;
use ps_sim::weapons::Ammo;
use ps_sim::{Team, VehicleId};

use crate::assets::{Model, Models};
use crate::bases::{Lamp, spawn_plated};
use crate::game::{DrawSet, Game, Mover, alpha};
use crate::hull::{HullMaterial, follow, vehicle_armor, vehicle_hull};
use crate::materials::{AccentMaterial, GlowMaterial, NeonUniform, Palette, team_color};

struct VehicleView {
    root: Entity,
    /// Per gun: the turret (yaw) and barrel (pitch) entities.
    guns: Vec<(Entity, Entity)>,
    /// Wheels (and a tracked one's road wheels and sprockets), each with its radius.
    wheels: Vec<(Entity, f32)>,
    mast: Option<Entity>,
    engines: Vec<Entity>,
    /// The engines' fans, and how far they have turned.
    fans: Vec<Entity>,
    fan_turn: f32,
    /// Per gun: how far its barrel is pushed back by its last shot (m).
    recoil: Vec<f32>,
    /// The tracks' running light (its `tread` parts' own material) and which way it runs now
    /// (`tread_gear`).
    tread: Option<(Handle<AccentMaterial>, i8)>,
    bubble: Entity,
    team: Team,
    /// How far it has rolled forwards (m): what its wheels have turned by.
    roll: f32,
    /// Its plating: the dark body's and the armour plates'.
    hull: [Handle<HullMaterial>; 2],
    beam: Option<Entity>,
}

#[derive(Resource, Default)]
struct Views {
    vehicles: HashMap<VehicleId, VehicleView>,
    continent: Option<u8>,
}

pub struct VehiclesPlugin;

impl Plugin for VehiclesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Views>()
            .add_systems(
                Update,
                sync.run_if(resource_exists::<Game>)
                    .in_set(DrawSet)
                    .after(crate::fx::FxSet),
            )
            .add_systems(Update, clear.run_if(not(resource_exists::<Game>)));
    }
}

fn clear(mut commands: Commands, mut views: ResMut<Views>) {
    for (_, v) in views.vehicles.drain() {
        commands.entity(v.root).despawn();
    }
}

pub fn prefix(kind: VehicleKind) -> &'static str {
    match kind {
        VehicleKind::LightTank => "light_tank",
        VehicleKind::LightAaa => "light_aaa",
        VehicleKind::HeavyTank => "heavy_tank",
        VehicleKind::SpawnBus => "spawn_bus",
        VehicleKind::Vtol => "vtol",
        VehicleKind::Harvester => "harvester",
        VehicleKind::Rover => "rover",
        VehicleKind::Interceptor => "interceptor",
        VehicleKind::Dropship => "dropship",
    }
}

/// How far a shot of `ammo` pushes its barrel back (m; 0: not at all).
fn kick(ammo: Ammo) -> f32 {
    match ammo {
        Ammo::HeavyShell | Ammo::Rail => 0.42,
        Ammo::LightShell => 0.28,
        Ammo::Autocannon => 0.1,
        Ammo::Flak => 0.09,
        Ammo::Chaingun => 0.04,
        _ => 0.0,
    }
}

/// Which way and how fast a track's light runs at `speed` (m/s forwards): -1, 0, 1 or 2.
fn tread_gear(speed: f32) -> i8 {
    match speed {
        s if s < -0.6 => -1,
        s if s < 0.6 => 0,
        s if s < 8.0 => 1,
        _ => 2,
    }
}

fn boxy(meshes: &mut Assets<Mesh>, size: Vec3, at: Vec3) -> Model {
    Model {
        parts: vec![
            (
                "body".into(),
                meshes.add(Cuboid::from_size(size).mesh().build().translated_by(at)),
            ),
            (
                "accent".into(),
                meshes.add(
                    Cuboid::new(size.x + 0.05, 0.1, size.z + 0.05)
                        .mesh()
                        .build()
                        .translated_by(at + Vec3::Y * (size.y * 0.3)),
                ),
            ),
        ],
        nodes: HashMap::new(),
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn spawn_vehicle(
    commands: &mut Commands,
    v: &VehicleSnap,
    models: &Models,
    palette: &Palette,
    meshes: &mut Assets<Mesh>,
    glows: &mut Assets<GlowMaterial>,
    accents: &mut Assets<AccentMaterial>,
    hulls: &mut Assets<HullMaterial>,
) -> VehicleView {
    let team = Some(v.team);
    let hull_mat = [
        hulls.add(vehicle_hull(team_color(team) * 0.6)),
        hulls.add(vehicle_armor(team_color(team))),
    ];
    // (A tracked one's `tread` lines get a material of their own: they run with it.)
    let tread = matches!(v.kind.spec().motion, ps_sim::vehicle::MotionKind::Tracked).then(|| {
        accents.add(AccentMaterial {
            overlay: false,
            neon: NeonUniform::flow(team_color(team), 3.0, 0.0),
        })
    });
    let spawn_model =
        |commands: &mut Commands, e: Entity, m: &Model, palette: &Palette, team: Option<Team>| {
            let mut rest = m.clone();
            if let Some(t) = &tread {
                rest.parts.retain(|(mat, mesh)| {
                    if mat != "tread" {
                        return true;
                    }
                    commands.spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(t.clone()),
                        Transform::IDENTITY,
                        ChildOf(e),
                    ));
                    false
                });
            }
            spawn_plated(
                commands,
                e,
                &rest,
                palette,
                team,
                None,
                Some((&hull_mat[0], &hull_mat[1])),
            );
        };
    let spec = v.kind.spec();
    let p = prefix(v.kind);
    let get = |name: &str, fallback: Model| -> Model {
        if models.has(name) {
            models.get(name)
        } else {
            fallback
        }
    };
    let root = commands
        .spawn((
            Transform::from_translation(v.pos).with_rotation(v.rot),
            Visibility::default(),
        ))
        .id();
    let h = spec.half;
    let hull_name = if v.kind == VehicleKind::Vtol {
        "vtol_body".to_string()
    } else {
        format!("{p}_hull")
    };
    let hull_fallback = boxy(
        meshes,
        Vec3::new(h.x * 2.0, h.y * 1.2, h.z * 2.0),
        Vec3::Y * h.y * 0.6,
    );
    let hull = get(&hull_name, hull_fallback);
    let he = commands
        .spawn((Transform::IDENTITY, Visibility::default(), ChildOf(root)))
        .id();
    spawn_model(commands, he, &hull, palette, team);
    let mut guns = Vec::new();
    for (gi, g) in spec.guns.iter().enumerate() {
        let (tn, bn) = if v.kind == VehicleKind::HeavyTank && gi == 1 {
            (
                "heavy_tank_mg_turret".to_string(),
                "heavy_tank_mg".to_string(),
            )
        } else {
            (format!("{p}_turret"), format!("{p}_barrel"))
        };
        let turret = commands
            .spawn((
                Transform::from_translation(g.pivot),
                Visibility::default(),
                ChildOf(root),
            ))
            .id();
        let barrel = commands
            .spawn((Transform::IDENTITY, Visibility::default(), ChildOf(turret)))
            .id();
        if !g.fixed {
            let tf = boxy(meshes, Vec3::new(h.x * 1.1, 0.7, h.x * 1.4), Vec3::ZERO);
            spawn_model(commands, turret, &get(&tn, tf), palette, team);
            let bf = boxy(
                meshes,
                Vec3::new(0.25, 0.25, g.barrel),
                Vec3::new(0.0, 0.0, -g.barrel / 2.0),
            );
            spawn_model(commands, barrel, &get(&bn, bf), palette, team);
        }
        guns.push((turret, barrel));
    }
    let mut wheels = Vec::new();
    let mut mast = None;
    let mut engines = Vec::new();
    let mut fans = Vec::new();
    match v.kind {
        VehicleKind::Rover => {
            const WHEEL_R: f32 = 0.5;
            let wheel = get(
                "rover_wheel",
                boxy(meshes, Vec3::new(0.4, 1.0, 1.0), Vec3::ZERO),
            );
            for z in [-1.45f32, 1.45] {
                for x in [-1.1f32, 1.1] {
                    let w = commands
                        .spawn((
                            Transform::from_xyz(x, 0.5, z),
                            Visibility::default(),
                            ChildOf(root),
                        ))
                        .id();
                    spawn_model(commands, w, &wheel, palette, team);
                    wheels.push((w, WHEEL_R));
                }
            }
        }
        VehicleKind::Harvester => {
            const WHEEL_R: f32 = 0.9;
            let wheel = get(
                "harvester_wheel",
                boxy(meshes, Vec3::new(0.56, 1.8, 1.8), Vec3::ZERO),
            );
            for z in [-3.8f32, 0.0, 3.8] {
                for x in [-1.85f32, 1.85] {
                    let w = commands
                        .spawn((
                            Transform::from_xyz(x, 0.9, z),
                            Visibility::default(),
                            ChildOf(root),
                        ))
                        .id();
                    spawn_model(commands, w, &wheel, palette, team);
                    wheels.push((w, WHEEL_R));
                }
            }
        }
        VehicleKind::SpawnBus => {
            const WHEEL_R: f32 = 0.7;
            let wheel = get(
                "spawn_bus_wheel",
                boxy(meshes, Vec3::new(0.5, 1.4, 1.4), Vec3::ZERO),
            );
            for z in [-3.4f32, 0.0, 3.4] {
                for x in [-1.55f32, 1.55] {
                    let w = commands
                        .spawn((
                            Transform::from_xyz(x, 0.7, z),
                            Visibility::default(),
                            ChildOf(root),
                        ))
                        .id();
                    spawn_model(commands, w, &wheel, palette, team);
                    wheels.push((w, WHEEL_R));
                }
            }
            let m = commands
                .spawn((
                    Transform::from_xyz(0.0, 3.6, 1.5).with_scale(Vec3::new(1.0, 0.05, 1.0)),
                    Visibility::default(),
                    ChildOf(root),
                ))
                .id();
            spawn_model(
                commands,
                m,
                &get(
                    "spawn_bus_mast",
                    boxy(meshes, Vec3::new(0.3, 4.0, 0.3), Vec3::Y * 2.0),
                ),
                palette,
                team,
            );
            mast = Some(m);
        }
        VehicleKind::Vtol => {
            let engine = get(
                "vtol_engine",
                boxy(meshes, Vec3::new(1.0, 1.0, 2.2), Vec3::ZERO),
            );
            for x in [-2.6f32, 2.6] {
                let e = commands
                    .spawn((
                        Transform::from_xyz(x, 1.1, 0.2),
                        Visibility::default(),
                        ChildOf(root),
                    ))
                    .id();
                spawn_model(commands, e, &engine, palette, team);
                engines.push(e);
                if models.has("vtol_fan") {
                    let f = commands
                        .spawn((Transform::IDENTITY, Visibility::default(), ChildOf(e)))
                        .id();
                    spawn_model(commands, f, &models.get("vtol_fan"), palette, team);
                    fans.push(f);
                }
            }
        }
        VehicleKind::LightTank | VehicleKind::LightAaa | VehicleKind::HeavyTank => {
            // The running gear inside the tracks (vehicles.py: LT_TRACK, HT_TRACK): road
            // wheels on the ground run, a sprocket at each end. (None without the models.)
            let heavy = v.kind == VehicleKind::HeavyTank;
            let (name, x, road, ends): (_, f32, (f32, f32, &[f32]), (f32, f32, f32)) = if heavy {
                (
                    "heavy_tank",
                    1.785,
                    (0.46, 0.38, &[-2.5, -1.5, -0.5, 0.5, 1.5, 2.5]),
                    (0.75, 0.34, 3.3),
                )
            } else {
                (
                    "light_tank",
                    1.29,
                    (0.38, 0.31, &[-1.7, -0.85, 0.0, 0.85, 1.7]),
                    (0.6, 0.3, 2.35),
                )
            };
            let mut place = |mesh: String, y: f32, r: f32, zs: &[f32]| {
                if !models.has(&mesh) {
                    return;
                }
                let model = models.get(&mesh);
                for &z in zs {
                    for sx in [-x, x] {
                        let w = commands
                            .spawn((
                                Transform::from_xyz(sx, y, z),
                                Visibility::default(),
                                ChildOf(root),
                            ))
                            .id();
                        spawn_model(commands, w, &model, palette, team);
                        wheels.push((w, r));
                    }
                }
            };
            place(format!("{name}_wheel"), road.0, road.1, road.2);
            place(
                format!("{name}_sprocket"),
                ends.0,
                ends.1,
                &[-ends.2, ends.2],
            );
        }
        _ => {}
    }
    let bubble = commands
        .spawn((
            Mesh3d(meshes.add(Sphere::new(ps_sim::sim::CLOAK_SEEN).mesh().ico(3).unwrap())),
            MeshMaterial3d(glows.add(GlowMaterial {
                neon: NeonUniform::fixed(team_color(team), 0.5).shield(),
                overlay: false,
            })),
            Transform::IDENTITY,
            Visibility::Hidden,
            ChildOf(root),
        ))
        .id();
    // Headlights (lit at night).
    if !v.kind.aircraft() {
        for x in [-h.x * 0.6, h.x * 0.6] {
            commands.spawn((
                SpotLight {
                    color: Color::linear_rgb(1.0, 0.95, 0.85),
                    intensity: 0.0,
                    range: 70.0,
                    outer_angle: 0.5,
                    inner_angle: 0.3,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Lamp(1_500_000.0),
                crate::bases::Near(250.0),
                Transform::from_xyz(x, h.y, -h.z - 0.2)
                    .looking_to(Vec3::new(0.0, -0.15, -1.0), Vec3::Y),
                ChildOf(root),
            ));
        }
    }
    // The harvester's beam: a hot core in a wider glow, shown only while it mines.
    let beam = (v.kind == VehicleKind::Harvester).then(|| {
        let root = commands
            .spawn((Transform::IDENTITY, Visibility::Hidden))
            .id();
        for (r, c, i) in [
            (0.07, Vec3::new(1.0, 0.85, 0.5), 6.0),
            (0.32, Vec3::new(1.0, 0.5, 0.08), 1.2),
        ] {
            commands.spawn((
                Mesh3d(meshes.add(Cylinder::new(r, 1.0).mesh().resolution(12).build())),
                MeshMaterial3d(glows.add(GlowMaterial {
                    neon: NeonUniform::fixed(c, i),
                    overlay: false,
                })),
                Transform::IDENTITY,
                ChildOf(root),
            ));
        }
        root
    });
    VehicleView {
        hull: hull_mat,
        beam,
        root,
        guns,
        wheels,
        mast,
        engines,
        fans,
        fan_turn: 0.0,
        recoil: vec![0.0; spec.guns.len()],
        tread: tread.map(|t| (t, 0)),
        bubble,
        team: v.team,
        roll: 0.0,
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn sync(
    mut commands: Commands,
    game: Res<Game>,
    mover: Res<Mover>,
    mut views: ResMut<Views>,
    models: Res<Models>,
    palette: Res<Palette>,
    (mut meshes, mut glows, mut accents, mut hulls): (
        ResMut<Assets<Mesh>>,
        ResMut<Assets<GlowMaterial>>,
        ResMut<Assets<AccentMaterial>>,
        ResMut<Assets<HullMaterial>>,
    ),
    (mut fx, local): (
        MessageReader<crate::game::FxEvent>,
        Option<Res<crate::fx::LocalShots>>,
    ),
    time: Res<Time<Fixed>>,
    real: Res<Time>,
    mut transforms: Query<&mut Transform>,
    mut vis: Query<&mut Visibility>,
    sparks: Option<Res<crate::bases::BaseAssets>>,
    (view, look): (
        Res<crate::seatview::SeatView>,
        Option<Res<crate::desktop::Look>>,
    ),
) {
    let a = alpha(&time);
    // Where shells and bolts left a barrel this frame (others' and one's own): the guns there
    // recoil.
    let mut shots: Vec<(Ammo, Vec3)> = fx
        .read()
        .filter_map(|e| match e.0 {
            ps_sim::snapshot::Fx::Shot { ammo, from, .. } => Some((ammo, from)),
            _ => None,
        })
        .collect();
    if let Some(l) = &local {
        shots.extend(l.fired.iter().copied());
    }
    shots.retain(|(ammo, _)| kick(*ammo) > 0.0);
    // (The one the player sits in isn't drawn from inside.)
    let inside = crate::seatview::from_inside(&view, look.as_deref())
        .then_some(view.vehicle)
        .flatten();
    let views = &mut *views;
    if views.continent != game.continent {
        views.continent = game.continent;
        for (_, v) in views.vehicles.drain() {
            commands.entity(v.root).despawn();
        }
    }
    let gone: Vec<VehicleId> = views
        .vehicles
        .iter()
        .filter(|(id, v)| {
            !game
                .snap
                .vehicles
                .iter()
                .any(|s| s.id == **id && s.team == v.team)
        })
        .map(|(id, _)| *id)
        .collect();
    for id in gone {
        if let Some(v) = views.vehicles.remove(&id) {
            commands.entity(v.root).despawn();
            if let Some(b) = v.beam {
                commands.entity(b).despawn();
            }
        }
    }
    let dt = real.delta_secs();
    for s in &game.snap.vehicles {
        let v = views.vehicles.entry(s.id).or_insert_with(|| {
            spawn_vehicle(
                &mut commands,
                s,
                &models,
                &palette,
                &mut meshes,
                &mut glows,
                &mut accents,
                &mut hulls,
            )
        });
        let prev = game.prev.vehicles.iter().find(|q| q.id == s.id);
        let (pos, rot) = match &mover.drive {
            Some((id, _, m)) if *id == s.id => (m.pos, m.rot),
            _ => match prev {
                Some(q) if (q.pos - s.pos).length() < 30.0 => {
                    (q.pos.lerp(s.pos, a), q.rot.slerp(s.rot, a))
                }
                _ => (s.pos, s.rot),
            },
        };
        if let Ok(mut t) = transforms.get_mut(v.root) {
            t.translation = pos;
            t.rotation = rot;
        }
        if let Ok(mut vv) = vis.get_mut(v.root) {
            let want = if inside == Some(s.id) {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
            if *vv != want {
                *vv = want;
            }
        }
        let at = GlobalTransform::from(Transform::from_translation(pos).with_rotation(rot));
        for h in &v.hull {
            if let Some(mut m) = hulls.get_mut(h) {
                follow(&mut m, &at);
            }
        }
        // The harvester's beam, to the node it mines.
        if let Some(beam) = v.beam {
            let on = s.flags & vflags::HARVESTING != 0;
            let spec = s.kind.spec();
            let g = spec.guns[0];
            let (yaw, pitch) = s
                .aim
                .first()
                .map_or((0.0, 0.0), |a| (a.0 as f32 / 1e4, a.1 as f32 / 1e4));
            let aim = rot * Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch);
            let from = pos + rot * g.pivot + aim * Vec3::new(0.0, 0.0, -g.barrel);
            let dir = aim * Vec3::NEG_Z;
            // (The node it mines: the nearest, as the sim picks it.)
            let pivot = pos + rot * g.pivot;
            let to = game
                .snap
                .nodes
                .iter()
                .filter(|n| n.amount > 0)
                .map(|n| n.pos + Vec3::Y * 1.5)
                .min_by(|a, b| (*a - pivot).length().total_cmp(&(*b - pivot).length()))
                .filter(|c| (*c - pivot).length() < ps_sim::sim::HARVEST_REACH + 4.0)
                .unwrap_or(from + dir * 20.0);
            if let Ok(mut vv) = vis.get_mut(beam) {
                *vv = if on {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
            if on && let Some(assets) = &sparks {
                for _ in 0..2 {
                    crate::bases::harvest_spark(&mut commands, assets, to, from);
                }
            }
            if on && let Ok(mut t) = transforms.get_mut(beam) {
                let d = to - from;
                *t = Transform::from_translation(from + d / 2.0)
                    .with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize_or(Vec3::Y)))
                    .with_scale(Vec3::new(
                        1.0 + 0.12 * (real.elapsed_secs() * 31.0).sin(),
                        d.length(),
                        1.0 + 0.12 * (real.elapsed_secs() * 31.0).sin(),
                    ));
            }
        }
        // Unloading into a silo: ore pouring in.
        if s.flags & vflags::UNLOADING != 0
            && let Some(assets) = &sparks
            && let Some(silo) = game.map.as_ref().and_then(|m| {
                m.bases
                    .iter()
                    .filter_map(|b| b.silo)
                    .min_by(|a, b| (*a - pos).length().total_cmp(&(*b - pos).length()))
            })
        {
            for _ in 0..2 {
                crate::bases::unload_fx(&mut commands, assets, pos + Vec3::Y * 2.5, silo);
            }
        }
        let spec = s.kind.spec();
        for (i, &(turret, barrel)) in v.guns.iter().enumerate() {
            let Some(g) = spec.guns.get(i) else {
                continue;
            };
            let (yaw, pitch) = s
                .aim
                .get(i)
                .map_or((0.0, 0.0), |a| (a.0 as f32 / 1e4, a.1 as f32 / 1e4));
            let aim = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch);
            // A shot from its muzzle kicks the barrel back; a spring returns it.
            if !g.fixed {
                let muzzle = pos + rot * (g.pivot + aim * Vec3::new(0.0, 0.0, -g.barrel));
                let reach = g.barrel_gap * g.barrels as f32 * 0.5 + 2.5;
                for (ammo, from) in &shots {
                    if from.distance_squared(muzzle) < reach * reach {
                        v.recoil[i] = v.recoil[i].max(kick(*ammo));
                    }
                }
            }
            v.recoil[i] *= (-9.0 * dt).exp();
            if s.aim.get(i).is_some()
                && let Ok(mut t) = transforms.get_mut(turret)
            {
                t.rotation = Quat::from_rotation_y(yaw);
            }
            if let Ok(mut t) = transforms.get_mut(barrel) {
                if s.aim.get(i).is_some() {
                    t.rotation = Quat::from_rotation_x(pitch);
                }
                t.translation = t.rotation * Vec3::new(0.0, 0.0, v.recoil[i]);
            }
        }
        let speed = s.vel.dot(rot * Vec3::NEG_Z);
        // The tracks' light runs the way they do (in steps: the shader's pulses would jump
        // with a speed that slides).
        if let Some((h, gear)) = &mut v.tread {
            let want = tread_gear(speed);
            if want != *gear
                && let Some(mut m) = accents.get_mut(&*h)
            {
                *gear = want;
                m.neon.extra.y = want as f32 * 2.5;
            }
        }
        v.fan_turn = (v.fan_turn + (22.0 + speed.abs() * 0.4) * dt) % std::f32::consts::TAU;
        for &f in &v.fans {
            if let Ok(mut t) = transforms.get_mut(f) {
                t.rotation = Quat::from_rotation_y(v.fan_turn);
            }
        }
        // (Wrapped where every wheel has turned whole turns or near enough, so the angle
        // keeps its precision on a long drive.)
        v.roll = (v.roll + speed * dt) % 1000.0;
        for &(w, r) in &v.wheels {
            if let Ok(mut t) = transforms.get_mut(w) {
                t.rotation = Quat::from_rotation_x(-v.roll / r);
            }
        }
        let deployed = s.flags & vflags::DEPLOYED != 0;
        if let Some(m) = v.mast
            && let Ok(mut t) = transforms.get_mut(m)
        {
            let want = if deployed { 1.0 } else { 0.05 };
            t.scale.y += (want - t.scale.y) * (dt * 2.0).min(1.0);
        }
        for &e in &v.engines {
            if let Ok(mut t) = transforms.get_mut(e) {
                // Tilted forward with speed (hovering: upright).
                let tilt = -(speed / 55.0).clamp(0.0, 1.0) * 1.2;
                t.rotation = Quat::from_rotation_x(tilt);
            }
        }
        if let Ok(mut b) = vis.get_mut(v.bubble) {
            let want = if deployed {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *b != want {
                *b = want;
            }
        }
    }
}
