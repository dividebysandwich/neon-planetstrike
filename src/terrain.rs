//! The continent around the player: the land (chunks of the heightfield, finer near the
//! camera, its roads drawn by the shader), the sea, the sky and the fog, lit and coloured by
//! the continent's theme. Rebuilt when the player arrives on another continent (`Arrived`),
//! with the scenery (src/flora.rs) and the bases and gates (src/bases.rs).

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::{DistanceFog, ExtendedMaterial, FogFalloff, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use ps_sim::Team;
use ps_sim::roads::NEAR_MAX;
use ps_sim::terrain::{CELL, HALF, N, SEA, Terrain};
use ps_sim::theme::Theme;

use crate::assets::MeshData;
use crate::desktop::MainCamera;
use crate::game::{Arrived, Game};
use crate::materials::{AccentMaterial, NeonUniform, team_color};

/// Everything of the continent shown (taken down on leaving it).
#[derive(Component)]
pub struct ContinentPart;

/// How a theme looks (linear colours).
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub low: Vec3,
    pub high: Vec3,
    pub rock: Vec3,
    pub contour: Vec3,
    pub grid: Vec3,
    pub horizon: Vec3,
    pub zenith: Vec3,
    pub water: Vec3,
    pub sun: Vec3,
    /// The scenery's neon: two colours.
    pub neon: [Vec3; 2],
    pub contour_every: f32,
}

pub fn look(theme: Theme, team: Option<Team>) -> Look {
    let v = Vec3::new;
    match theme {
        Theme::SynthDunes => Look {
            low: v(0.05, 0.022, 0.04),
            high: v(0.09, 0.04, 0.05),
            rock: v(0.03, 0.02, 0.035),
            contour: v(1.0, 0.15, 0.6),
            grid: v(0.9, 0.4, 0.05),
            horizon: v(1.0, 0.32, 0.12),
            zenith: v(0.25, 0.02, 0.3),
            water: v(0.02, 0.01, 0.05),
            sun: v(1.0, 0.6, 0.5),
            neon: [v(1.0, 0.15, 0.7), v(1.0, 0.55, 0.05)],
            contour_every: 6.0,
        },
        Theme::NeonJungle => Look {
            low: v(0.01, 0.05, 0.03),
            high: v(0.02, 0.07, 0.04),
            rock: v(0.02, 0.03, 0.03),
            contour: v(0.05, 1.0, 0.5),
            grid: v(0.0, 0.6, 0.9),
            horizon: v(0.05, 0.6, 0.55),
            zenith: v(0.06, 0.02, 0.22),
            water: v(0.0, 0.03, 0.03),
            sun: v(0.6, 1.0, 0.8),
            neon: [v(0.1, 1.0, 0.6), v(0.9, 0.2, 1.0)],
            contour_every: 8.0,
        },
        Theme::Glacier => Look {
            low: v(0.08, 0.1, 0.14),
            high: v(0.18, 0.22, 0.28),
            rock: v(0.03, 0.04, 0.06),
            contour: v(0.3, 0.9, 1.0),
            grid: v(0.6, 0.6, 1.0),
            horizon: v(0.1, 0.8, 0.5),
            zenith: v(0.02, 0.05, 0.25),
            water: v(0.01, 0.03, 0.06),
            sun: v(0.7, 0.85, 1.0),
            neon: [v(0.3, 0.9, 1.0), v(0.4, 1.0, 0.6)],
            contour_every: 10.0,
        },
        Theme::Ember => Look {
            low: v(0.03, 0.012, 0.01),
            high: v(0.05, 0.02, 0.015),
            rock: v(0.015, 0.01, 0.01),
            contour: v(1.0, 0.25, 0.02),
            grid: v(1.0, 0.05, 0.05),
            horizon: v(1.0, 0.15, 0.02),
            zenith: v(0.15, 0.0, 0.05),
            water: v(0.15, 0.02, 0.0),
            sun: v(1.0, 0.5, 0.3),
            neon: [v(1.0, 0.3, 0.02), v(1.0, 0.8, 0.1)],
            contour_every: 7.0,
        },
        Theme::FungalMarsh => Look {
            low: v(0.02, 0.04, 0.045),
            high: v(0.04, 0.04, 0.06),
            rock: v(0.02, 0.02, 0.03),
            contour: v(0.7, 0.2, 1.0),
            grid: v(1.0, 0.3, 0.6),
            horizon: v(0.6, 0.15, 0.8),
            zenith: v(0.05, 0.0, 0.15),
            water: v(0.01, 0.04, 0.04),
            sun: v(0.8, 0.6, 1.0),
            neon: [v(0.8, 0.2, 1.0), v(0.1, 1.0, 0.8)],
            contour_every: 3.0,
        },
        Theme::CircuitMesa => Look {
            low: v(0.04, 0.04, 0.05),
            high: v(0.06, 0.055, 0.06),
            rock: v(0.03, 0.025, 0.03),
            contour: v(1.0, 0.8, 0.05),
            grid: v(0.05, 0.9, 1.0),
            horizon: v(0.9, 0.5, 0.1),
            zenith: v(0.02, 0.05, 0.2),
            water: v(0.01, 0.02, 0.05),
            sun: v(1.0, 0.9, 0.7),
            neon: [v(1.0, 0.8, 0.05), v(0.05, 0.9, 1.0)],
            contour_every: 12.0,
        },
        Theme::HollowReach => Look {
            low: v(0.025, 0.015, 0.045),
            high: v(0.05, 0.03, 0.07),
            rock: v(0.03, 0.018, 0.045),
            contour: v(0.1, 0.9, 0.85),
            grid: v(0.9, 0.1, 0.7),
            horizon: v(0.35, 0.1, 0.6),
            zenith: v(0.03, 0.0, 0.1),
            water: v(0.01, 0.01, 0.04),
            sun: v(0.7, 0.6, 1.0),
            neon: [v(0.1, 1.0, 0.85), v(1.0, 0.15, 0.75)],
            contour_every: 9.0,
        },
        Theme::Haven => {
            let t = team_color(team);
            Look {
                low: v(0.03, 0.035, 0.05),
                high: v(0.05, 0.05, 0.07),
                rock: v(0.03, 0.03, 0.04),
                contour: t,
                grid: v(0.8, 0.8, 1.0),
                horizon: t * 0.6 + v(0.1, 0.1, 0.2),
                zenith: v(0.02, 0.02, 0.08),
                water: v(0.01, 0.02, 0.05),
                sun: v(0.9, 0.9, 1.0),
                neon: [t, v(1.0, 1.0, 1.0)],
                contour_every: 8.0,
            }
        }
    }
}

