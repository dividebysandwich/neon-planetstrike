//! The continent's scenery drawn: every piece `ps_sim::scenery::place` put there (the same on
//! the server, which collides with the solid ones), built from a few procedural shapes per
//! kind (dark bodies, neon in the theme's two colours) and merged into one mesh per material
//! for each square of the map, hidden beyond the fog.

use std::f32::consts::{PI, TAU};

use bevy::math::Affine3A;
use bevy::prelude::*;
use ps_sim::map::ContinentMap;
use ps_sim::noise;
use ps_sim::scenery::Item;
use ps_sim::terrain::HALF;
use ps_sim::theme::SceneryKind;

use crate::assets::MeshData;
use crate::desktop::MainCamera;
use crate::materials::{AccentMaterial, NeonUniform};
use crate::terrain::{ContinentPart, FOG_END, Look};

/// The map squares scenery is merged by (m).
const REGION: f32 = 256.0;

#[derive(Component)]
struct Region(Vec2);

/// A kind's geometry: dark body, and neon in the theme's first and second colour.
#[derive(Default)]
struct Parts {
    body: MeshData,
    neon: [MeshData; 2],
}

/// Appends a frustum around +Y from `y0` (radius `r0`) to `y1` (`r1`), `sides` faces, flat
/// shaded, transformed by `t`.
#[allow(clippy::too_many_arguments)]
fn frustum(
    m: &mut MeshData,
    t: Affine3A,
    sides: usize,
    r0: f32,
    r1: f32,
    y0: f32,
    y1: f32,
    phase: f32,
) {
    let pt = |a: f32, r: f32, y: f32| t.transform_point3(Vec3::new(a.cos() * r, y, a.sin() * r));
    for i in 0..sides {
        let a0 = phase + i as f32 / sides as f32 * TAU;
        let a1 = phase + (i + 1) as f32 / sides as f32 * TAU;
        quad(
            m,
            [
                pt(a0, r0, y0),
                pt(a1, r0, y0),
                pt(a1, r1, y1),
                pt(a0, r1, y1),
            ],
        );
    }
    if r1 > 0.0 {
        let top: Vec<Vec3> = (0..sides)
            .map(|i| pt(phase + i as f32 / sides as f32 * TAU, r1, y1))
            .collect();
        fan(m, &top, true);
    }
}

/// A flat-shaded quad (a, b, c, d anticlockwise seen from its front).
fn quad(m: &mut MeshData, q: [Vec3; 4]) {
    let n = (q[1] - q[0]).cross(q[3] - q[0]).normalize_or(Vec3::Y);
    // (Outward: the frustum's sides are listed so that the normal points away from the axis.)
    let n = -n;
    let base = m.pos.len() as u32;
    for p in q {
        m.pos.push(p.to_array());
        m.nrm.push(n.to_array());
        m.uv.push([0.0, 0.0]);
    }
    m.idx
        .extend([base, base + 2, base + 1, base, base + 3, base + 2]);
}

fn tri2(m: &mut MeshData, a: Vec3, b: Vec3, c: Vec3) {
    // Both sides (leaves, blades).
    let n = (b - a).cross(c - a).normalize_or(Vec3::Y);
    let base = m.pos.len() as u32;
    for p in [a, b, c] {
        m.pos.push(p.to_array());
        m.nrm.push(n.to_array());
        m.uv.push([0.0, 0.0]);
    }
    m.idx
        .extend([base, base + 1, base + 2, base, base + 2, base + 1]);
}

fn fan(m: &mut MeshData, ring: &[Vec3], up: bool) {
    let c = ring.iter().copied().sum::<Vec3>() / ring.len() as f32;
    let n = if up { Vec3::Y } else { Vec3::NEG_Y };
    let base = m.pos.len() as u32;
    m.pos.push(c.to_array());
    m.nrm.push(n.to_array());
    m.uv.push([0.0, 0.0]);
    for p in ring {
        m.pos.push(p.to_array());
        m.nrm.push(n.to_array());
        m.uv.push([0.0, 0.0]);
    }
    let k = ring.len() as u32;
    for i in 0..k {
        let (a, b) = (base + 1 + i, base + 1 + (i + 1) % k);
        if up {
            m.idx.extend([base, b, a]);
        } else {
            m.idx.extend([base, a, b]);
        }
    }
}

