//! The bases, towers and warp gates drawn: each base's walls, stairs (its ramps), floors and
//! furniture from its colliders and its architecture's drawn-only details (cornices,
//! buttresses, window frames and glass: plated, src/hull.rs; neon trims in its owner's colour), its props from
//! props.glb (terminals, console, pads, spawn tubes, generator, silo filled as its resources,
//! the big wall turrets turning as the snapshots say), lamps that light up at night, the spawn
//! rooms' and gateways' forcefields (gone while the generator is down), and the warp gates
//! with their swirling cores, pulsing shells, spinning rings and rising sparks. Resource nodes
//! glow where they are. Recoloured when a base changes hands.

use std::collections::HashMap;

use bevy::prelude::*;
use ps_sim::base::{BaseLayout, Finish, Style, TURRET_PIVOT, TerminalKind};
use ps_sim::collide::Collider;
use ps_sim::{BaseIdx, Team};

use crate::assets::{MeshData, Model, Models};
use crate::desktop::MainCamera;
use crate::game::{Arrived, Game};
use crate::hull::{HullMaterial, Hulls};
use crate::materials::{AccentMaterial, GlowMaterial, NeonUniform, Palette, team_color};

/// Everything the bases and gates are drawn with (taken down on leaving the continent).
#[derive(Component)]
pub struct BasePart;

/// A part tinted by base `.0`'s owner: an accent or a glow (`.1`: brightness).
#[derive(Component)]
pub struct BaseTint(pub BaseIdx, pub f32);

/// The turret head of turret `.0` (its index on the continent).
#[derive(Component)]
pub struct TurretHead(pub usize);

/// A forcefield of base `.0`: shown while it has power.
#[derive(Component)]
struct Field(BaseIdx);

/// Base `.0`'s generator core (pulsing; dark when down) and silo fill.
#[derive(Component)]
struct GeneratorCore(BaseIdx);
#[derive(Component)]
struct SiloFill(BaseIdx);

/// A fill gauge on base `.0`'s silo's side: its bar, scaled up from its foot with the
/// resources; amber, red when running low (the two materials).
#[derive(Component)]
struct SiloGauge(BaseIdx, Handle<AccentMaterial>, Handle<AccentMaterial>);

/// The silo's gauges: their height (m), and below what share they show red.
const GAUGE_H: f32 = 9.0;
const GAUGE_LOW: f32 = 0.2;

/// A lamp: brighter at night (its full intensity).
#[derive(Component)]
pub struct Lamp(pub f32);

/// A light shown only within this distance of the camera (lights are costly).
#[derive(Component)]
pub struct Near(pub f32);

#[derive(Component)]
struct GateCore;
#[derive(Component)]
struct GateRings(f32);
#[derive(Component)]
struct GateShell(Handle<GlowMaterial>);

/// A spark rising in a warp gate: velocity, seconds left.
#[derive(Component)]
pub struct Spark {
    vel: Vec3,
    life: f32,
    /// How fast its heading turns about +Y (rad/s).
    spin: f32,
    size: f32,
}

/// A warp gate's middle (for its sparks).
#[derive(Component)]
struct GateAt(Vec3);

#[derive(Component)]
struct Node(u32);

/// The owners the tints were last set for.
#[derive(Resource, Default)]
struct Shown(HashMap<BaseIdx, Option<Team>>);

#[derive(Resource)]
pub struct BaseAssets {
    spark: Handle<Mesh>,
    spark_mat: Handle<GlowMaterial>,
    harvest_mat: Handle<GlowMaterial>,
    node: Model,
    node_mat: Handle<AccentMaterial>,
}

pub struct BasesPlugin;

impl Plugin for BasesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Shown>()
            .add_systems(PostStartup, make_assets)
            .add_systems(Update, clear.run_if(not(resource_exists::<Game>)))
            .add_systems(
                Update,
                (
                    build.after(crate::terrain::TerrainBuild),
                    retint,
                    turrets,
                    power,
                    gates,
                    sparks,
                    nodes,
                    cull_lights,
                )
                    .run_if(resource_exists::<Game>),
            );
    }
}

fn make_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut glows: ResMut<Assets<GlowMaterial>>,
    mut accents: ResMut<Assets<AccentMaterial>>,
    models: Res<Models>,
) {
    let node = if models.has("resource_node") {
        models.get("resource_node")
    } else {
        Model {
            parts: vec![(
                "resource".into(),
                meshes.add(
                    Cone::new(1.2, 3.0)
                        .mesh()
                        .build()
                        .translated_by(Vec3::Y * 1.5),
                ),
            )],
            nodes: HashMap::new(),
        }
    };
    commands.insert_resource(BaseAssets {
        spark: meshes.add(Rectangle::new(0.5, 0.5)),
        spark_mat: glows.add(GlowMaterial {
            neon: NeonUniform::fixed(Vec3::new(0.5, 0.9, 1.0), 3.0),
            overlay: false,
        }),
        harvest_mat: glows.add(GlowMaterial {
            neon: NeonUniform::fixed(Vec3::new(1.0, 0.6, 0.1), 4.0),
            overlay: false,
        }),
        node,
        node_mat: accents.add(AccentMaterial {
            neon: NeonUniform::fixed(Vec3::new(1.0, 0.55, 0.05), 4.0),
            overlay: false,
        }),
    });
}

