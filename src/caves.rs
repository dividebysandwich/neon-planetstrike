//! The canyon caves drawn (`ps_sim::caves`): the roofs' rock slabs with glowing stalactites
//! under them, the land under the roofs darkened (`cover_grid`, used by src/terrain.rs), and
//! the light dimmed while the eyes are under a roof (`CaveCover`, used by the day's light).

use bevy::prelude::*;
use ps_sim::collide::Collider;
use ps_sim::map::ContinentMap;
use ps_sim::terrain::{CELL, HALF, N};

use crate::assets::MeshData;
use crate::desktop::MainCamera;
use crate::game::Game;
use crate::materials::{AccentMaterial, NeonUniform};
use crate::terrain::{ContinentPart, Look};

/// How much the eyes are under a cave's roof now (0..1, eased).
#[derive(Resource, Default)]
pub struct CaveCover(pub f32);

pub struct CavesPlugin;

impl Plugin for CavesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CaveCover>()
            .add_systems(Update, cover.run_if(resource_exists::<Game>));
    }
}

/// How covered each sample of the land is by a roof over it (0..1, feathered over a cell),
/// or nothing where there are no caves.
pub fn cover_grid(map: &ContinentMap) -> Option<Vec<f32>> {
    if map.caves.roofs.is_empty() {
        return None;
    }
    let mut hard = vec![0u8; N * N];
    let at = |v: f32| ((v + HALF) / CELL).round() as i32;
    for c in &map.caves.roofs {
        let (lo, hi) = c.bounds();
        for j in at(lo.z).max(0)..=at(hi.z).min(N as i32 - 1) {
            for i in at(lo.x).max(0)..=at(hi.x).min(N as i32 - 1) {
                let (x, z) = (i as f32 * CELL - HALF, j as f32 * CELL - HALF);
                if c.top_at(x, z).is_some() && map.terrain.at(i, j) < lo.y {
                    hard[j as usize * N + i as usize] = 1;
                }
            }
        }
    }
    let mut soft = vec![0.0f32; N * N];
    for j in 1..N - 1 {
        for i in 1..N - 1 {
            let mut s = 0u32;
            for b in j - 1..=j + 1 {
                for a in i - 1..=i + 1 {
                    s += hard[b * N + a] as u32;
                }
            }
            soft[j * N + i] = s as f32 / 9.0;
        }
    }
    Some(soft)
}

/// A box `w` thick along `p`..`q` (both windings: seen from anywhere).
fn beam(m: &mut MeshData, p: Vec3, q: Vec3, w: f32) {
    let d = (q - p).normalize_or_zero();
    let side = if d.y.abs() > 0.9 { Vec3::X } else { Vec3::Y };
    let u = d.cross(side).normalize_or_zero() * w / 2.0;
    let v = d.cross(u).normalize_or_zero() * w / 2.0;
    let corners = [u + v, u - v, -u - v, -u + v];
    for k in 0..4 {
        let (a, b) = (corners[k], corners[(k + 1) % 4]);
        let nrm = (a + b).normalize_or_zero();
        let base = m.pos.len() as u32;
        let len = (q - p).length();
        for (c, s) in [(p + a, 0.0), (p + b, 0.0), (q + b, len), (q + a, len)] {
            m.pos.push(c.to_array());
            m.nrm.push(nrm.to_array());
            m.uv.push([s, 0.0]);
            m.col.push([1.0; 4]);
        }
        m.idx
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        m.idx
            .extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    }
}

