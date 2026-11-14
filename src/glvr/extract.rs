//! What the render thread draws, taken from the game's world after each update (as
//! stealth-mission-89's src/glvr/extract.rs): the visible meshes seen by the eyes (frustum
//! culled) with their transforms and materials (StandardMaterial lit, the neon accents and
//! their halos), the neon's frame state, the light, and the rig placing the tracking space in
//! the arena. Meshes go to the render thread once (again when changed), on their own channel
//! so a draw list replaced before it was drawn loses none.

use super::render::{AssetMsg, MeshData, eye_views, inside, planes};
use crate::materials::{
    AccentMaterial, BLUE, GlowMaterial, NeonGlobals, PULSES, ParticleMaterial, RED, SHOT_LIGHTS,
};
use crate::vr::{HandInput, XrRig};
use bevy::asset::AssetId;
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::GlobalAmbientLight;
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use openxr as xr;
use std::collections::HashSet;
use std::sync::mpsc::Sender;

/// The eyes' fields of view are widened this much for culling: the draw list is drawn a frame
/// after the poses it was culled with.
const CULL_MARGIN: f32 = 1.15;

/// The point lights drawn (the nearest to the eyes): the bolts' lights.
pub const POINT_LIGHTS: usize = 16;

/// Bevy's default camera exposure (EV100 9.7) as a factor: lux and cd/m² to the shader's units.
const EXPOSURE: f32 = 1.0 / 998.0;

/// Which views see an item: the eyes, the scope (src/scope.rs).
pub const IN_EYES: u8 = 1;
pub const IN_SCOPE: u8 = 2;

/// A StandardMaterial mesh to draw.
pub struct LitItem {
    pub mesh: AssetId<Mesh>,
    /// `IN_EYES` | `IN_SCOPE`.
    pub views: u8,
    pub model: Mat4,
    /// Linear RGBA.
    pub base: Vec4,
    pub double_sided: bool,
    /// The surface's detail (lit.glsl): `LIT_PLAIN`, `LIT_HULL` (plating: `frame` world to
    /// object, `detail` its params and seam light) or `LIT_TERRAIN` (`detail`: the contour,
    /// grid, params and shore of terrain.wgsl's lines).
    pub kind: i32,
    pub frame: Mat4,
    pub detail: [Vec4; 4],
}

pub const LIT_PLAIN: i32 = 0;
pub const LIT_HULL: i32 = 1;
pub const LIT_TERRAIN: i32 = 2;

/// A neon mesh (accent or halo) to draw: its material's colour and params (`NeonUniform`).
pub struct NeonItem {
    pub mesh: AssetId<Mesh>,
    pub views: u8,
    pub model: Mat4,
    pub colour: Vec4,
    pub params: Vec4,
    pub extra: Vec4,
}

/// A soft particle (smoke, fire, a flash: `ParticleMaterial`) to draw, blended after the halos.
pub struct ParticleItem {
    pub mesh: AssetId<Mesh>,
    pub views: u8,
    pub model: Mat4,
    pub colour: Vec4,
    pub params: Vec4,
    pub additive: bool,
    /// How far from the eyes (squared): drawn farthest first.
    pub far: f32,
}

/// The neon's state this update (`NeonGlobals`).
#[derive(Clone, Copy, Default)]
pub struct NeonFrame {
    pub time: f32,
    pub red: Vec3,
    pub blue: Vec3,
    pub pulse_pos: [Vec4; PULSES],
    pub pulse_colour: [Vec4; PULSES],
    /// `NeonGlobals::shots`.
    pub shots: [Vec4; SHOT_LIGHTS],
    pub zones: Vec4,
}

/// The prism lance's scope (src/scope.rs): the lens (world: centre, facing +Z, scaled to its
/// radius) and, held up to an eye, the view it shows (world to clip) from `eye`.
#[derive(Clone, Copy)]
pub struct ScopeDraw {
    pub lens: Mat4,
    pub view: Option<(Mat4, Vec3)>,
    /// The team's colour (the reticle), linear.
    pub colour: Vec3,
}

/// One update's picture.
#[derive(Default)]
pub struct DrawList {
    /// The tracking space in the world (`XrRig`).
    pub rig: Mat4,
    pub clear: Vec4,
    /// Towards the directional light, its colour times strength, the ambient light.
    pub light: (Vec3, Vec3, Vec3),
    pub neon: NeonFrame,
    pub lit: Vec<LitItem>,
    pub accents: Vec<NeonItem>,
    pub glows: Vec<NeonItem>,
    /// Overlays (the VR menus: `materials::NeonKey`), drawn last over everything.
    pub overlay_accents: Vec<NeonItem>,
    pub overlay_glows: Vec<NeonItem>,
    /// The soft particles, farthest first.
    pub particles: Vec<ParticleItem>,
    /// Point lights: position and range (w), colour times strength at 1 m (rgb).
    pub points: Vec<(Vec4, Vec3)>,
    pub scope: Option<ScopeDraw>,
    pub culled: u32,
    /// Vibrations asked for (`XrInput::haptic`), per hand.
    pub haptics: [f32; 2],
}