fn push_box(m: &mut MeshData, min: Vec3, max: Vec3) {
    let c = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ];
    let faces: [([usize; 4], Vec3); 6] = [
        ([0, 3, 2, 1], Vec3::NEG_Z),
        ([4, 5, 6, 7], Vec3::Z),
        ([0, 4, 7, 3], Vec3::NEG_X),
        ([1, 2, 6, 5], Vec3::X),
        ([3, 7, 6, 2], Vec3::Y),
        ([0, 1, 5, 4], Vec3::NEG_Y),
    ];
    for (f, n) in faces {
        let base = m.pos.len() as u32;
        for k in f {
            m.pos.push(c[k].to_array());
            m.nrm.push(n.to_array());
            m.uv.push([0.0, 0.0]);
        }
        m.idx
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

fn push_poly(m: &mut MeshData, pts: &[Vec3], n: Vec3) {
    let base = m.pos.len() as u32;
    for p in pts {
        m.pos.push(p.to_array());
        m.nrm.push(n.to_array());
        m.uv.push([0.0, 0.0]);
    }
    for i in 1..pts.len() as u32 - 1 {
        m.idx.extend([base, base + i, base + i + 1]);
    }
}

/// A ramp drawn as a flight of stairs (its collider stays the smooth slope, which runs through
/// the middle of every tread): solid down to its foot, a lit strip along each nosing (`trim`).
fn push_stairs(m: &mut MeshData, trim: &mut MeshData, min: Vec3, max: Vec3, dir: u8) {
    let h = max.y - min.y;
    let along_z = dir.is_multiple_of(2);
    let (lo, hi) = if along_z {
        (min.z, max.z)
    } else {
        (min.x, max.x)
    };
    let (w0, w1) = if along_z {
        (min.x, max.x)
    } else {
        (min.z, max.z)
    };
    let up = dir < 2;
    let n = (h.abs() / 0.3).round().max(1.0) as usize;
    // A point from (t up the flight, across, height).
    let at = |t: f32, w: f32, y: f32| -> Vec3 {
        let a = if up {
            lo + (hi - lo) * t
        } else {
            hi - (hi - lo) * t
        };
        if along_z {
            Vec3::new(w, y, a)
        } else {
            Vec3::new(a, y, w)
        }
    };
    let rise = h / n as f32;
    for k in 0..n {
        let t0 = (k as f32 + 0.5) / n as f32;
        let t1 = ((k as f32 + 1.5) / n as f32).min(1.0);
        let y1 = min.y + rise * (k + 1) as f32;
        let a = at(t0, w0, y1);
        let b = at(t1, w1, min.y);
        push_box_open(m, a.min(b), a.max(b));
        // The nosing's strip.
        let ws = (w1 - w0) * 0.08;
        let p = at(t0, w0 + ws, y1 - 0.04);
        let q = at(t0 + 0.06 / (hi - lo), w1 - ws, y1 + 0.006);
        push_box(trim, p.min(q), p.max(q));
    }
}

/// A box without its bottom face.
fn push_box_open(m: &mut MeshData, min: Vec3, max: Vec3) {
    push_box(m, min, max);
    // (push_box's last face is the bottom.)
    let n = m.pos.len() - 4;
    m.pos.truncate(n);
    m.nrm.truncate(n);
    m.uv.truncate(n);
    m.idx.truncate(m.idx.len() - 6);
}

/// A drawn-only detail: its section's four corners swept to its end's (`Detail::pts`), each
/// face wound outward from its middle.
fn push_detail(m: &mut MeshData, pts: &[Vec3; 8]) {
    let mid = pts.iter().copied().sum::<Vec3>() / 8.0;
    let mut faces: Vec<[Vec3; 4]> = (0..4)
        .map(|k| {
            let j = (k + 1) % 4;
            [pts[k], pts[j], pts[j + 4], pts[k + 4]]
        })
        .collect();
    faces.push([pts[0], pts[1], pts[2], pts[3]]);
    faces.push([pts[4], pts[5], pts[6], pts[7]]);
    for mut f in faces {
        // Newell's normal (the quads may be bent a little).
        let mut n = Vec3::ZERO;
        for i in 0..4 {
            let (a, b) = (f[i], f[(i + 1) % 4]);
            n += Vec3::new(
                (a.y - b.y) * (a.z + b.z),
                (a.z - b.z) * (a.x + b.x),
                (a.x - b.x) * (a.y + b.y),
            );
        }
        if n.length_squared() < 1e-8 {
            continue;
        }
        let c = (f[0] + f[1] + f[2] + f[3]) / 4.0;
        if n.dot(c - mid) < 0.0 {
            f.reverse();
            n = -n;
        }
        push_poly(m, &f, n.normalize());
    }
}

fn push_prism(m: &mut MeshData, c: &Collider) {
    let Collider::Prism { min, max, n, pts } = *c else {
        return;
    };
    let n = n as usize;
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let q = [
            Vec3::new(a.x, min.y, a.y),
            Vec3::new(a.x, max.y, a.y),
            Vec3::new(b.x, max.y, b.y),
            Vec3::new(b.x, min.y, b.y),
        ];
        let out = Vec3::new(b.y - a.y, 0.0, a.x - b.x).normalize_or(Vec3::X);
        push_poly(m, &q, -out);
        push_poly(m, &[q[3], q[2], q[1], q[0]], out);
    }
    let top: Vec<Vec3> = pts[..n]
        .iter()
        .map(|p| Vec3::new(p.x, max.y, p.y))
        .collect();
    let mut rev = top.clone();
    rev.reverse();
    push_poly(m, &top, Vec3::Y);
    push_poly(m, &rev, Vec3::Y);
}

/// The canyon caves' alien dress on a wall or a building's side (`min`..`max`): tapered ribs up
/// both long faces every `every` metres with a vein of team light up each, and (the compound's
/// walls) a spire at each end of the run.
fn alien_ribs(
    body: &mut MeshData,
    trim: &mut MeshData,
    min: Vec3,
    max: Vec3,
    every: f32,
    spires: bool,
) {
    let along_x = max.x - min.x >= max.z - min.z;
    // (Local: u along the run, v out of the faces.)
    let (u0, u1) = if along_x {
        (min.x, max.x)
    } else {
        (min.z, max.z)
    };
    let (v0, v1) = if along_x {
        (min.z, max.z)
    } else {
        (min.x, max.x)
    };
    let at = |u: f32, y: f32, v: f32| {
        if along_x {
            Vec3::new(u, y, v)
        } else {
            Vec3::new(v, y, u)
        }
    };
    let (y0, y1) = (min.y.max(max.y - 14.0), max.y - 0.4);
    if y1 - y0 < 2.0 {
        return;
    }
    let n = ((u1 - u0) / every).floor() as i32;
    let start = u0 + (u1 - u0 - (n - 1).max(0) as f32 * every) / 2.0;
    for k in 0..n {
        let u = start + k as f32 * every;
        for (face, out) in [(v0, -1.0f32), (v1, 1.0f32)] {
            // A section round the rib at the foot and at the top, out from the face.
            let sec = |y: f32, w: f32, o0: f32, o1: f32| -> [Vec3; 4] {
                [
                    at(u - w, y, face + out * o0),
                    at(u + w, y, face + out * o0),
                    at(u + w, y, face + out * o1),
                    at(u - w, y, face + out * o1),
                ]
            };
            let (a, b) = (sec(y0, 0.32, -0.02, 0.3), sec(y1, 0.14, -0.02, 0.1));
            push_detail(body, &[a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]]);
            let (a, b) = (
                sec(y0 + 0.8, 0.04, 0.27, 0.32),
                sec(y1 - 0.3, 0.03, 0.09, 0.13),
            );
            push_detail(trim, &[a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]]);
        }
    }
    if spires && u1 - u0 > 6.0 {
        let vm = (v0 + v1) / 2.0;
        for u in [u0 + 1.0, u1 - 1.0] {
            let r = 0.7;
            let foot = [
                at(u - r, max.y, vm - r),
                at(u + r, max.y, vm - r),
                at(u + r, max.y, vm + r),
                at(u - r, max.y, vm + r),
            ];
            let tip = at(u, max.y + 2.6, vm);
            let t = 0.03;
            let top = [
                tip + Vec3::new(-t, 0.0, -t),
                tip + Vec3::new(t, 0.0, -t),
                tip + Vec3::new(t, 0.0, t),
                tip + Vec3::new(-t, 0.0, t),
            ];
            push_detail(
                body,
                &[
                    foot[0], foot[1], foot[2], foot[3], top[0], top[1], top[2], top[3],
                ],
            );
            let glow: [Vec3; 4] = std::array::from_fn(|i| tip + (foot[i] - tip) * 0.27);
            let tip2: [Vec3; 4] = std::array::from_fn(|i| top[i] + Vec3::Y * 0.02);
            push_detail(
                trim,
                &[
                    glow[0], glow[1], glow[2], glow[3], tip2[0], tip2[1], tip2[2], tip2[3],
                ],
            );
        }
    }
}