#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
pub struct TerrainExt {
    #[uniform(100)]
    pub lines: TerrainLines,
}

#[derive(ShaderType, Clone, Copy, Debug)]
pub struct TerrainLines {
    pub contour: Vec4,
    pub grid: Vec4,
    pub params: Vec4,
    pub shore: Vec4,
}

impl MaterialExtension for TerrainExt {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }
}

pub type TerrainMaterial = ExtendedMaterial<StandardMaterial, TerrainExt>;

/// Cells per chunk side, and how far the fine meshes show.
const CHUNK: usize = 64;
const FINE_RANGE: f32 = 650.0;
/// The fog: clear up to here, gone at `FOG_END`.
pub const FOG_START: f32 = 250.0;
pub const FOG_END: f32 = 1700.0;

#[derive(Component)]
struct Chunk {
    center: Vec3,
    fine: bool,
}

#[derive(Component)]
struct SkyDome;

/// The continent's look and its sky's material (day and night change them).
#[derive(Resource)]
struct Sky {
    look: Look,
    material: Handle<AccentMaterial>,
}

/// How much daylight there is at time of day `day` (0 midnight, 0.5 noon): 0..1.
pub fn daylight(day: f32) -> f32 {
    let elevation = ((day - 0.25) * std::f32::consts::TAU).sin();
    ((elevation + 0.12) / 0.4).clamp(0.0, 1.0)
}