/// The eyes and the controllers as the render thread last saw them (tracking space).
#[derive(Clone)]
pub struct Eyes {
    pub views: Vec<xr::View>,
    pub controllers: [HandInput; 2],
}

type Common = (
    &'static Mesh3d,
    &'static GlobalTransform,
    &'static InheritedVisibility,
    Option<&'static Aabb>,
    Has<NoFrustumCulling>,
);

/// A mesh's common components, as a query gives them.
type CommonItem<'a> = (
    &'a Mesh3d,
    &'a GlobalTransform,
    &'a InheritedVisibility,
    Option<&'a Aabb>,
    bool,
);

/// The frusta culled to: the eyes' (none: everything is seen) and the scope's, if it is drawn.
struct Frusta {
    eyes: Vec<[Vec4; 5]>,
    scope: Option<[Vec4; 5]>,
    /// Between the eyes: small things farther than `FAR` aren't drawn (the fog hides them).
    centre: Option<Vec3>,
}

/// How far small things are drawn (m); the land and other big things always.
const FAR: f32 = 450.0;

/// Which views see `c` (`IN_EYES` | `IN_SCOPE`; 0 if none do), or None if it is hidden.
fn seen(frusta: &Frusta, c: &CommonItem) -> Option<u8> {
    let (_, transform, visible, aabb, no_cull) = *c;
    if !visible.get() {
        return None;
    }
    let scope = if frusta.scope.is_some() { IN_SCOPE } else { 0 };
    let (Some(aabb), false) = (aabb, no_cull) else {
        return Some(IN_EYES | scope);
    };
    if let Some(c) = frusta.centre {
        let mid = transform.transform_point(Vec3::from(aabb.center));
        let radius = (Vec3::from(aabb.half_extents) * transform.scale()).length();
        if radius < 60.0 && (mid - c).length() - radius > FAR {
            return Some(0);
        }
    }
    let eyes = frusta.eyes.is_empty() || frusta.eyes.iter().any(|f| inside(f, transform, aabb));
    let scope = frusta.scope.is_some_and(|f| inside(&f, transform, aabb));
    Some(if eyes { IN_EYES } else { 0 } | if scope { IN_SCOPE } else { 0 })
}