/// A railing over the thin box `min`..`max` (its collider): posts, a top rail with a lit strip
/// under it, a mid rail and a kick plate; in the canyon caves bone-like spikes under one rail.
fn push_rail(m: &mut MeshData, trim: &mut MeshData, min: Vec3, max: Vec3, alien: bool) {
    let along_x = max.x - min.x >= max.z - min.z;
    let (u0, u1) = if along_x {
        (min.x, max.x)
    } else {
        (min.z, max.z)
    };
    let v = if along_x {
        (min.z + max.z) / 2.0
    } else {
        (min.x + max.x) / 2.0
    };
    let at = |u: f32, y: f32, dv: f32| {
        if along_x {
            Vec3::new(u, y, v + dv)
        } else {
            Vec3::new(v + dv, y, u)
        }
    };
    let bar = |m: &mut MeshData, u0: f32, u1: f32, y0: f32, y1: f32, w: f32| {
        let (a, b) = (at(u0, y0, -w), at(u1, y1, w));
        push_box(m, a.min(b), a.max(b));
    };
    let (y0, y1) = (min.y, max.y);
    let len = u1 - u0;
    let n = (len / 2.0).ceil().max(1.0) as usize;
    for k in 0..=n {
        let u = (u0 + len * k as f32 / n as f32).clamp(u0 + 0.04, u1 - 0.04);
        if alien {
            // A spike leaning up to the rail.
            let foot = [
                at(u - 0.1, y0, -0.07),
                at(u + 0.1, y0, -0.07),
                at(u + 0.1, y0, 0.07),
                at(u - 0.1, y0, 0.07),
            ];
            let tip = [
                at(u - 0.03, y1 + 0.25, -0.02),
                at(u + 0.03, y1 + 0.25, -0.02),
                at(u + 0.03, y1 + 0.25, 0.02),
                at(u - 0.03, y1 + 0.25, 0.02),
            ];
            push_detail(
                m,
                &[
                    foot[0], foot[1], foot[2], foot[3], tip[0], tip[1], tip[2], tip[3],
                ],
            );
        } else {
            bar(m, u - 0.035, u + 0.035, y0, y1 - 0.05, 0.035);
        }
    }
    bar(m, u0, u1, y1 - 0.06, y1, 0.055);
    bar(trim, u0 + 0.05, u1 - 0.05, y1 - 0.1, y1 - 0.065, 0.045);
    if !alien {
        let mid = (y0 + y1) / 2.0;
        bar(m, u0, u1, mid - 0.025, mid + 0.025, 0.02);
        bar(m, u0, u1, y0, y0 + 0.12, 0.012);
    }
}

/// A base's structure: bodies by surface (`SURFACES`: walls, floors, furniture, buildings,
/// roofs, bare metal, stairwells), the windows' glass, and trims along the tops of walls and
/// buildings, round the furniture, on the stairs' nosings, under the railings and wherever the
/// architecture's details put them.
fn structure(layout: &BaseLayout, alien: bool) -> ([MeshData; SURFACES], MeshData, MeshData) {
    let mut body: [MeshData; SURFACES] = Default::default();
    let mut glass = MeshData::default();
    let mut trim = MeshData::default();
    for (c, style) in layout.colliders.iter().zip(&layout.styles) {
        let k = match style {
            // (The turrets' pillars are drawn with the turrets.)
            Style::Mount => continue,
            Style::Floor | Style::Ramp | Style::Interior => 1,
            Style::Decor | Style::Cover => 2,
            Style::Building => 3,
            Style::Roof | Style::Canopy => 4,
            Style::Rail => 5,
            Style::Stairwell => 6,
            _ => 0,
        };
        match *c {
            Collider::Box { min, max } if matches!(style, Style::Rail) => {
                push_rail(&mut body[k], &mut trim, min, max, alien);
            }
            Collider::Box { min, max } => {
                push_box(&mut body[k], min, max);
                let e = 0.03;
                let band = match style {
                    Style::Wall | Style::Building | Style::Bastion => Some(0.2),
                    Style::Stairwell | Style::Gatehouse => Some(0.14),
                    Style::Parapet => Some(0.1),
                    Style::Roof | Style::Canopy => Some(0.08),
                    Style::Interior => Some(0.06),
                    Style::Decor | Style::Cover => Some(0.05),
                    _ => None,
                };
                if let Some(b) = band {
                    push_box(
                        &mut trim,
                        Vec3::new(min.x - e, max.y - b, min.z - e),
                        Vec3::new(max.x + e, max.y - b * 0.4, max.z + e),
                    );
                    let ribbed = matches!(
                        style,
                        Style::Wall | Style::Building | Style::Bastion | Style::Stairwell
                    );
                    if alien && ribbed {
                        // Ribs and veins up the wall (instead of bands).
                        let wall = matches!(style, Style::Wall | Style::Bastion);
                        let every = if wall { 3.5 } else { 4.0 };
                        alien_ribs(&mut body[k], &mut trim, min, max, every, wall);
                    } else if matches!(style, Style::Wall | Style::Bastion) {
                        // Bands up the wall.
                        for h in [3.0, 7.0] {
                            if min.y + h < max.y - 1.0 {
                                push_box(
                                    &mut trim,
                                    Vec3::new(min.x - e, min.y + h, min.z - e),
                                    Vec3::new(max.x + e, min.y + h + 0.14, max.z + e),
                                );
                            }
                        }
                    }
                }
                if matches!(style, Style::Floor) && max.y - min.y < 0.6 {
                    // The floors' edges glow: a thin strip round the top of each side.
                    let (y0, y1, w) = (max.y - 0.06, max.y + 0.005, 0.06);
                    for (a, b) in [
                        (
                            Vec3::new(min.x - e, y0, min.z - e),
                            Vec3::new(max.x + e, y1, min.z + w),
                        ),
                        (
                            Vec3::new(min.x - e, y0, max.z - w),
                            Vec3::new(max.x + e, y1, max.z + e),
                        ),
                        (
                            Vec3::new(min.x - e, y0, min.z - e),
                            Vec3::new(min.x + w, y1, max.z + e),
                        ),
                        (
                            Vec3::new(max.x - w, y0, min.z - e),
                            Vec3::new(max.x + e, y1, max.z + e),
                        ),
                    ] {
                        push_box(&mut trim, a, b);
                    }
                }
            }
            Collider::Ramp { min, max, dir } => push_stairs(&mut body[k], &mut trim, min, max, dir),
            Collider::Prism { .. } => push_prism(&mut body[k], c),
        }
    }
    for d in &layout.details {
        match d.finish {
            Finish::Body => push_detail(&mut body[0], &d.pts),
            Finish::Glass => push_detail(&mut glass, &d.pts),
            Finish::Trim => push_detail(&mut trim, &d.pts),
        }
    }
    (body, glass, trim)
}