/// The sun (by day) or moon (by night) going round; the light, sky, fog and the bases' lamps
/// following. Neon glows the same day and night.
#[allow(clippy::too_many_arguments)]
fn day_night(
    game: Res<Game>,
    sky: Option<Res<Sky>>,
    mut light: Query<(&mut DirectionalLight, &mut Transform)>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut accents: ResMut<Assets<AccentMaterial>>,
    mut fog: Query<&mut DistanceFog>,
    mut spots: Query<(&crate::bases::Lamp, &mut SpotLight), Without<PointLight>>,
    cave: Option<Res<crate::caves::CaveCover>>,
) {
    let Some(sky) = sky else { return };
    // Under a cave's roof: the sky's light shut out (the neon glows on).
    let under = cave.map_or(0.0, |c| c.0);
    let lk = sky.look;
    let d = daylight(game.day);
    let night = 1.0 - d;
    // The sun's (or the moon's) way across the sky.
    let a = (game.day - 0.25) * std::f32::consts::TAU;
    let sun = Vec3::new(a.cos() * 0.8, a.sin().abs().max(0.25), 0.4).normalize();
    let moon = Vec3::new(-0.4, 0.7, -0.5);
    for (mut l, mut t) in &mut light {
        let dir = if d > 0.02 { sun } else { moon.normalize() };
        *t = Transform::from_translation(dir).looking_at(Vec3::ZERO, Vec3::Y);
        let col = Vec3::new(0.35, 0.45, 0.9).lerp(lk.sun, d);
        l.color = Color::linear_rgb(col.x, col.y, col.z);
        l.illuminance = (700.0 + 5050.0 * d) * (1.0 - 0.8 * under);
    }
    let amb = Vec3::new(0.25, 0.3, 0.6).lerp(lk.horizon * 0.5 + Vec3::new(0.3, 0.3, 0.4), d);
    ambient.color = Color::linear_rgb(amb.x, amb.y, amb.z);
    ambient.brightness = (90.0 + 85.0 * d) * (1.0 - 0.45 * under);
    if let Some(mut m) = accents.get_mut(&sky.material) {
        m.neon.color.w = 0.12 + 0.88 * d;
    }
    for mut f in &mut fog {
        let c = lk.horizon * (0.04 + 0.16 * d);
        f.color = Color::linear_rgb(c.x, c.y, c.z);
    }
    // (The point lamps are indoors: on day and night.)
    for (l, mut p) in &mut spots {
        // Floodlights only after dusk.
        p.intensity = l.0 * night;
    }
}

/// The land being built (the bases and scenery are built after it).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TerrainBuild;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .add_systems(
                Update,
                (
                    build.run_if(resource_exists::<Game>).in_set(TerrainBuild),
                    lod,
                    follow_sky,
                    animate,
                    day_night.run_if(resource_exists::<Game>),
                    clear.run_if(not(resource_exists::<Game>)),
                ),
            );
    }
}

fn clear(mut commands: Commands, parts: Query<Entity, With<ContinentPart>>) {
    for e in &parts {
        commands.entity(e).despawn();
    }
}

