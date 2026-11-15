//! The plated look of structures and vehicles (assets/shaders/hull.wgsl): a lighter, metallic
//! dark grey with panels, seams, rivets and grime, instead of plain black. Structures share
//! world-space materials; each vehicle gets its own, following it, so its panels stay put on
//! it as it moves.

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
pub struct HullExt {
    #[uniform(100)]
    pub hull: HullUniform,
}

#[derive(ShaderType, Clone, Copy, Debug)]
pub struct HullUniform {
    pub frame: Mat4,
    pub params: Vec4,
    pub seam_light: Vec4,
}

impl MaterialExtension for HullExt {
    fn fragment_shader() -> ShaderRef {
        "shaders/hull.wgsl".into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/hull.wgsl".into()
    }
}

pub type HullMaterial = ExtendedMaterial<StandardMaterial, HullExt>;

/// The shared structure materials: the compound's walls (big panels), floors, furniture, the
/// buildings' lighter cladding, matte roofs, the railings' bare metal, the stairwells' warm
/// plating.
#[derive(Resource, Clone)]
pub struct Hulls {
    pub wall: Handle<HullMaterial>,
    pub floor: Handle<HullMaterial>,
    pub decor: Handle<HullMaterial>,
    pub building: Handle<HullMaterial>,
    pub roof: Handle<HullMaterial>,
    pub metal: Handle<HullMaterial>,
    pub stairwell: Handle<HullMaterial>,
    /// The bases' windows: dark, glossy glass.
    pub glass: Handle<HullMaterial>,
}

impl Hulls {
    /// The bases' bodies' materials, in the order src/bases.rs sorts them.
    pub fn surfaces(&self) -> [&Handle<HullMaterial>; 7] {
        [
            &self.wall,
            &self.floor,
            &self.decor,
            &self.building,
            &self.roof,
            &self.metal,
            &self.stairwell,
        ]
    }
}

pub fn hull(base: Color, panel: f32, roughness: f32, metallic: f32) -> HullMaterial {
    ExtendedMaterial {
        base: StandardMaterial {
            base_color: base,
            perceptual_roughness: roughness,
            metallic,
            reflectance: 0.4,
            ..default()
        },
        extension: HullExt {
            hull: HullUniform {
                frame: Mat4::IDENTITY,
                params: Vec4::new(panel, 0.55, 0.6, 0.35),
                seam_light: Vec4::ZERO,
            },
        },
    }
}

/// A vehicle's own material (its frame updated as it moves), its seams faintly in `glow`.
pub fn vehicle_hull(glow: Vec3) -> HullMaterial {
    let mut m = hull(Color::linear_rgb(0.11, 0.115, 0.125), 0.9, 0.42, 0.55);
    m.extension.hull.seam_light = glow.extend(0.15);
    m
}

/// A vehicle's armour plates: lighter, larger panels in a breath of `team`'s colour.
pub fn vehicle_armor(team: Vec3) -> HullMaterial {
    let c = Vec3::new(0.17, 0.175, 0.185).lerp(team * 0.3, 0.2);
    let mut m = hull(Color::linear_rgb(c.x, c.y, c.z), 1.6, 0.52, 0.35);
    m.extension.hull.params = Vec4::new(1.6, 0.45, 0.8, 0.5);
    m
}

/// Follows the object at `t`: the material's frame turns world positions into its own.
pub fn follow(m: &mut HullMaterial, t: &GlobalTransform) {
    m.extension.hull.frame = t.to_matrix().inverse();
}

pub struct HullPlugin;

impl Plugin for HullPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<HullMaterial>::default())
            .add_systems(PreStartup, make);
    }
}

fn make(mut commands: Commands, mut mats: ResMut<Assets<HullMaterial>>) {
    commands.insert_resource(Hulls {
        wall: mats.add(hull(Color::linear_rgb(0.085, 0.09, 0.1), 2.0, 0.6, 0.25)),
        floor: mats.add(hull(Color::linear_rgb(0.06, 0.062, 0.07), 1.5, 0.75, 0.1)),
        decor: mats.add(hull(Color::linear_rgb(0.1, 0.1, 0.105), 0.6, 0.5, 0.4)),
        building: mats.add(hull(Color::linear_rgb(0.125, 0.13, 0.14), 1.4, 0.55, 0.3)),
        roof: mats.add(hull(Color::linear_rgb(0.05, 0.052, 0.058), 3.0, 0.85, 0.05)),
        metal: mats.add(hull(Color::linear_rgb(0.3, 0.31, 0.33), 0.4, 0.35, 0.85)),
        stairwell: mats.add(hull(Color::linear_rgb(0.135, 0.12, 0.105), 1.0, 0.6, 0.3)),
        glass: mats.add(hull(Color::linear_rgb(0.015, 0.03, 0.045), 1.75, 0.08, 0.8)),
    });
}