/// Spawns the ziplines: a glowing cable between masts.
fn spawn_lines(
    commands: &mut Commands,
    map: &ContinentMap,
    lk: &Look,
    meshes: &mut Assets<Mesh>,
    std: &mut Assets<StandardMaterial>,
    accents: &mut Assets<AccentMaterial>,
) {
    if map.caves.lines.is_empty() {
        return;
    }
    let (mut cable, mut masts, mut lamps) = (
        MeshData::default(),
        MeshData::default(),
        MeshData::default(),
    );
    for z in &map.caves.lines {
        beam(&mut cable, z.a, z.b, 0.07);
        for (land, end) in [(z.land_a, z.a), (z.land_b, z.b)] {
            // A mast beside the landing, over to the cable's end.
            let foot = land + (land - end).with_y(0.0).normalize_or_zero() * 0.6;
            let top = Vec3::new(foot.x, end.y + 0.5, foot.z);
            beam(&mut masts, foot, top, 0.3);
            beam(&mut masts, top, end + Vec3::Y * 0.5, 0.18);
            beam(&mut lamps, top - Vec3::Y * 0.6, top + Vec3::Y * 0.05, 0.36);
        }
    }
    let rock = std.add(StandardMaterial {
        base_color: Color::linear_rgb(lk.rock.x * 2.0, lk.rock.y * 2.0, lk.rock.z * 2.0),
        perceptual_roughness: 0.6,
        ..default()
    });
    for (m, mat) in [
        (
            cable,
            accents.add(AccentMaterial {
                neon: NeonUniform::flow(lk.neon[1], 2.5, 6.0),
                overlay: false,
            }),
        ),
        (
            lamps,
            accents.add(AccentMaterial {
                neon: NeonUniform::fixed(lk.neon[1], 3.0),
                overlay: false,
            }),
        ),
    ] {
        commands.spawn((
            ContinentPart,
            Mesh3d(meshes.add(m.to_mesh())),
            MeshMaterial3d(mat),
            Transform::IDENTITY,
        ));
    }
    commands.spawn((
        ContinentPart,
        Mesh3d(meshes.add(masts.to_mesh())),
        MeshMaterial3d(rock),
        Transform::IDENTITY,
    ));
}