#[allow(clippy::too_many_arguments)]
fn build(
    mut commands: Commands,
    game: Res<Game>,
    mut arrived: MessageReader<Arrived>,
    old: Query<Entity, With<ContinentPart>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_mats: ResMut<Assets<TerrainMaterial>>,
    mut std: ResMut<Assets<StandardMaterial>>,
    mut accents: ResMut<Assets<AccentMaterial>>,
    camera: Query<Entity, With<MainCamera>>,
    mut light: Query<&mut DirectionalLight>,
    mut ambient: ResMut<GlobalAmbientLight>,
) {
    if arrived.read().last().is_none() {
        return;
    }
    let Some(map) = game.map.clone() else { return };
    for e in &old {
        commands.entity(e).despawn();
    }
    let theme = map.def.theme;
    let lk = look(theme, map.def.sanctuary);
    let mat = terrain_mats.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.85,
            reflectance: 0.2,
            ..default()
        },
        extension: TerrainExt {
            lines: TerrainLines {
                contour: lk.contour.extend(1.6),
                grid: lk.grid.extend(0.35),
                params: Vec4::new(lk.contour_every, 128.0, 0.0, 900.0),
                shore: Vec4::new(
                    0.25,
                    lk.neon[0].x * 0.4,
                    lk.neon[0].y * 0.4,
                    lk.neon[0].z * 0.4,
                ),
            },
        },
    });
    let t = &map.terrain;
    // (In the caves: darker under the roofs.)
    let covered = crate::caves::cover_grid(&map);
    let chunks = (N - 1) / CHUNK;
    for cz in 0..chunks {
        for cx in 0..chunks {
            let (i0, j0) = (cx * CHUNK, cz * CHUNK);
            let mid = Vec3::new(
                (i0 + CHUNK / 2) as f32 * CELL - HALF,
                t.at((i0 + CHUNK / 2) as i32, (j0 + CHUNK / 2) as i32),
                (j0 + CHUNK / 2) as f32 * CELL - HALF,
            );
            for (step, fine) in [(1usize, true), (4usize, false)] {
                // (Only the fine meshes, seen up close, leave out the cells over the holes.)
                let holes: &[(Vec2, Vec2)] = if fine { &map.world.holes } else { &[] };
                let mesh = chunk_mesh(t, &lk, i0, j0, step, holes, covered.as_deref());
                commands.spawn((
                    ContinentPart,
                    Chunk { center: mid, fine },
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(mat.clone()),
                    Transform::IDENTITY,
                    if fine {
                        Visibility::Hidden
                    } else {
                        Visibility::Visible
                    },
                ));
            }
        }
    }
    // The sea: a sheet out to the horizon.
    commands.spawn((
        ContinentPart,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(12_000.0, 12_000.0))),
        MeshMaterial3d(std.add(StandardMaterial {
            base_color: Color::linear_rgba(lk.water.x, lk.water.y, lk.water.z, 0.88),
            emissive: LinearRgba::rgb(lk.water.x * 0.6, lk.water.y * 0.6, lk.water.z * 0.6),
            perceptual_roughness: 0.08,
            reflectance: 0.6,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_xyz(0.0, SEA, 0.0),
    ));
    // Lakes: a sheet of water over each.
    let lake_mat = std.add(StandardMaterial {
        base_color: Color::linear_rgba(lk.water.x * 1.5, lk.water.y * 1.5, lk.water.z * 1.5, 0.85),
        emissive: LinearRgba::rgb(lk.water.x, lk.water.y, lk.water.z),
        perceptual_roughness: 0.05,
        reflectance: 0.7,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    for l in &map.terrain.lakes {
        commands.spawn((
            ContinentPart,
            Mesh3d(meshes.add(Circle::new(l.radius).mesh().resolution(64).build())),
            MeshMaterial3d(lake_mat.clone()),
            Transform::from_xyz(l.center.x, l.level, l.center.y)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ));
        // A neon shoreline.
        commands.spawn((
            ContinentPart,
            Mesh3d(
                meshes.add(
                    Annulus::new(l.radius - 0.4, l.radius)
                        .mesh()
                        .resolution(64)
                        .build(),
                ),
            ),
            MeshMaterial3d(accents.add(AccentMaterial {
                neon: NeonUniform::flow(lk.neon[0], 1.6, 2.0),
                overlay: false,
            })),
            Transform::from_xyz(l.center.x, l.level + 0.03, l.center.y)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ));
    }
    // The sky.
    let sky_mat = accents.add(AccentMaterial {
        neon: NeonUniform::vertex_coloured(1.0),
        overlay: false,
    });
    commands.insert_resource(Sky {
        look: lk,
        material: sky_mat.clone(),
    });
    commands.spawn((
        ContinentPart,
        SkyDome,
        Mesh3d(meshes.add(sky_mesh(&lk))),
        MeshMaterial3d(sky_mat),
        Transform::IDENTITY,
        bevy::light::NotShadowCaster,
    ));
    for cam in &camera {
        commands.entity(cam).insert(DistanceFog {
            color: Color::linear_rgb(
                lk.horizon.x * 0.18,
                lk.horizon.y * 0.18,
                lk.horizon.z * 0.18,
            ),
            directional_light_color: Color::NONE,
            directional_light_exponent: 8.0,
            falloff: FogFalloff::Linear {
                start: FOG_START,
                end: FOG_END,
            },
        });
    }
    for mut l in &mut light {
        l.color = Color::linear_rgb(lk.sun.x, lk.sun.y, lk.sun.z);
        l.illuminance = 1500.0;
    }
    ambient.color = Color::linear_rgb(
        lk.horizon.x * 0.5 + 0.3,
        lk.horizon.y * 0.5 + 0.3,
        lk.horizon.z * 0.5 + 0.4,
    );
    ambient.brightness = 120.0;
    crate::flora::spawn(
        &mut commands,
        &map,
        &lk,
        &mut meshes,
        &mut std,
        &mut accents,
    );
    crate::caves::spawn(
        &mut commands,
        &map,
        &lk,
        &mut meshes,
        &mut std,
        &mut accents,
    );
    info!(
        "on {} ({}): {} bases, {} gates, {} scenery pieces",
        map.def.name,
        theme.name(),
        map.bases.len(),
        map.gates.len(),
        map.scenery.len()
    );
}

/// A chunk of land from grid (i0, j0), every `step` samples, with a skirt hiding seams.
/// A chunk's mesh, every `step`th vertex; the cells touching any of `holes` (XZ min, max: a
/// tower's bunker below, its apron covering the rest) are left out.
fn chunk_mesh(
    t: &Terrain,
    lk: &Look,
    i0: usize,
    j0: usize,
    step: usize,
    holes: &[(Vec2, Vec2)],
    covered: Option<&[f32]>,
) -> Mesh {
    let n = CHUNK / step + 1;
    let mut m = MeshData::default();
    // Alpha: the distance to the nearest road (terrain.wgsl draws them), 0.5 on a road's
    // middle to 1 at `NEAR_MAX` or further.
    let color = |x: f32, y: f32, z: f32, nrm: Vec3, road: f32| -> [f32; 4] {
        let k = (y / 120.0).clamp(0.0, 1.0);
        let mut c = lk.low.lerp(lk.high, k);
        let steep = ((1.0 - nrm.y) * 3.0).clamp(0.0, 1.0);
        c = c.lerp(lk.rock, steep);
        if y < SEA + 0.5 {
            c *= 0.6;
        }
        // A little variety.
        let v = ps_sim::noise::perlin(x / 37.0, z / 37.0, 9) * 0.25 + 1.0;
        (c * v)
            .extend(0.5 + 0.5 * (road / NEAR_MAX).min(1.0))
            .to_array()
    };
    for b in 0..n {
        for a in 0..n {
            let (i, j) = ((i0 + a * step) as i32, (j0 + b * step) as i32);
            let x = i as f32 * CELL - HALF;
            let z = j as f32 * CELL - HALF;
            let y = t.at(i, j);
            let nrm = t.normal(x, z);
            m.pos.push([x, y, z]);
            m.nrm.push(nrm.to_array());
            m.uv.push([x / 64.0, z / 64.0]);
            let mut c = color(x, y, z, nrm, t.road_at(i, j));
            if let Some(cov) = covered {
                let k = 1.0 - 0.7 * cov[j as usize * N + i as usize];
                for v in &mut c[..3] {
                    *v *= k;
                }
            }
            m.col.push(c);
        }
    }
    for b in 0..n - 1 {
        for a in 0..n - 1 {
            let i00 = (b * n + a) as u32;
            let i10 = i00 + 1;
            let i01 = i00 + n as u32;
            let i11 = i01 + 1;
            let lo = Vec2::from_slice(&[m.pos[i00 as usize][0], m.pos[i00 as usize][2]]);
            let hi = Vec2::from_slice(&[m.pos[i11 as usize][0], m.pos[i11 as usize][2]]);
            if holes
                .iter()
                .any(|h| lo.x < h.1.x && hi.x > h.0.x && lo.y < h.1.y && hi.y > h.0.y)
            {
                continue;
            }
            // (The same diagonal as `Terrain::height`.)
            m.idx.extend([i00, i11, i10, i00, i01, i11]);
        }
    }
    // Skirts: down from each edge.
    let edge: Vec<Vec<usize>> = vec![
        (0..n).collect(),
        (0..n).map(|a| (n - 1) * n + a).collect(),
        (0..n).map(|b| b * n).collect(),
        (0..n).map(|b| b * n + n - 1).collect(),
    ];
    for e in edge {
        let base = m.pos.len() as u32;
        for &k in &e {
            let p = m.pos[k];
            m.pos.push([p[0], p[1] - 8.0, p[2]]);
            m.nrm.push(m.nrm[k]);
            m.uv.push(m.uv[k]);
            m.col.push(m.col[k]);
        }
        for w in 0..e.len() - 1 {
            let (a, b) = (e[w] as u32, e[w + 1] as u32);
            let (c, d) = (base + w as u32, base + w as u32 + 1);
            // Both windings (seen from either side).
            m.idx.extend([a, b, d, a, d, c, a, d, b, a, c, d]);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, m.pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, m.nrm);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, m.uv);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, m.col);
    mesh.insert_indices(Indices::U32(m.idx));
    mesh
}

fn sky_mesh(lk: &Look) -> Mesh {
    let mut m = MeshData::default();
    let (seg, rings) = (48usize, 12usize);
    let r = 5000.0;
    for k in 0..=rings {
        let el = -0.15 + (k as f32 / rings as f32) * (std::f32::consts::FRAC_PI_2 + 0.15);
        let t = (el.max(0.0) / std::f32::consts::FRAC_PI_2).powf(0.6);
        let c = lk.horizon.lerp(lk.zenith, t) * (1.0 - t * 0.7) * 0.6;
        for s in 0..=seg {
            let a = s as f32 / seg as f32 * std::f32::consts::TAU;
            m.pos
                .push([a.cos() * el.cos() * r, el.sin() * r, a.sin() * el.cos() * r]);
            m.nrm.push([0.0, -1.0, 0.0]);
            m.uv.push([0.0, 0.0]);
            m.col.push(c.extend(1.0).to_array());
        }
    }
    let w = (seg + 1) as u32;
    for k in 0..rings as u32 {
        for s in 0..seg as u32 {
            let a = k * w + s;
            m.idx.extend([
                a,
                a + w,
                a + 1,
                a + 1,
                a + w,
                a + w + 1,
                a,
                a + 1,
                a + w,
                a + 1,
                a + w + 1,
                a + w,
            ]);
        }
    }
    let mut mesh = m.to_mesh();
    mesh.remove_attribute(Mesh::ATTRIBUTE_NORMAL);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[0.0f32, -1.0, 0.0]; m.pos.len()],
    );
    mesh
}

fn camera_pos(cams: &Query<&GlobalTransform, With<MainCamera>>) -> Option<Vec3> {
    cams.iter().next().map(|t| t.translation())
}

/// The fine chunks near the camera, the coarse ones elsewhere.
fn lod(
    cams: Query<&GlobalTransform, With<MainCamera>>,
    mut chunks: Query<(&Chunk, &mut Visibility)>,
) {
    let Some(eye) = camera_pos(&cams) else { return };
    for (c, mut vis) in &mut chunks {
        let d = Vec2::new(c.center.x - eye.x, c.center.z - eye.z).length();
        let near = d < FINE_RANGE;
        let want = if near == c.fine {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
}

fn follow_sky(
    cams: Query<&GlobalTransform, With<MainCamera>>,
    mut sky: Query<&mut Transform, With<SkyDome>>,
) {
    let Some(eye) = camera_pos(&cams) else { return };
    for mut t in &mut sky {
        t.translation = Vec3::new(eye.x, 0.0, eye.z);
    }
}

fn animate(time: Res<Time>, mut mats: ResMut<Assets<TerrainMaterial>>) {
    for (_, m) in mats.iter_mut() {
        m.extension.lines.params.z = time.elapsed_secs_wrapped();
    }
}