/// A squashed low-poly ball.
fn ball(m: &mut MeshData, t: Affine3A, r: f32, squash: f32, sides: usize) {
    let rings = 4;
    for k in 0..rings {
        let e0 = -PI / 2.0 + k as f32 / rings as f32 * PI;
        let e1 = -PI / 2.0 + (k + 1) as f32 / rings as f32 * PI;
        let (y0, y1) = (e0.sin() * r * squash, e1.sin() * r * squash);
        frustum_open(m, t, sides, e0.cos() * r, e1.cos() * r, y0, y1);
    }
}

fn frustum_open(m: &mut MeshData, t: Affine3A, sides: usize, r0: f32, r1: f32, y0: f32, y1: f32) {
    let pt = |a: f32, r: f32, y: f32| t.transform_point3(Vec3::new(a.cos() * r, y, a.sin() * r));
    for i in 0..sides {
        let a0 = i as f32 / sides as f32 * TAU;
        let a1 = (i + 1) as f32 / sides as f32 * TAU;
        quad(
            m,
            [
                pt(a0, r0, y0),
                pt(a1, r0, y0),
                pt(a1, r1, y1),
                pt(a0, r1, y1),
            ],
        );
    }
}

/// A crystal: two pyramids base to base.
fn crystal(m: &mut MeshData, t: Affine3A, r: f32, h: f32) {
    frustum_open(m, t, 4, r * 0.7, r, 0.0, h * 0.7);
    frustum_open(m, t, 4, r, 0.0, h * 0.7, h);
}

fn at(pos: Vec3, rot: Quat, scale: f32) -> Affine3A {
    Affine3A::from_scale_rotation_translation(Vec3::splat(scale), rot, pos)
}