/// How many surfaces a base's bodies are sorted into (`Hulls::surfaces`' order).
const SURFACES: usize = 7;

/// A model's parts spawned under `parent` (bodies plated); team-tinted parts get `tint`.
pub fn spawn_parts(
    commands: &mut Commands,
    parent: Entity,
    model: &Model,
    palette: &Palette,
    team: Option<Team>,
    tint: Option<BaseIdx>,
    hull: Option<&Handle<HullMaterial>>,
) {
    spawn_plated(
        commands,
        parent,
        model,
        palette,
        team,
        tint,
        hull.map(|h| (h, h)),
    );
}

/// As `spawn_parts`, with plating of its own for the armour plates: `hull` is (body, armor).
pub fn spawn_plated(
    commands: &mut Commands,
    parent: Entity,
    model: &Model,
    palette: &Palette,
    team: Option<Team>,
    tint: Option<BaseIdx>,
    hull: Option<(&Handle<HullMaterial>, &Handle<HullMaterial>)>,
) {
    for (mat, mesh) in &model.parts {
        let mut e = commands.spawn((Mesh3d(mesh.clone()), Transform::IDENTITY, ChildOf(parent)));
        match (mat.as_str(), hull) {
            ("armor", Some((_, armor))) => {
                e.insert(MeshMaterial3d(armor.clone()));
                continue;
            }
            (name, _) => {
                if let Some(m) = palette.surface(name, team) {
                    e.insert(MeshMaterial3d(m));
                    continue;
                }
            }
        }
        match mat.as_str() {
            "body" | "sclera" | "fur_light" => match hull {
                Some((h, _)) => {
                    e.insert(MeshMaterial3d(h.clone()));
                }
                None => {
                    e.insert(MeshMaterial3d(palette.body.clone()));
                }
            },
            "glow" => {
                e.insert(MeshMaterial3d::<GlowMaterial>(palette.glow_for(team)));
                if let Some(b) = tint {
                    e.insert(BaseTint(b, 2.0));
                }
            }
            "halo_white" => {
                e.insert(MeshMaterial3d::<GlowMaterial>(palette.glow_for(None)));
            }
            "glow_white" | "muzzle" | "lamp" => {
                e.insert(MeshMaterial3d::<AccentMaterial>(palette.accent_for(None)));
            }
            "flow" | "tread" => {
                e.insert(MeshMaterial3d::<AccentMaterial>(palette.flow_for(team)));
            }
            _ => {
                e.insert(MeshMaterial3d::<AccentMaterial>(palette.accent_for(team)));
                if let Some(b) = tint {
                    e.insert(BaseTint(b, 3.0));
                }
            }
        }
    }
}

/// A stand-in prop: a dark box with a coloured band.
fn stand_in(meshes: &mut Assets<Mesh>, size: Vec3) -> Model {
    Model {
        parts: vec![
            (
                "body".into(),
                meshes.add(
                    Cuboid::from_size(size)
                        .mesh()
                        .build()
                        .translated_by(Vec3::Y * size.y / 2.0),
                ),
            ),
            (
                "accent".into(),
                meshes.add(
                    Cuboid::new(size.x + 0.04, 0.08, size.z + 0.04)
                        .mesh()
                        .build()
                        .translated_by(Vec3::Y * size.y * 0.85),
                ),
            ),
        ],
        nodes: HashMap::new(),
    }
}

/// A forcefield sheet over a box collider (its long side).
fn field_quad(c: &Collider) -> (Rectangle, Transform) {
    let (min, max) = c.bounds();
    let size = max - min;
    let mid = (min + max) / 2.0;
    let along_x = size.x > size.z;
    let quad = Rectangle::new(if along_x { size.x } else { size.z }, size.y);
    let rot = if along_x {
        Quat::IDENTITY
    } else {
        Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
    };
    (quad, Transform::from_translation(mid).with_rotation(rot))
}

