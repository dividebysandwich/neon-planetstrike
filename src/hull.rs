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

/// The shared structure materials: walls and buildings (big panels), floors, furniture.
#[derive(Resource, Clone)]
pub struct Hulls {
    pub wall: Handle<HullMaterial>,
    pub floor: Handle<HullMaterial>,
    pub decor: Handle<HullMaterial>,
    /// The bases' windows: dark, glossy glass.
    pub glass: Handle<HullMaterial>,
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
        glass: mats.add(hull(Color::linear_rgb(0.015, 0.03, 0.045), 1.75, 0.08, 0.8)),
    });
}