/// The geometry of one piece of scenery, appended to `p`.
fn build(p: &mut Parts, it: &Item) {
    let base = at(it.pos, Quat::from_rotation_y(it.yaw), it.scale);
    let h = it.hash;
    let r = |k: u32| noise::unit(h.rotate_left(k));
    use SceneryKind::*;
    match it.kind {
        Rock => {
            frustum(
                &mut p.body,
                base,
                6,
                1.3,
                0.6 + r(3) * 0.4,
                -0.5,
                1.2 + r(5) * 0.6,
                r(7),
            );
        }
        Boulder => {
            ball(
                &mut p.body,
                base * Affine3A::from_translation(Vec3::Y * 1.2),
                3.0,
                0.7,
                7,
            );
            frustum_open(&mut p.neon[0], base, 7, 3.05, 3.05, 1.1, 1.2);
        }
        Crystal => {
            frustum(&mut p.body, base, 5, 1.0, 0.7, -0.3, 0.4, 0.0);
            for k in 0..3 {
                let a = k as f32 / 3.0 * TAU + r(k * 4);
                let tilt = Quat::from_rotation_z(0.35) * Quat::from_rotation_y(a);
                let t = base
                    * Affine3A::from_rotation_translation(
                        tilt,
                        Vec3::new(a.cos() * 0.4, 0.2, a.sin() * 0.4),
                    );
                crystal(&mut p.neon[(k % 2) as usize], t, 0.35, 2.4 + r(k + 9) * 1.4);
            }
        }
        Obelisk => {
            frustum(&mut p.body, base, 4, 1.3, 0.6, -1.0, 12.0, PI / 4.0);
            for y in [3.0f32, 6.5, 10.0] {
                let k = (12.0 - y) / 13.0;
                let rr = 0.6 + 0.7 * k + 0.03;
                frustum_open(&mut p.neon[0], base, 4, rr, rr, y, y + 0.25);
            }
            crystal(
                &mut p.neon[1],
                base * Affine3A::from_translation(Vec3::Y * 12.0),
                0.5,
                1.6,
            );
        }
        Palm => {
            let lean = Quat::from_rotation_z(0.12 * (r(2) - 0.5));
            let t = base * Affine3A::from_quat(lean);
            frustum_open(&mut p.body, t, 6, 0.35, 0.2, 0.0, 9.0);
            let top = Vec3::Y * 9.0;
            for k in 0..7 {
                let a = k as f32 / 7.0 * TAU + r(k);
                let d = Vec3::new(a.cos(), 0.0, a.sin());
                let side = Vec3::new(-d.z, 0.0, d.x) * 0.7;
                let tip = top + d * 4.2 - Vec3::Y * 2.2;
                let mid = top + d * 2.2 + Vec3::Y * 0.4;
                let (tp, tm, tt) = (
                    t.transform_point3(top),
                    t.transform_point3(mid),
                    t.transform_point3(tip),
                );
                let (ms1, ms2) = (
                    t.transform_point3(mid + side),
                    t.transform_point3(mid - side),
                );
                tri2(&mut p.body, tp, ms1, tt);
                tri2(&mut p.body, tp, tt, ms2);
                // A glowing vein down the leaf.
                tri2(
                    &mut p.neon[0],
                    tp + Vec3::Y * 0.04,
                    tm + Vec3::Y * 0.06,
                    tt + Vec3::Y * 0.04,
                );
            }
            ball(
                &mut p.neon[1],
                t * Affine3A::from_translation(top - Vec3::Y * 0.4),
                0.35,
                1.0,
                5,
            );
        }
        Fern => {
            for k in 0..5 {
                let a = k as f32 / 5.0 * TAU + r(k);
                let d = Vec3::new(a.cos(), 0.0, a.sin());
                let side = Vec3::new(-d.z, 0.0, d.x) * 0.25;
                let o = base.transform_point3(Vec3::ZERO);
                let tip = base.transform_point3(d * 1.4 + Vec3::Y * 0.6);
                let mid = d * 0.7 + Vec3::Y * 0.7;
                tri2(
                    &mut p.neon[(k % 2) as usize],
                    o,
                    base.transform_point3(mid + side),
                    tip,
                );
                tri2(&mut p.body, o, tip, base.transform_point3(mid - side));
            }
        }
        Flower => {
            frustum_open(&mut p.body, base, 4, 0.04, 0.03, 0.0, 0.9);
            ball(
                &mut p.neon[1],
                base * Affine3A::from_translation(Vec3::Y * 1.0),
                0.18,
                1.0,
                5,
            );
        }
        IceSpire => {
            frustum(&mut p.body, base, 5, 2.2, 0.0, -1.0, 18.0, r(1));
            for y in [2.0f32, 6.0, 10.0] {
                let rr = 2.2 * (1.0 - (y + 1.0) / 19.0) + 0.05;
                frustum_open(&mut p.neon[0], base, 5, rr, rr * 0.97, y, y + 0.3);
            }
        }
        IceShard => {
            let tilt = Quat::from_rotation_x(0.3 * (r(2) - 0.5));
            crystal(&mut p.neon[0], base * Affine3A::from_quat(tilt), 0.6, 4.0);
            frustum(&mut p.body, base, 5, 0.9, 0.5, -0.4, 0.5, 0.0);
        }
        Basalt => {
            for k in 0..4 {
                let a = k as f32 / 4.0 * TAU;
                let off = Vec3::new(a.cos(), 0.0, a.sin()) * 0.9 * (k > 0) as u8 as f32;
                let hh = 6.0 * (0.5 + r(k * 3) * 0.6);
                let t = base * Affine3A::from_translation(off);
                frustum(&mut p.body, t, 6, 0.7, 0.7, -1.0, hh, 0.0);
                frustum_open(&mut p.neon[0], t, 6, 0.72, 0.72, hh - 0.15, hh + 0.02);
            }
        }
        Vent => {
            frustum(&mut p.body, base, 7, 1.6, 0.6, -0.3, 0.8, 0.0);
            frustum(&mut p.neon[0], base, 7, 0.55, 0.45, 0.8, 0.85, 0.0);
        }
        DeadTree => {
            frustum_open(&mut p.body, base, 5, 0.3, 0.12, 0.0, 6.0);
            for k in 0..3 {
                let a = k as f32 / 3.0 * TAU + r(k);
                let q = Quat::from_rotation_y(a) * Quat::from_rotation_z(-0.9);
                let t = base * Affine3A::from_rotation_translation(q, Vec3::Y * (3.0 + k as f32));
                frustum_open(&mut p.body, t, 4, 0.12, 0.03, 0.0, 2.2);
                ball(
                    &mut p.neon[0],
                    t * Affine3A::from_translation(Vec3::Y * 2.2),
                    0.08,
                    1.0,
                    4,
                );
            }
        }
        Mushroom => {
            frustum_open(&mut p.body, base, 7, 0.7, 0.5, 0.0, 7.0);
            let cap = base * Affine3A::from_translation(Vec3::Y * 7.5);
            frustum(&mut p.body, cap, 9, 4.0, 2.2, -0.6, 1.0, 0.0);
            frustum_open(&mut p.neon[0], cap, 9, 4.05, 4.05, -0.75, -0.55);
            for k in 0..5 {
                let a = k as f32 / 5.0 * TAU + r(k);
                ball(
                    &mut p.neon[1],
                    cap * Affine3A::from_translation(Vec3::new(a.cos() * 2.6, 0.75, a.sin() * 2.6)),
                    0.3,
                    0.6,
                    4,
                );
            }
        }
        Toadstool => {
            frustum_open(&mut p.body, base, 5, 0.08, 0.06, 0.0, 0.5);
            frustum(
                &mut p.neon[(h % 2) as usize],
                base,
                6,
                0.45,
                0.15,
                0.45,
                0.7,
                0.0,
            );
        }
        Reed => {
            for k in 0..3 {
                let a = k as f32 / 3.0 * TAU + r(k);
                let o = Vec3::new(a.cos(), 0.0, a.sin()) * 0.15;
                let top = o * 3.0 + Vec3::Y * (2.0 + r(k + 5));
                tri2(
                    &mut p.body,
                    base.transform_point3(o - Vec3::X * 0.05),
                    base.transform_point3(o + Vec3::X * 0.05),
                    base.transform_point3(top),
                );
                ball(
                    &mut p.neon[0],
                    base * Affine3A::from_translation(top),
                    0.07,
                    2.0,
                    4,
                );
            }
        }
        CircuitTree => {
            frustum_open(&mut p.body, base, 4, 0.4, 0.3, 0.0, 8.0);
            frustum_open(&mut p.neon[0], base, 4, 0.42, 0.42, 1.0, 1.1);
            for k in 0..4 {
                let a = k as f32 * PI / 2.0 + r(1);
                let d = Vec3::new(a.cos(), 0.0, a.sin());
                let y = 3.5 + k as f32 * 1.1;
                let len = 2.2 + r(k + 2) * 1.5;
                // A trace out, then up, with a node at its end.
                let t = base
                    * Affine3A::from_rotation_translation(
                        Quat::from_rotation_arc(Vec3::Y, d),
                        Vec3::Y * y,
                    );
                frustum_open(&mut p.neon[(k % 2) as usize], t, 4, 0.06, 0.06, 0.0, len);
                let end = d * len + Vec3::Y * y;
                frustum_open(
                    &mut p.neon[(k % 2) as usize],
                    base * Affine3A::from_translation(end),
                    4,
                    0.06,
                    0.06,
                    0.0,
                    1.2,
                );
                ball(
                    &mut p.neon[1],
                    base * Affine3A::from_translation(end + Vec3::Y * 1.3),
                    0.2,
                    1.0,
                    4,
                );
            }
        }
        Monolith => {
            let t = base * Affine3A::from_scale(Vec3::new(1.5, 1.0, 0.5));
            frustum(&mut p.body, t, 4, 1.4, 1.4, -1.0, 10.0, PI / 4.0);
            let s = base * Affine3A::from_scale(Vec3::new(1.52, 1.0, 0.52));
            for y in [1.5f32, 5.0, 8.5] {
                frustum_open(
                    &mut p.neon[(y as u32 % 2) as usize],
                    s,
                    4,
                    1.4,
                    1.4,
                    y,
                    y + 0.12,
                );
            }
        }
        Shrub => {
            ball(
                &mut p.body,
                base * Affine3A::from_translation(Vec3::Y * 0.5),
                0.9,
                0.7,
                6,
            );
            for k in 0..3 {
                let a = k as f32 / 3.0 * TAU + r(k);
                ball(
                    &mut p.neon[(k % 2) as usize],
                    base * Affine3A::from_translation(Vec3::new(a.cos() * 0.7, 0.8, a.sin() * 0.7)),
                    0.1,
                    1.0,
                    4,
                );
            }
        }
        Grass => {
            for k in 0..4 {
                let a = k as f32 / 4.0 * TAU + r(k);
                let d = Vec3::new(a.cos(), 0.0, a.sin()) * 0.3;
                let top = d * 2.0 + Vec3::Y * (0.7 + r(k + 3) * 0.5);
                let m = if k == 0 { &mut p.neon[0] } else { &mut p.body };
                tri2(
                    m,
                    base.transform_point3(d - Vec3::X * 0.06),
                    base.transform_point3(d + Vec3::X * 0.06),
                    base.transform_point3(top),
                );
            }
        }
        Tree => {
            frustum_open(&mut p.body, base, 6, 0.35, 0.25, 0.0, 3.0);
            for (k, (y, rr)) in [(2.5f32, 2.6f32), (4.5, 2.0), (6.3, 1.3)]
                .into_iter()
                .enumerate()
            {
                let t = base * Affine3A::from_translation(Vec3::Y * y);
                frustum(&mut p.body, t, 7, rr, 0.2, 0.0, 2.6, k as f32);
                frustum_open(&mut p.neon[0], t, 7, rr + 0.02, rr + 0.02, 0.0, 0.08);
            }
        }
        Lamp => {
            frustum_open(&mut p.body, base, 4, 0.12, 0.08, 0.0, 5.0);
            ball(
                &mut p.neon[1],
                base * Affine3A::from_translation(Vec3::Y * 5.2),
                0.3,
                1.0,
                6,
            );
        }
    }
}