#[allow(clippy::too_many_arguments)]
fn build(
    mut commands: Commands,
    game: Res<Game>,
    mut arrived: MessageReader<Arrived>,
    models: Res<Models>,
    palette: Res<Palette>,
    hulls: Res<Hulls>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut accents: ResMut<Assets<AccentMaterial>>,
    mut glows: ResMut<Assets<GlowMaterial>>,
    mut shown: ResMut<Shown>,
    old: Query<Entity, With<BasePart>>,
    bars: Option<Res<crate::worldbar::BarAssets>>,
) {
    if arrived.read().last().is_none() {
        return;
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    let Some(map) = game.map.clone() else { return };
    shown.0.clear();
    let c = map.def.id;
    // (In the canyon caves: the alien equipment, alien.glb's, where there's one.)
    let alien = map.def.theme.caves();
    let named = |name: &str| -> String {
        let a = format!("alien_{name}");
        if alien && models.has(&a) {
            a
        } else {
            name.to_string()
        }
    };
    let get = |name: &str, size: Vec3, meshes: &mut Assets<Mesh>| -> Model {
        let name = named(name);
        if models.has(&name) {
            models.get(&name)
        } else {
            stand_in(meshes, size)
        }
    };
    let turret_base = get("wall_turret_base", Vec3::new(2.4, 1.8, 2.4), &mut meshes);
    let turret_head = get("wall_turret_head", Vec3::new(2.4, 1.0, 3.0), &mut meshes);
    // The pillars raising the turrets over the walls, with a lit band under the mount.
    let rise = ps_sim::base::TURRET_RISE;
    let pillar = meshes.add(
        Cylinder::new(0.8, rise)
            .mesh()
            .resolution(8)
            .build()
            .translated_by(Vec3::Y * (rise / 2.0)),
    );
    let band = meshes.add(
        Cylinder::new(0.86, 0.14)
            .mesh()
            .resolution(8)
            .build()
            .translated_by(Vec3::Y * (rise - 0.35)),
    );
    let mut turret_index = 0usize;
    for (bi, layout) in map.bases.iter().enumerate() {
        let b = bi as BaseIdx;
        let owner = game.state.base(c, b).owner;
        let root = commands
            .spawn((BasePart, Transform::IDENTITY, Visibility::default()))
            .id();
        let (body, glass, trim) = structure(layout, alien);
        for (k, m) in body.iter().enumerate() {
            if m.is_empty() {
                continue;
            }
            let mat = hulls.surfaces()[k].clone();
            commands.spawn((
                Mesh3d(meshes.add(m.to_mesh())),
                MeshMaterial3d(mat),
                ChildOf(root),
            ));
        }
        if !glass.is_empty() {
            commands.spawn((
                Mesh3d(meshes.add(glass.to_mesh())),
                MeshMaterial3d(hulls.glass.clone()),
                ChildOf(root),
            ));
        }
        commands.spawn((
            Mesh3d(meshes.add(trim.to_mesh())),
            MeshMaterial3d(palette.accent_for(owner)),
            BaseTint(b, 3.0),
            ChildOf(root),
        ));
        let prop = |commands: &mut Commands, model: &Model, at: Vec3, yaw: f32| {
            let e = commands
                .spawn((
                    Transform::from_translation(at).with_rotation(Quat::from_rotation_y(yaw)),
                    Visibility::default(),
                    ChildOf(root),
                ))
                .id();
            spawn_parts(
                commands,
                e,
                model,
                &palette,
                owner,
                Some(b),
                Some(&hulls.decor),
            );
            e
        };
        for t in &layout.terminals {
            let (name, size) = match t.kind {
                TerminalKind::Equipment => ("terminal_equipment", Vec3::new(1.0, 1.9, 0.5)),
                TerminalKind::Medical => ("terminal_medical", Vec3::new(1.0, 1.9, 0.5)),
                TerminalKind::Vehicle => ("terminal_vehicle", Vec3::new(1.0, 1.9, 0.5)),
            };
            let m = get(name, size, &mut meshes);
            prop(&mut commands, &m, t.pos, t.yaw);
        }
        if let Some((at, yaw)) = layout.console {
            let m = get("console_core", Vec3::new(1.1, 1.1, 1.1), &mut meshes);
            prop(&mut commands, &m, at, yaw);
        }
        // Capture points: a pylon each, its ring round the zone, its letter and bar over it;
        // a bar over the console (or the base's middle) while it's being taken.
        let pylon = get(
            "capture_pylon",
            Vec3::new(
                2.0 * ps_sim::base::PYLON_R,
                ps_sim::base::PYLON_H,
                2.0 * ps_sim::base::PYLON_R,
            ),
            &mut meshes,
        );
        for (i, pt) in layout.points.iter().enumerate() {
            prop(&mut commands, &pylon, pt.pylon, 0.0);
            if let Some(bars) = &bars {
                use crate::worldbar::{BarOf, spawn_bar, spawn_ring};
                let top = pt.pylon + Vec3::Y * (ps_sim::base::PYLON_H + 1.0);
                spawn_bar(
                    &mut commands,
                    bars,
                    root,
                    BarOf::Point(b, i as u8),
                    top,
                    1.8,
                );
                spawn_ring(&mut commands, bars, root, b, i as u8, pt.pos, pt.radius);
            }
        }
        if let Some(bars) = &bars {
            use crate::worldbar::{BarOf, spawn_bar};
            if let Some((at, _)) = layout.console {
                spawn_bar(
                    &mut commands,
                    bars,
                    root,
                    BarOf::Console(b),
                    at + Vec3::Y * 2.9,
                    1.6,
                );
            } else if !layout.points.is_empty() {
                let at = layout.center + Vec3::Y * (layout.building.1.y - layout.center.y + 4.0);
                spawn_bar(&mut commands, bars, root, BarOf::Console(b), at, 4.0);
            }
            if let Some(g) = layout.generator {
                spawn_bar(
                    &mut commands,
                    bars,
                    root,
                    BarOf::Generator(b),
                    g + Vec3::Y * 4.3,
                    1.6,
                );
            }
            for (i, &at) in layout.turrets.iter().enumerate() {
                let at = at + Vec3::Y * (TURRET_PIVOT + 1.8);
                spawn_bar(
                    &mut commands,
                    bars,
                    root,
                    BarOf::Turret(b, i as u8),
                    at,
                    1.4,
                );
            }
        }
        for &(at, yaw) in &layout.pads {
            let m = get("vehicle_pad", Vec3::new(14.0, 0.05, 14.0), &mut meshes);
            prop(&mut commands, &m, at + Vec3::Y * 0.02, yaw);
        }
        for &at in &layout.rearm {
            let m = get("rearm_pad", Vec3::new(16.0, 0.05, 16.0), &mut meshes);
            prop(&mut commands, &m, at + Vec3::Y * 0.02, 0.0);
        }
        if models.has(&named("spawn_tube")) {
            let m = models.get(&named("spawn_tube"));
            for &(at, yaw) in &layout.spawns {
                prop(&mut commands, &m, at, yaw);
            }
        }
        if let Some(g) = layout.generator {
            let m = get("generator", Vec3::new(4.0, 3.5, 4.0), &mut meshes);
            let yaw = layout.quarter as f32 * std::f32::consts::FRAC_PI_2;
            prop(&mut commands, &m, g, yaw);
            if models.has(&named("generator_core")) {
                let core = models.get(&named("generator_core"));
                let e = commands
                    .spawn((
                        GeneratorCore(b),
                        Transform::from_translation(g),
                        Visibility::default(),
                        ChildOf(root),
                    ))
                    .id();
                spawn_parts(&mut commands, e, &core, &palette, owner, Some(b), None);
            }
            commands.spawn((
                PointLight {
                    color: Color::linear_rgb(0.6, 0.8, 1.0),
                    intensity: 60_000.0,
                    range: 10.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Lamp(60_000.0),
                Near(120.0),
                Transform::from_translation(g + Vec3::Y * 2.0),
                ChildOf(root),
            ));
        }
        if let Some(sp) = layout.silo {
            let m = get("resource_silo", Vec3::new(8.0, 12.0, 8.0), &mut meshes);
            prop(&mut commands, &m, sp, 0.0);
            let fill = if models.has(&named("resource_silo_fill")) {
                models.get(&named("resource_silo_fill"))
            } else {
                Model {
                    parts: vec![(
                        "accent".into(),
                        meshes.add(
                            Cylinder::new(3.6, 1.0)
                                .mesh()
                                .build()
                                .translated_by(Vec3::Y * 0.5),
                        ),
                    )],
                    nodes: HashMap::new(),
                }
            };
            let e = commands
                .spawn((
                    SiloFill(b),
                    Transform::from_translation(sp + Vec3::Y * 0.3)
                        .with_scale(Vec3::new(1.0, 6.0, 1.0)),
                    Visibility::default(),
                    ChildOf(root),
                ))
                .id();
            for (_, mesh) in &fill.parts {
                commands.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(accents.add(AccentMaterial {
                        neon: NeonUniform::flow(Vec3::new(1.0, 0.55, 0.05), 2.5, 1.5),
                        overlay: false,
                    })),
                    ChildOf(e),
                ));
            }
            // Gauges on four sides, so one faces whoever looks: a dark slot, and in it a bar
            // as full as the silo.
            let amber = accents.add(AccentMaterial {
                neon: NeonUniform::fixed(Vec3::new(1.0, 0.55, 0.05), 3.0),
                overlay: false,
            });
            let red = accents.add(AccentMaterial {
                neon: NeonUniform::fixed(Vec3::new(1.0, 0.08, 0.04), 3.5),
                overlay: false,
            });
            let slot = meshes.add(Cuboid::new(0.9, GAUGE_H + 0.4, 0.16));
            let bar = meshes.add(
                Cuboid::new(0.6, 1.0, 0.1)
                    .mesh()
                    .build()
                    .translated_by(Vec3::Y * 0.5),
            );
            let r = ps_sim::base::SILO_R + 0.3;
            for k in 0..4 {
                let yaw = k as f32 * std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_4;
                let rot = Quat::from_rotation_y(yaw);
                let out = rot * Vec3::Z;
                let foot = sp + Vec3::Y * 1.2;
                commands.spawn((
                    Mesh3d(slot.clone()),
                    MeshMaterial3d(hulls.wall.clone()),
                    Transform::from_translation(foot + out * r + Vec3::Y * (GAUGE_H / 2.0))
                        .with_rotation(rot),
                    ChildOf(root),
                ));
                commands.spawn((
                    SiloGauge(b, amber.clone(), red.clone()),
                    Mesh3d(bar.clone()),
                    MeshMaterial3d(amber.clone()),
                    Transform::from_translation(foot + out * (r + 0.06))
                        .with_rotation(rot)
                        .with_scale(Vec3::new(1.0, 0.05, 1.0)),
                    ChildOf(root),
                ));
            }
        }
        for &at in &layout.turrets {
            let foot = at - Vec3::Y * rise;
            commands.spawn((
                Mesh3d(pillar.clone()),
                MeshMaterial3d(hulls.decor.clone()),
                Transform::from_translation(foot),
                ChildOf(root),
            ));
            commands.spawn((
                Mesh3d(band.clone()),
                MeshMaterial3d(palette.accent_for(owner)),
                BaseTint(b, 3.0),
                Transform::from_translation(foot),
                ChildOf(root),
            ));
            prop(&mut commands, &turret_base, at, 0.0);
            let head = commands
                .spawn((
                    TurretHead(turret_index),
                    Transform::from_translation(at + Vec3::Y * TURRET_PIVOT),
                    Visibility::default(),
                    ChildOf(root),
                ))
                .id();
            spawn_parts(
                &mut commands,
                head,
                &turret_head,
                &palette,
                owner,
                Some(b),
                Some(&hulls.decor),
            );
            turret_index += 1;
        }
        // Forcefields: the spawn rooms' and the gateways'.
        for c in layout
            .shields
            .iter()
            .map(|s| s.collider)
            .chain(layout.gate_fields.iter().copied())
        {
            let (quad, t) = field_quad(&c);
            let mat = glows.add(GlowMaterial {
                neon: NeonUniform::fixed(team_color(owner), 1.4).shield(),
                overlay: false,
            });
            commands.spawn((
                Mesh3d(meshes.add(quad)),
                MeshMaterial3d(mat),
                BaseTint(b, 1.4),
                Field(b),
                t,
                ChildOf(root),
            ));
        }
        for l in layout.lamps.iter() {
            let e = match l.aim {
                Some(aim) => commands
                    .spawn((
                        SpotLight {
                            color: Color::linear_rgb(0.9, 0.95, 1.0),
                            intensity: 4_000_000.0,
                            range: 140.0,
                            outer_angle: 0.75,
                            inner_angle: 0.45,
                            shadow_maps_enabled: false,
                            ..default()
                        },
                        Lamp(4_000_000.0),
                        Near(260.0),
                        Transform::from_translation(l.pos).looking_at(aim, Vec3::Y),
                    ))
                    .id(),
                None => commands
                    .spawn((
                        PointLight {
                            color: Color::linear_rgb(0.85, 0.9, 1.0),
                            intensity: 400_000.0,
                            range: 20.0,
                            shadow_maps_enabled: false,
                            ..default()
                        },
                        Lamp(400_000.0),
                        Near(90.0),
                        Transform::from_translation(l.pos),
                    ))
                    .id(),
            };
            commands.entity(e).insert(ChildOf(root));
        }
        shown.0.insert(b, owner);
    }
    // Warp gates.
    let gate = get("warpgate", Vec3::new(4.0, 55.0, 4.0), &mut meshes);
    let core_mat = glows.add(GlowMaterial {
        neon: NeonUniform::fixed(Vec3::new(0.4, 0.8, 1.0), 1.4).volume(),
        overlay: false,
    });
    for &at in &map.gates {
        let root = commands
            .spawn((
                BasePart,
                GateAt(at),
                Transform::from_translation(at),
                Visibility::default(),
            ))
            .id();
        if models.has("warpgate") {
            spawn_parts(
                &mut commands,
                root,
                &gate,
                &palette,
                None,
                None,
                Some(&hulls.wall),
            );
        } else {
            let mut m = MeshData::default();
            for c in ps_sim::base::gate_colliders(Vec3::ZERO) {
                push_prism(&mut m, &c);
            }
            commands.spawn((
                Mesh3d(meshes.add(m.to_mesh())),
                MeshMaterial3d(hulls.wall.clone()),
                ChildOf(root),
            ));
        }
        let core = models
            .named
            .get("warpgate_core")
            .and_then(|m| m.parts.first().map(|p| p.1.clone()))
            .unwrap_or_else(|| {
                meshes.add(
                    Cylinder::new(ps_sim::base::GATE_CORE, 50.0)
                        .mesh()
                        .resolution(24)
                        .build()
                        .translated_by(Vec3::Y * 25.0),
                )
            });
        commands.spawn((
            Mesh3d(core),
            MeshMaterial3d(core_mat.clone()),
            GateCore,
            Transform::IDENTITY,
            ChildOf(root),
        ));
        if let Some(shell) = models
            .named
            .get("warpgate_shell")
            .and_then(|m| m.parts.first().map(|p| p.1.clone()))
        {
            let mat = glows.add(GlowMaterial {
                neon: NeonUniform::fixed(Vec3::new(0.3, 0.55, 1.0), 0.6).shield(),
                overlay: false,
            });
            commands.spawn((
                Mesh3d(shell),
                MeshMaterial3d(mat.clone()),
                GateShell(mat),
                Transform::IDENTITY,
                ChildOf(root),
            ));
        }
        if models.has("warpgate_rings") {
            let rings = models.get("warpgate_rings");
            for (k, speed) in [(0, 0.35f32), (1, -0.22)] {
                let e = commands
                    .spawn((
                        GateRings(speed),
                        Transform::from_rotation(Quat::from_rotation_y(k as f32)),
                        Visibility::default(),
                        ChildOf(root),
                    ))
                    .id();
                let mut flow = rings.clone();
                for p in &mut flow.parts {
                    if p.0 == "accent" {
                        p.0 = "flow".into();
                    }
                }
                spawn_parts(&mut commands, e, &flow, &palette, None, None, None);
                if k == 0 {
                    break;
                }
            }
        }
        let accent = accents.add(AccentMaterial {
            neon: NeonUniform::flow(Vec3::new(0.4, 0.8, 1.0), 3.0, 4.0),
            overlay: false,
        });
        commands.spawn((
            Mesh3d(meshes.add(Annulus::new(44.0, 45.0).mesh().resolution(96).build())),
            MeshMaterial3d(accent),
            Transform::from_xyz(0.0, 0.06, 0.0)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            ChildOf(root),
        ));
        commands.spawn((
            PointLight {
                color: Color::linear_rgb(0.4, 0.7, 1.0),
                intensity: 20_000_000.0,
                range: 120.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(0.0, 20.0, 0.0),
            Near(400.0),
            ChildOf(root),
        ));
    }
}

/// Only the lights near the camera (and on) are shown.
pub fn cull_lights(
    cams: Query<&GlobalTransform, With<MainCamera>>,
    mut points: Query<(&Near, &GlobalTransform, &PointLight, &mut Visibility), Without<SpotLight>>,
    mut spots: Query<(&Near, &GlobalTransform, &SpotLight, &mut Visibility), Without<PointLight>>,
) {
    let Some(eye) = cams.iter().next().map(|t| t.translation()) else {
        return;
    };
    let set = |v: &mut Visibility, on: bool| {
        let want = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
    };
    for (n, t, l, mut v) in &mut points {
        set(
            &mut v,
            l.intensity > 1.0 && (t.translation() - eye).length() < n.0,
        );
    }
    for (n, t, l, mut v) in &mut spots {
        set(
            &mut v,
            l.intensity > 1.0 && (t.translation() - eye).length() < n.0,
        );
    }
}

/// The warp gates' cores and rings turn, their shells breathe.
fn gates(
    time: Res<Time>,
    mut cores: Query<&mut Transform, (With<GateCore>, Without<GateRings>)>,
    mut rings: Query<(&GateRings, &mut Transform), Without<GateCore>>,
    shells: Query<&GateShell>,
    mut glows: ResMut<Assets<GlowMaterial>>,
) {
    let t = time.elapsed_secs_wrapped();
    for mut c in &mut cores {
        c.rotation = Quat::from_rotation_y(t * 0.6);
    }
    for (r, mut tr) in &mut rings {
        tr.rotation = Quat::from_rotation_y(t * r.0);
    }
    for s in &shells {
        if let Some(mut m) = glows.get_mut(&s.0) {
            m.neon.color.w = 0.45 + 0.35 * (t * 1.3).sin().abs();
        }
    }
}

/// Sparks rise round the warp gates near the camera.
fn sparks(
    mut commands: Commands,
    time: Res<Time>,
    assets: Option<Res<BaseAssets>>,
    gates: Query<&GateAt>,
    cams: Query<&GlobalTransform, With<MainCamera>>,
    mut q: Query<(Entity, &mut Spark, &mut Transform)>,
    mut timer: Local<f32>,
) {
    let dt = time.delta_secs();
    let eye = cams.iter().next().map(|t| t.translation());
    for (e, mut s, mut t) in &mut q {
        s.life -= dt;
        if s.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        // Gate sparks spiral up round the core; harvest sparks fly straight.
        t.translation += s.vel * dt;
        s.vel = Quat::from_rotation_y(dt * s.spin) * s.vel;
        if let Some(eye) = eye {
            t.look_at(eye, Vec3::Y);
        }
        t.scale = Vec3::splat(s.size * (s.life / 4.0).min(1.0));
    }
    let Some(assets) = assets else { return };
    *timer -= dt;
    if *timer > 0.0 {
        return;
    }
    *timer = 0.06;
    for g in &gates {
        if eye.is_none_or(|e| (e - g.0).length() > 900.0) {
            continue;
        }
        let a = rand::random::<f32>() * std::f32::consts::TAU;
        let r = 4.0 + rand::random::<f32>() * 12.0;
        let at = g.0 + Vec3::new(a.cos() * r, 0.5, a.sin() * r);
        let tangent = Vec3::new(-a.sin(), 0.0, a.cos()) * 3.0;
        commands.spawn((
            BasePart,
            Spark {
                vel: tangent + Vec3::Y * (6.0 + rand::random::<f32>() * 6.0),
                life: 4.0,
                spin: 0.8,
                size: 1.0,
            },
            Mesh3d(assets.spark.clone()),
            MeshMaterial3d(assets.spark_mat.clone()),
            Transform::from_translation(at),
        ));
    }
}

/// A spark of ore flying from a mined node at `from` into the harvester at `to`.
pub fn harvest_spark(commands: &mut Commands, assets: &BaseAssets, from: Vec3, to: Vec3) {
    let jitter = Vec3::new(
        rand::random::<f32>() - 0.5,
        rand::random::<f32>() - 0.5,
        rand::random::<f32>() - 0.5,
    ) * 1.6;
    let start = from + jitter;
    let life = 0.45 + rand::random::<f32>() * 0.25;
    commands.spawn((
        BasePart,
        Spark {
            vel: (to - start) / life,
            life,
            spin: 0.0,
            // Scale is size × life/4: about 0.5 m across at the start.
            size: 2.5 + rand::random::<f32>() * 2.0,
        },
        Mesh3d(assets.spark.clone()),
        MeshMaterial3d(assets.harvest_mat.clone()),
        Transform::from_translation(start).with_scale(Vec3::ZERO),
    ));
}

/// Ore pouring from an unloading harvester (`from`) into its silo (foot `silo`): sparks
/// arcing up into the silo's top, and a glow swirling round the rim as it fills.
pub fn unload_fx(commands: &mut Commands, assets: &BaseAssets, from: Vec3, silo: Vec3) {
    let top = silo + Vec3::Y * (ps_sim::base::SILO_H + 0.5);
    let jitter = Vec3::new(
        rand::random::<f32>() - 0.5,
        rand::random::<f32>() - 0.5,
        rand::random::<f32>() - 0.5,
    );
    let start = from + jitter * 1.6;
    let life = 0.6 + rand::random::<f32>() * 0.3;
    // (Thrown up so that it falls into the top: up a little more than straight there.)
    let vel = (top - start) / life + Vec3::Y * 4.0 * life;
    commands.spawn((
        BasePart,
        Spark {
            vel,
            life,
            spin: 0.0,
            size: 2.5 + rand::random::<f32>() * 2.0,
        },
        Mesh3d(assets.spark.clone()),
        MeshMaterial3d(assets.harvest_mat.clone()),
        Transform::from_translation(start).with_scale(Vec3::ZERO),
    ));
    if rand::random::<f32>() < 0.5 {
        let a = rand::random::<f32>() * std::f32::consts::TAU;
        let r = ps_sim::base::SILO_R * (0.6 + rand::random::<f32>() * 0.4);
        let at = top + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
        let tangent = Vec3::new(-a.sin(), 0.0, a.cos()) * 3.0;
        commands.spawn((
            BasePart,
            Spark {
                vel: tangent + Vec3::Y * (2.0 + rand::random::<f32>() * 2.0),
                life: 1.2,
                spin: 1.6,
                size: 1.6,
            },
            Mesh3d(assets.spark.clone()),
            MeshMaterial3d(assets.harvest_mat.clone()),
            Transform::from_translation(at),
        ));
    }
}

/// Resource nodes where the snapshot has them, shrinking as they're mined.
fn nodes(
    mut commands: Commands,
    game: Res<Game>,
    assets: Option<Res<BaseAssets>>,
    mut q: Query<(Entity, &Node, &mut Transform)>,
    palette: Res<Palette>,
) {
    let Some(assets) = assets else { return };
    let mut seen: Vec<u32> = Vec::new();
    for (e, n, mut t) in &mut q {
        match game.snap.nodes.iter().find(|s| s.id == n.0) {
            Some(s) => {
                // (Full size at the biggest a node grows: `NODE_SIZES`' last.)
                let big = ps_sim::sim::NODE_SIZES[2] * ps_sim::sim::CARGO;
                t.scale = Vec3::splat(0.5 + (s.amount as f32 / big).min(1.0) * 0.8);
                seen.push(n.0);
            }
            None => commands.entity(e).despawn(),
        }
    }
    for s in &game.snap.nodes {
        if seen.contains(&s.id) {
            continue;
        }
        let e = commands
            .spawn((
                BasePart,
                Node(s.id),
                Transform::from_translation(s.pos)
                    .with_rotation(Quat::from_rotation_y(s.id as f32)),
                Visibility::default(),
            ))
            .id();
        for (mat, mesh) in &assets.node.parts {
            let mut c = commands.spawn((Mesh3d(mesh.clone()), ChildOf(e)));
            if mat == "resource" {
                c.insert(MeshMaterial3d(assets.node_mat.clone()));
            } else {
                c.insert(MeshMaterial3d(palette.body.clone()));
            }
        }
        commands.spawn((
            PointLight {
                color: Color::linear_rgb(1.0, 0.6, 0.15),
                intensity: 300_000.0,
                range: 16.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(0.0, 2.0, 0.0),
            Near(150.0),
            ChildOf(e),
        ));
    }
}

/// Forcefields, generator cores and silos as the bases' power and resources are.
#[allow(clippy::type_complexity)]
fn power(
    game: Res<Game>,
    time: Res<Time>,
    mut fields: Query<(&Field, &mut Visibility), Without<GeneratorCore>>,
    mut cores: Query<
        (&GeneratorCore, &mut Transform, &mut Visibility),
        (Without<Field>, Without<SiloFill>),
    >,
    mut fills: Query<(&SiloFill, &mut Transform), Without<GeneratorCore>>,
    mut gauges: Query<
        (
            &SiloGauge,
            &mut Transform,
            &mut MeshMaterial3d<AccentMaterial>,
        ),
        (Without<GeneratorCore>, Without<SiloFill>),
    >,
) {
    let powered = |b: BaseIdx| {
        game.snap
            .bases
            .get(b as usize)
            .is_none_or(|s| s.generator > 0.0)
    };
    for (f, mut v) in &mut fields {
        let want = if powered(f.0) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
    }
    let t = time.elapsed_secs_wrapped();
    for (c, mut tr, mut v) in &mut cores {
        let on = powered(c.0);
        *v = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        tr.scale = Vec3::splat(1.0 + 0.06 * (t * 4.0 + c.0 as f32).sin());
    }
    for (f, mut tr) in &mut fills {
        if let Some(s) = game.snap.bases.get(f.0 as usize) {
            let k = (s.resources as f32 / ps_sim::state::SILO_CAP).clamp(0.0, 1.0);
            tr.scale.y = (k * 11.4).max(0.05);
        }
    }
    for (g, mut tr, mut mat) in &mut gauges {
        if let Some(s) = game.snap.bases.get(g.0 as usize) {
            let k = (s.resources as f32 / ps_sim::state::SILO_CAP).clamp(0.0, 1.0);
            tr.scale.y = (k * GAUGE_H).max(0.05);
            let want = if k < GAUGE_LOW { &g.2 } else { &g.1 };
            if mat.0 != *want {
                mat.0 = want.clone();
            }
        }
    }
}

/// Recolours the parts of bases that changed hands.
fn retint(
    game: Res<Game>,
    palette: Res<Palette>,
    mut shown: ResMut<Shown>,
    mut glows: ResMut<Assets<GlowMaterial>>,
    mut accents_q: Query<(&BaseTint, &mut MeshMaterial3d<AccentMaterial>)>,
    mut glow_handles: Query<
        (&BaseTint, &mut MeshMaterial3d<GlowMaterial>),
        Without<MeshMaterial3d<AccentMaterial>>,
    >,
) {
    let Some(c) = game.continent else { return };
    let mut changed: Vec<(BaseIdx, Option<Team>)> = Vec::new();
    for (b, was) in shown.0.iter_mut() {
        let now = game
            .state
            .continents
            .get(c as usize)
            .and_then(|s| s.bases.get(*b as usize))
            .and_then(|s| s.owner);
        if now != *was {
            *was = now;
            changed.push((*b, now));
        }
    }
    if changed.is_empty() {
        return;
    }
    for (tint, mut m) in &mut accents_q {
        if let Some((_, owner)) = changed.iter().find(|(b, _)| *b == tint.0) {
            m.0 = palette.accent_for(*owner);
        }
    }
    for (tint, mut m) in &mut glow_handles {
        if let Some((_, owner)) = changed.iter().find(|(b, _)| *b == tint.0) {
            if let Some(mut mat) = glows.get_mut(&m.0)
                && mat.neon.extra.x == 3.0
            {
                mat.neon.color = team_color(*owner).extend(tint.1);
            } else {
                m.0 = palette.glow_for(*owner);
            }
        }
    }
}

/// Turret heads turn as the snapshot says (the one the player mans, as predicted, and not
/// drawn from inside); destroyed ones slump.
fn turrets(
    game: Res<Game>,
    view: Res<crate::seatview::SeatView>,
    look: Option<Res<crate::desktop::Look>>,
    mut heads: Query<(&TurretHead, &mut Transform, &mut Visibility)>,
) {
    let Some(map) = &game.map else { return };
    let inside = crate::seatview::from_inside(&view, look.as_deref())
        .then_some(view.turret)
        .flatten();
    // Turret indices: the bases' mounts in order (as the sim numbers them).
    let mut index = Vec::new();
    for (b, l) in map.bases.iter().enumerate() {
        for i in 0..l.turrets.len() {
            index.push((b as BaseIdx, i as u8));
        }
    }
    for (h, mut t, mut vis) in &mut heads {
        let mine = view.turret == Some(h.0 as u16);
        let want = if inside == Some(h.0 as u16) {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *vis != want {
            *vis = want;
        }
        if mine && view.active {
            t.rotation = view.rot;
            continue;
        }
        let Some(&(b, i)) = index.get(h.0) else {
            continue;
        };
        if let Some(s) = game
            .snap
            .turrets
            .iter()
            .find(|s| s.base == b && s.index == i)
        {
            if s.health <= 0.0 {
                t.rotation = Quat::from_rotation_y(s.yaw)
                    * Quat::from_rotation_x(-0.5)
                    * Quat::from_rotation_z(0.3);
            } else {
                t.rotation = Quat::from_rotation_y(s.yaw) * Quat::from_rotation_x(s.pitch);
            }
        }
    }
}

fn clear(mut commands: Commands, parts: Query<Entity, With<BasePart>>) {
    for e in &parts {
        commands.entity(e).despawn();
    }
}