/// The queries of the meshes drawn, by material.
struct Queries {
    standard: QueryState<
        (Common, &'static MeshMaterial3d<StandardMaterial>),
        Without<crate::vr::Vignette>,
    >,
    /// The plated structures and vehicles, and the land: drawn lit in their base colour (the
    /// plating's and the contour lines' detail is the desktop renderer's).
    hulls: QueryState<(Common, &'static MeshMaterial3d<crate::hull::HullMaterial>)>,
    terrain: QueryState<(
        Common,
        &'static MeshMaterial3d<crate::terrain::TerrainMaterial>,
    )>,
    accents: QueryState<(Common, &'static MeshMaterial3d<AccentMaterial>)>,
    glows: QueryState<(Common, &'static MeshMaterial3d<GlowMaterial>)>,
    particles: QueryState<(Common, &'static MeshMaterial3d<ParticleMaterial>)>,
    lights: QueryState<(&'static DirectionalLight, &'static GlobalTransform)>,
    points: QueryState<(
        &'static PointLight,
        &'static GlobalTransform,
        &'static InheritedVisibility,
    )>,
    transforms: QueryState<&'static GlobalTransform>,
}

pub struct Extractor {
    assets: Sender<AssetMsg>,
    /// Meshes the render thread has.
    sent: HashSet<AssetId<Mesh>>,
    queries: Option<Queries>,
}

impl Extractor {
    pub fn new(assets: Sender<AssetMsg>) -> Self {
        Extractor {
            assets,
            sent: HashSet::new(),
            queries: None,
        }
    }

    /// Meshes changed or gone: the render thread drops them, they are sent again when drawn.
    pub fn forget(&mut self, ids: impl Iterator<Item = AssetId<Mesh>>) {
        for id in ids {
            if self.sent.remove(&id) {
                let _ = self.assets.send(AssetMsg::ForgetMesh(id));
            }
        }
    }

    /// Sends mesh `id` if the render thread doesn't have it: whether it has it now.
    fn send_mesh(&mut self, meshes: &Assets<Mesh>, id: AssetId<Mesh>) -> bool {
        if self.sent.contains(&id) {
            return true;
        }
        let Some(data) = meshes.get(id).and_then(mesh_data) else {
            return false;
        };
        let _ = self.assets.send(AssetMsg::Mesh(id, data));
        self.sent.insert(id);
        true
    }

    /// The views that draw `c` (its mesh sent if need be), or None.
    fn take(
        &mut self,
        frusta: &Frusta,
        list: &mut DrawList,
        meshes: &Assets<Mesh>,
        c: &CommonItem,
    ) -> Option<u8> {
        match seen(frusta, c)? {
            0 => {
                list.culled += 1;
                None
            }
            views => self.send_mesh(meshes, c.0.0.id()).then_some(views),
        }
    }

    /// The draw list of `world`, culled to `eyes` (everything while there are none yet).
    pub fn extract(&mut self, world: &mut World, eyes: Option<&Eyes>) -> DrawList {
        let mut q = self.queries.take().unwrap_or_else(|| Queries {
            standard: world.query_filtered(),
            hulls: world.query(),
            terrain: world.query(),
            accents: world.query(),
            glows: world.query(),
            particles: world.query(),
            lights: world.query(),
            points: world.query(),
            transforms: world.query(),
        });
        let mut list = DrawList::default();
        {
            let mut input = world.resource_mut::<crate::vr::XrInput>();
            list.haptics = std::mem::take(&mut input.haptic);
        }
        let world: &World = world;
        let rig = world.get_resource::<XrRig>().map(|r| r.0);
        list.rig = rig
            .and_then(|r| q.transforms.get(world, r).ok())
            .map_or(Mat4::IDENTITY, |t| t.to_matrix());
        let eye_frusta: Vec<[Vec4; 5]> = eyes
            .map(|e| {
                eye_views(list.rig, &e.views, CULL_MARGIN)
                    .iter()
                    .map(|v| planes(v.view_proj))
                    .collect()
            })
            .unwrap_or_default();
        // (No scope in this game: the renderer keeps its support for one.)
        list.scope = None;
        let frusta = Frusta {
            eyes: eye_frusta,
            scope: list.scope.and_then(|s| s.view).map(|(vp, _)| planes(vp)),
            centre: eyes.and_then(|e| eye_views(list.rig, &e.views, 1.0).first().map(|v| v.eye)),
        };
        list.clear = world
            .get_resource::<ClearColor>()
            .map_or(Vec4::new(0.0, 0.0, 0.0, 1.0), |c| {
                Vec4::from_array(c.0.to_linear().to_f32_array())
            });
        let (dir, colour) = q
            .lights
            .iter(world)
            .next()
            .map_or((Vec3::Y, Vec3::ZERO), |(l, t)| {
                let c = l.color.to_linear();
                (
                    t.back().as_vec3(),
                    // (No shadows here: a weaker sun, or it lights the rooms inside.)
                    Vec3::new(c.red, c.green, c.blue) * l.illuminance * EXPOSURE * 0.4,
                )
            });
        let ambient = world
            .get_resource::<GlobalAmbientLight>()
            .map_or(Vec3::ZERO, |a| {
                let c = a.color.to_linear();
                Vec3::new(c.red, c.green, c.blue) * a.brightness * EXPOSURE * 1.3
            });
        list.light = (dir, colour, ambient);
        // The point lights nearest the eyes (lumens to candela at 1 m, then exposure).
        let centre = eyes
            .and_then(|e| eye_views(list.rig, &e.views, 1.0).first().map(|v| v.eye))
            .unwrap_or(list.rig.w_axis.truncate());
        let mut points: Vec<(f32, Vec4, Vec3)> = q
            .points
            .iter(world)
            .filter(|(_, _, v)| v.get())
            .map(|(l, t, _)| {
                let p = t.translation();
                let c = l.color.to_linear();
                let strength = l.intensity / (4.0 * std::f32::consts::PI) * EXPOSURE;
                (
                    (p - centre).length_squared(),
                    p.extend(l.range),
                    Vec3::new(c.red, c.green, c.blue) * strength,
                )
            })
            .collect();
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        list.points = points
            .into_iter()
            .take(POINT_LIGHTS)
            .map(|(_, p, c)| (p, c))
            .collect();
        if let Some(g) = world.get_resource::<NeonGlobals>() {
            let mut n = NeonFrame {
                time: g.time,
                red: RED,
                blue: BLUE,
                ..default()
            };
            for (i, p) in g.pulses.iter().enumerate() {
                n.pulse_pos[i] = p.0.extend(p.1);
                n.pulse_colour[i] = p.2.extend(1.0);
            }
            n.shots = g.shots;
            n.zones = g.zones;
            list.neon = n;
        }
        let meshes = world.resource::<Assets<Mesh>>();
        let materials = world.resource::<Assets<StandardMaterial>>();
        for (common, material) in q.standard.iter(world) {
            let Some(m) = materials.get(&material.0) else {
                continue;
            };
            let Some(views) = self.take(&frusta, &mut list, meshes, &common) else {
                continue;
            };
            list.lit.push(LitItem {
                mesh: common.0.0.id(),
                views,
                model: common.1.to_matrix(),
                base: Vec4::from_array(m.base_color.to_linear().to_f32_array()),
                double_sided: m.double_sided || m.cull_mode.is_none(),
                kind: LIT_PLAIN,
                frame: Mat4::IDENTITY,
                detail: [Vec4::ZERO; 4],
            });
        }
        let hulls = world.resource::<Assets<crate::hull::HullMaterial>>();
        for (common, material) in q.hulls.iter(world) {
            let Some(m) = hulls.get(&material.0) else {
                continue;
            };
            let Some(views) = self.take(&frusta, &mut list, meshes, &common) else {
                continue;
            };
            list.lit.push(LitItem {
                mesh: common.0.0.id(),
                views,
                model: common.1.to_matrix(),
                base: Vec4::from_array(m.base.base_color.to_linear().to_f32_array()),
                double_sided: m.base.double_sided || m.base.cull_mode.is_none(),
                kind: LIT_HULL,
                frame: m.extension.hull.frame,
                detail: [
                    m.extension.hull.params,
                    m.extension.hull.seam_light,
                    Vec4::ZERO,
                    Vec4::ZERO,
                ],
            });
        }
        let lands = world.resource::<Assets<crate::terrain::TerrainMaterial>>();
        for (common, material) in q.terrain.iter(world) {
            let Some(m) = lands.get(&material.0) else {
                continue;
            };
            let Some(views) = self.take(&frusta, &mut list, meshes, &common) else {
                continue;
            };
            list.lit.push(LitItem {
                mesh: common.0.0.id(),
                views,
                model: common.1.to_matrix(),
                base: Vec4::from_array(m.base.base_color.to_linear().to_f32_array()),
                double_sided: false,
                kind: LIT_TERRAIN,
                frame: Mat4::IDENTITY,
                detail: {
                    let l = m.extension.lines;
                    [l.contour, l.grid, l.params, l.shore]
                },
            });
        }
        let materials = world.resource::<Assets<AccentMaterial>>();
        for (common, material) in q.accents.iter(world) {
            let Some(m) = materials.get(&material.0) else {
                continue;
            };
            let Some(views) = self.take(&frusta, &mut list, meshes, &common) else {
                continue;
            };
            let to = if m.overlay {
                &mut list.overlay_accents
            } else {
                &mut list.accents
            };
            to.push(NeonItem {
                mesh: common.0.0.id(),
                views,
                model: common.1.to_matrix(),
                colour: m.neon.color,
                params: m.neon.params,
                extra: m.neon.extra,
            });
        }
        let materials = world.resource::<Assets<GlowMaterial>>();
        for (common, material) in q.glows.iter(world) {
            let Some(m) = materials.get(&material.0) else {
                continue;
            };
            let Some(views) = self.take(&frusta, &mut list, meshes, &common) else {
                continue;
            };
            let to = if m.overlay {
                &mut list.overlay_glows
            } else {
                &mut list.glows
            };
            to.push(NeonItem {
                mesh: common.0.0.id(),
                views,
                model: common.1.to_matrix(),
                colour: m.neon.color,
                params: m.neon.params,
                extra: m.neon.extra,
            });
        }
        let materials = world.resource::<Assets<ParticleMaterial>>();
        let centre = frusta.centre.unwrap_or(list.rig.w_axis.truncate());
        for (common, material) in q.particles.iter(world) {
            let Some(m) = materials.get(&material.0) else {
                continue;
            };
            let Some(views) = self.take(&frusta, &mut list, meshes, &common) else {
                continue;
            };
            list.particles.push(ParticleItem {
                mesh: common.0.0.id(),
                views,
                model: common.1.to_matrix(),
                colour: m.particle.color,
                params: m.particle.params,
                additive: m.additive,
                far: (common.1.translation() - centre).length_squared(),
            });
        }
        list.particles.sort_by(|a, b| b.far.total_cmp(&a.far));
        self.queries = Some(q);
        list
    }
}

/// What the render thread needs of `mesh`: positions, normals, UVs, indices, topology.
fn mesh_data(mesh: &Mesh) -> Option<MeshData> {
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return None;
    };
    let normals = match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
        Some(VertexAttributeValues::Float32x3(n)) => Some(n.clone()),
        _ => None,
    };
    let uvs = match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
        Some(VertexAttributeValues::Float32x2(uv)) => Some(uv.clone()),
        _ => None,
    };
    let colours = match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
        Some(VertexAttributeValues::Float32x4(c)) => Some(c.clone()),
        _ => None,
    };
    Some(MeshData {
        positions: positions.clone(),
        normals,
        uvs,
        colours,
        indices: mesh.indices().cloned(),
        topology: mesh.primitive_topology(),
    })
}