pub fn spawn(
    commands: &mut Commands,
    map: &ContinentMap,
    lk: &Look,
    meshes: &mut Assets<Mesh>,
    std: &mut Assets<StandardMaterial>,
    accents: &mut Assets<AccentMaterial>,
) {
    let body = std.add(StandardMaterial {
        base_color: Color::linear_rgb(lk.rock.x * 1.5, lk.rock.y * 1.5, lk.rock.z * 1.5),
        perceptual_roughness: 0.7,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let neon = [
        accents.add(AccentMaterial {
            neon: NeonUniform::fixed(lk.neon[0], 2.4),
            overlay: false,
        }),
        accents.add(AccentMaterial {
            neon: NeonUniform::fixed(lk.neon[1], 2.4),
            overlay: false,
        }),
    ];
    let n = (2.0 * HALF / REGION).ceil() as usize;
    let mut regions: Vec<Parts> = (0..n * n).map(|_| Parts::default()).collect();
    for it in &map.scenery {
        let i = (((it.pos.x + HALF) / REGION) as usize).min(n - 1);
        let j = (((it.pos.z + HALF) / REGION) as usize).min(n - 1);
        build(&mut regions[j * n + i], it);
    }
    for (k, p) in regions.into_iter().enumerate() {
        let mid = Vec2::new(
            (k % n) as f32 * REGION - HALF + REGION / 2.0,
            (k / n) as f32 * REGION - HALF + REGION / 2.0,
        );
        let parent = commands
            .spawn((
                ContinentPart,
                Region(mid),
                Transform::IDENTITY,
                Visibility::default(),
            ))
            .id();
        if !p.body.is_empty() {
            commands.spawn((
                Mesh3d(meshes.add(p.body.to_mesh())),
                MeshMaterial3d(body.clone()),
                ChildOf(parent),
            ));
        }
        for (i, m) in p.neon.iter().enumerate() {
            if !m.is_empty() {
                commands.spawn((
                    Mesh3d(meshes.add(m.to_mesh())),
                    MeshMaterial3d(neon[i].clone()),
                    ChildOf(parent),
                ));
            }
        }
    }
}

pub struct FloraPlugin;

impl Plugin for FloraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, cull);
    }
}

/// Regions beyond the fog are hidden.
fn cull(
    cams: Query<&GlobalTransform, With<MainCamera>>,
    mut regions: Query<(&Region, &mut Visibility)>,
) {
    let Some(eye) = cams.iter().next().map(|t| t.translation()) else {
        return;
    };
    for (r, mut vis) in &mut regions {
        let d = (r.0 - Vec2::new(eye.x, eye.z)).length() - REGION * 0.71;
        let want = if d < FOG_END {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
}