/// Spawns the roofs' slabs and stalactites.
pub fn spawn(
    commands: &mut Commands,
    map: &ContinentMap,
    lk: &Look,
    meshes: &mut Assets<Mesh>,
    std: &mut Assets<StandardMaterial>,
    accents: &mut Assets<AccentMaterial>,
) {
    spawn_lines(commands, map, lk, meshes, std, accents);
    if map.caves.roofs.is_empty() {
        return;
    }
    let mut rock = MeshData::default();
    let mut glow = [MeshData::default(), MeshData::default()];
    let under = (lk.rock * 0.8).extend(1.0).to_array();
    let side = (lk.high * 0.9).extend(1.0).to_array();
    let above = lk.high.extend(1.0).to_array();
    // Neon along the edges that overhang the open (a sinkhole's rim, a skylight's).
    let mut edge = MeshData::default();
    for (k, c) in map.caves.roofs.iter().enumerate() {
        let Collider::Prism { min, max, n, pts } = *c else {
            continue;
        };
        let n = n as usize;
        let ring =
            |y: f32| -> Vec<Vec3> { (0..n).map(|i| Vec3::new(pts[i].x, y, pts[i].y)).collect() };
        let (bot, top) = (ring(min.y), ring(max.y));
        // The underside (seen from below), the top, the sides.
        let face = |m: &mut MeshData, q: &[Vec3], nrm: Vec3, col: [f32; 4]| {
            let base = m.pos.len() as u32;
            for p in q {
                m.pos.push(p.to_array());
                m.nrm.push(nrm.to_array());
                m.uv.push([p.x / 8.0, p.z / 8.0]);
                m.col.push(col);
            }
            for i in 1..q.len() as u32 - 1 {
                m.idx.extend([base, base + i, base + i + 1]);
            }
        };
        // (The corners go round anticlockwise in (x, z), which faces down in 3D: reversed
        // for the top and the sides.)
        face(&mut rock, &bot, Vec3::NEG_Y, under);
        let up: Vec<Vec3> = top.iter().rev().copied().collect();
        face(&mut rock, &up, Vec3::Y, above);
        for i in 0..n {
            let j = (i + 1) % n;
            let e = (top[j] - top[i]).normalize_or_zero();
            let out = Vec3::new(e.z, 0.0, -e.x);
            face(&mut rock, &[bot[j], bot[i], top[i], top[j]], out, side);
            // (Not where another slab carries on.)
            let mid = (bot[i] + bot[j]) / 2.0 + out * 1.5;
            let open = map.terrain.height(mid.x, mid.z) < min.y - 2.0
                && !map
                    .caves
                    .roofs
                    .iter()
                    .any(|r| r.top_at(mid.x, mid.z).is_some());
            if open {
                let o = out * 0.03;
                let up = Vec3::Y * 0.3;
                face(
                    &mut edge,
                    &[bot[j] + o, bot[i] + o, bot[i] + o + up, bot[j] + o + up],
                    out,
                    [1.0; 4],
                );
            }
        }
        // Stalactites: a few glowing spikes hanging from it.
        let h = ps_sim::noise::hash2(k as i32, 7, 0x5a1);
        for s in 0..(2 + h % 3) {
            let r = |q: u32| {
                ps_sim::noise::unit(ps_sim::noise::hash2(k as i32, (s * 5 + q) as i32, 0x5a2))
            };
            // A point in the slab's footprint: between two of its corners and the middle.
            let mid = bot.iter().copied().sum::<Vec3>() / n as f32;
            let (a, b) = (
                bot[(r(0) * n as f32) as usize % n],
                bot[(r(1) * n as f32) as usize % n],
            );
            let (u, v) = (r(2) * 0.6, r(3) * 0.6);
            let foot = mid + (a - mid) * u + (b - mid) * v - Vec3::Y * 0.05;
            let len = 1.2 + 2.2 * r(4);
            let w = 0.25 + 0.3 * r(5);
            let tip = foot - Vec3::Y * len;
            let m = &mut glow[(s as usize + k) % 2];
            for t in 0..3 {
                let ang = |q: u32| (q as f32 / 3.0 + r(6)) * std::f32::consts::TAU;
                let p0 = foot + Vec3::new(ang(t).cos(), 0.0, ang(t).sin()) * w;
                let p1 = foot + Vec3::new(ang(t + 1).cos(), 0.0, ang(t + 1).sin()) * w;
                let nrm = (p1 - p0).cross(tip - p0).normalize_or_zero();
                let base = m.pos.len() as u32;
                for p in [p0, tip, p1] {
                    m.pos.push(p.to_array());
                    m.nrm.push((-nrm).to_array());
                    m.uv.push([0.0, 0.0]);
                    m.col.push([1.0; 4]);
                }
                m.idx
                    .extend([base, base + 1, base + 2, base, base + 2, base + 1]);
            }
        }
    }
    let mat = std.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.95,
        reflectance: 0.1,
        ..default()
    });
    commands.spawn((
        ContinentPart,
        Mesh3d(meshes.add(rock.to_mesh())),
        MeshMaterial3d(mat),
        Transform::IDENTITY,
    ));
    if !edge.pos.is_empty() {
        commands.spawn((
            ContinentPart,
            Mesh3d(meshes.add(edge.to_mesh())),
            MeshMaterial3d(accents.add(AccentMaterial {
                neon: NeonUniform::fixed(lk.neon[0], 1.8),
                overlay: false,
            })),
            Transform::IDENTITY,
        ));
    }
    for (m, c) in glow.into_iter().zip(lk.neon) {
        if m.pos.is_empty() {
            continue;
        }
        commands.spawn((
            ContinentPart,
            Mesh3d(meshes.add(m.to_mesh())),
            MeshMaterial3d(accents.add(AccentMaterial {
                neon: NeonUniform::fixed(c, 2.2),
                overlay: false,
            })),
            Transform::IDENTITY,
        ));
    }
}

/// Eases how covered the eyes are.
fn cover(
    game: Res<Game>,
    time: Res<Time>,
    cams: Query<&GlobalTransform, With<MainCamera>>,
    mut cover: ResMut<CaveCover>,
) {
    let under = match (game.map.as_ref(), cams.iter().next()) {
        (Some(m), Some(t)) if !m.caves.roofs.is_empty() => {
            ps_sim::caves::roofed(&m.caves, t.translation(), 80.0)
        }
        _ => false,
    };
    let want = if under { 1.0 } else { 0.0 };
    let k = (time.delta_secs() / 0.6).min(1.0);
    cover.0 += (want - cover.0) * k;
}
