//! The neon look: `AccentMaterial` (glowing strips, opaque, HDR so bloom picks it up) and
//! `GlowMaterial` (soft additive halos beside the strips, which fake the bloom where there is
//! none: the foveated GL renderer draws straight into the headset's swapchain). Both share
//! `NeonGlobals` (time, team colours, event pulses), copied into them every frame.
//! Shader: assets/shaders/neon.wgsl (and its GL port assets/shaders/gl/neon.glsl).

use bevy::material::OpaqueRendererMethod;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, RenderPipelineDescriptor,
    ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use ps_sim::Team;

const SHADER: &str = "shaders/neon.wgsl";
pub const PULSES: usize = 8;
/// Shots lighting up the arena's dimmed lights around them (`NeonGlobals::shots`).
pub const SHOT_LIGHTS: usize = 8;

/// Team colours (linear, HDR multipliers come from the material's intensity).
pub const RED: Vec3 = Vec3::new(1.0, 0.06, 0.08);
pub const BLUE: Vec3 = Vec3::new(0.08, 0.35, 1.0);
pub const GREEN: Vec3 = Vec3::new(0.1, 1.0, 0.25);
pub const WHITE: Vec3 = Vec3::new(1.0, 1.0, 1.0);
/// The arena's middle and the scenery: purple, yellow, green, cyan, orange (white is kept for
/// reload stations, the team colours for the halves behind the targets).
pub const ZONE_COLORS: [Vec3; 5] = [
    Vec3::new(0.5, 0.08, 1.0),
    Vec3::new(1.0, 0.75, 0.04),
    Vec3::new(0.08, 1.0, 0.2),
    Vec3::new(0.04, 0.85, 0.9),
    Vec3::new(1.0, 0.35, 0.02),
];

pub fn team_color(team: Option<Team>) -> Vec3 {
    match team {
        Some(Team::Red) => RED,
        Some(Team::Blue) => BLUE,
        Some(Team::Green) => GREEN,
        None => WHITE,
    }
}

#[derive(ShaderType, Clone, Copy, Debug)]
pub struct NeonUniform {
    pub color: Vec4,
    pub params: Vec4,
    pub red: Vec4,
    pub blue: Vec4,
    pub pulse_pos: [Vec4; PULSES],
    pub pulse_color: [Vec4; PULSES],
    /// x: halo shape (0 strip, 1 volume: fades where seen edge on, 2 haze, 3 forcefield, 4
    /// cloak), y: flow speed (m/s, mode 2), z: 1 for the arena's own lights (dimmed in a
    /// blackout), w: how bright those are now (`NeonGlobals::dim`).
    pub extra: Vec4,
    /// Shots near the eye (xyz, w strength; 0: none): the dimmed lights brighten around them.
    pub shots: [Vec4; SHOT_LIGHTS],
    /// How dark each quarter of the arena is (0..1; a blackout breaker's, `NeonGlobals::zones`),
    /// in `ps_sim::console::quarter`'s order.
    pub zones: Vec4,
}

impl NeonUniform {
    /// Coloured by the half of the arena it's in.
    pub fn team_zoned(intensity: f32) -> Self {
        Self::new(Vec3::ONE, intensity, 0.0)
    }

    pub fn fixed(color: Vec3, intensity: f32) -> Self {
        Self::new(color, intensity, 1.0)
    }

    /// A fixed colour with pulses of light flowing along the strip (uv.x: metres) at `speed`.
    pub fn flow(color: Vec3, intensity: f32, speed: f32) -> Self {
        let mut n = Self::new(color, intensity, 2.0);
        n.extra.y = speed;
        n
    }

    /// The mesh's vertex colours, times `intensity` (the sky).
    pub fn vertex_coloured(intensity: f32) -> Self {
        Self::new(Vec3::ONE, intensity, 3.0)
    }

    /// As a halo: a volume (light cones), fading where seen edge on.
    pub fn volume(mut self) -> Self {
        self.extra.x = 1.0;
        self
    }

    /// As a halo: haze, in slowly drifting patches.
    pub fn haze(mut self) -> Self {
        self.extra.x = 2.0;
        self
    }

    /// As a halo: a translucent forcefield (uv 0..1 across and up the sheet).
    pub fn shield(mut self) -> Self {
        self.extra.x = 3.0;
        self
    }

    /// One of the arena's own lights: dimmed in a blackout (`NeonGlobals::dim`).
    pub fn dimmable(mut self) -> Self {
        self.extra.z = 1.0;
        self.extra.w = 1.0;
        self
    }

    /// As a halo: a cloaked player's shimmer, brightest at the silhouette.
    pub fn cloak(mut self) -> Self {
        self.extra.x = 4.0;
        self
    }

    fn new(color: Vec3, intensity: f32, mode: f32) -> Self {
        NeonUniform {
            color: color.extend(intensity),
            params: Vec4::new(mode, 0.0, 0.0, 100.0),
            red: RED.extend(1.0),
            blue: BLUE.extend(1.0),
            pulse_pos: [Vec4::new(0.0, 0.0, 0.0, -1.0); PULSES],
            pulse_color: [Vec4::ZERO; PULSES],
            extra: Vec4::ZERO,
            shots: [Vec4::ZERO; SHOT_LIGHTS],
            zones: Vec4::ZERO,
        }
    }

    pub fn mode(&self) -> f32 {
        self.params.x
    }

    fn apply(&mut self, g: &NeonGlobals, glow: bool) {
        self.params.y = if glow { 1.0 } else { 0.0 };
        self.params.z = g.time;
        self.params.w = g.half_length;
        self.red = RED.extend(1.0);
        self.blue = BLUE.extend(1.0);
        self.extra.w = g.dim;
        self.shots = g.shots;
        self.zones = g.zones;
        for (i, p) in g.pulses.iter().enumerate() {
            self.pulse_pos[i] = p.0.extend(p.1);
            self.pulse_color[i] = p.2.extend(1.0);
        }
    }
}

/// How deep overlays draw: the world's depth from the near plane to this far is kept in front
/// of them (reversed depth, 1 at the near plane), in step with neon.wgsl's OVERLAY_DEPTH.
pub const OVERLAY_DEPTH: f32 = 0.01;

/// The variant of a neon material's pipeline: an overlay (the VR menus, scoreboards and the
/// pointers at them) is drawn over the world, never hidden behind it, and event pulses don't
/// light it up. The GL headset renderer draws overlays last, after clearing the depth.
#[repr(C)]
#[derive(Eq, PartialEq, Hash, Copy, Clone)]
pub struct NeonKey {
    overlay: bool,
}

fn specialize_neon(descriptor: &mut RenderPipelineDescriptor, key: NeonKey) {
    if key.overlay
        && let Some(fragment) = descriptor.fragment.as_mut()
    {
        fragment.shader_defs.push("OVERLAY".into());
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(NeonKey)]
pub struct AccentMaterial {
    #[uniform(0)]
    pub neon: NeonUniform,
    /// Drawn over the world (`NeonKey`).
    pub overlay: bool,
}

impl From<&AccentMaterial> for NeonKey {
    fn from(m: &AccentMaterial) -> Self {
        NeonKey { overlay: m.overlay }
    }
}

impl Material for AccentMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    /// Deferred while reflections are on (`DefaultOpaqueRendererMethod`, src/reflections.rs),
    /// so they show in them; overlays always forward (they set their own depth).
    fn opaque_render_method(&self) -> OpaqueRendererMethod {
        if self.overlay {
            OpaqueRendererMethod::Forward
        } else {
            OpaqueRendererMethod::Auto
        }
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        specialize_neon(descriptor, key.bind_group_data);
        Ok(())
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(NeonKey)]
pub struct GlowMaterial {
    #[uniform(0)]
    pub neon: NeonUniform,
    /// Drawn over the world (`NeonKey`).
    pub overlay: bool,
}

impl From<&GlowMaterial> for NeonKey {
    fn from(m: &GlowMaterial) -> Self {
        NeonKey { overlay: m.overlay }
    }
}

impl Material for GlowMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        specialize_neon(descriptor, key.bind_group_data);
        descriptor.primitive.cull_mode = None;
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent::OVER,
                });
            }
        }
        Ok(())
    }
}

/// Shared animation state of every neon surface.
#[derive(Resource)]
pub struct NeonGlobals {
    pub time: f32,
    /// (origin, start time (negative: unused), colour).
    pub pulses: [(Vec3, f32, Vec3); PULSES],
    next: usize,
    /// Half the arena's length (m): the waves run from its ends.
    pub half_length: f32,
    /// How bright the arena's own lights are (1; less in a blackout).
    pub dim: f32,
    /// Shots lighting up those around them (xyz, w strength): set every frame by
    /// src/actors.rs (`shot_lights`).
    pub shots: [Vec4; SHOT_LIGHTS],
    /// How dark each quarter of the arena is (0..1): blackout breakers' (eased).
    pub zones: Vec4,
}

impl Default for NeonGlobals {
    fn default() -> Self {
        NeonGlobals {
            time: 0.0,
            pulses: [(Vec3::ZERO, -1.0, Vec3::ZERO); PULSES],
            next: 0,
            half_length: 100.0,
            dim: 1.0,
            shots: [Vec4::ZERO; SHOT_LIGHTS],
            zones: Vec4::ZERO,
        }
    }
}

impl NeonGlobals {
    /// A ripple of `color` running out from `origin` over the accents.
    pub fn pulse(&mut self, origin: Vec3, color: Vec3) {
        self.pulses[self.next] = (origin, self.time, color);
        self.next = (self.next + 1) % PULSES;
    }
}

/// Materials everything shares.
#[derive(Resource, Clone)]
pub struct Palette {
    pub body: Handle<StandardMaterial>,
    pub suit: Handle<StandardMaterial>,
    /// The avatars' own dark surfaces: a touch greyer than `suit`, so they don't read as
    /// pitch black.
    pub avatar: Handle<StandardMaterial>,
    /// Lighter surfaces of the avatars (muzzles, patches).
    pub fur: Handle<StandardMaterial>,
    /// The whites of the avatars' eyes (a little self-lit, so they read in the dark).
    pub sclera: Handle<StandardMaterial>,
    pub arena_accent: Handle<AccentMaterial>,
    pub arena_glow: Handle<GlowMaterial>,
    /// The border walls around the arena: a fixed colour, turned slowly through the rainbow
    /// (`animate_border`).
    pub border_accent: Handle<AccentMaterial>,
    pub border_glow: Handle<GlowMaterial>,
    /// Fixed colours: red, blue, green, white (neutral).
    pub accent: [Handle<AccentMaterial>; 4],
    pub glow: [Handle<GlowMaterial>; 4],
    /// Brighter, for bolts.
    pub bolt: [Handle<AccentMaterial>; 4],
    /// Avatars: eyes (bright) and flowing stripes.
    pub eyes: [Handle<AccentMaterial>; 4],
    pub flow: [Handle<AccentMaterial>; 4],
    /// The models' plain surfaces (tools/models/README.md, Materials): painted armour plate
    /// and team paint by colour, then bare metal, rubber, dark glass, heat-stained metal and
    /// polymer.
    pub armor: [Handle<StandardMaterial>; 4],
    pub paint: [Handle<StandardMaterial>; 4],
    pub metal: Handle<StandardMaterial>,
    pub rubber: Handle<StandardMaterial>,
    pub glass: Handle<StandardMaterial>,
    pub heat: Handle<StandardMaterial>,
    pub polymer: Handle<StandardMaterial>,
}

impl Palette {
    /// The plain surface a model's material `name` stands for, if it is one of them.
    pub fn surface(&self, name: &str, team: Option<Team>) -> Option<Handle<StandardMaterial>> {
        let i = team.map_or(3, |t| t.index());
        Some(match name {
            "armor" => self.armor[i].clone(),
            "paint" => self.paint[i].clone(),
            "metal" => self.metal.clone(),
            "rubber" => self.rubber.clone(),
            "glass" => self.glass.clone(),
            "heat" => self.heat.clone(),
            "polymer" => self.polymer.clone(),
            _ => return None,
        })
    }

    pub fn accent_for(&self, team: Option<Team>) -> Handle<AccentMaterial> {
        self.accent[team.map_or(3, |t| t.index())].clone()
    }

    pub fn glow_for(&self, team: Option<Team>) -> Handle<GlowMaterial> {
        self.glow[team.map_or(3, |t| t.index())].clone()
    }

    pub fn bolt_for(&self, team: Option<Team>) -> Handle<AccentMaterial> {
        self.bolt[team.map_or(3, |t| t.index())].clone()
    }

    pub fn eyes_for(&self, team: Option<Team>) -> Handle<AccentMaterial> {
        self.eyes[team.map_or(3, |t| t.index())].clone()
    }

    pub fn flow_for(&self, team: Option<Team>) -> Handle<AccentMaterial> {
        self.flow[team.map_or(3, |t| t.index())].clone()
    }
}

/// A soft particle (smoke, dust, fire, flashes) on a quad turned to the viewer: a puff or a
/// glow drawn in the shader, no texture. Smoke and dust are alpha blended; fire and flashes
/// added over the world. Shader: assets/shaders/particle.wgsl (GL: shaders/gl/particle.glsl).
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(ParticleKey)]
pub struct ParticleMaterial {
    #[uniform(0)]
    pub particle: ParticleUniform,
    pub additive: bool,
}

#[derive(ShaderType, Clone, Copy, Debug)]
pub struct ParticleUniform {
    /// rgb: colour (HDR), a: opacity.
    pub color: Vec4,
    /// x: the look (`PARTICLE_PUFF` or `PARTICLE_GLOW`), y: the noise's seed.
    pub params: Vec4,
}

/// A billowy puff with a ragged edge (smoke, dust).
pub const PARTICLE_PUFF: f32 = 0.0;
/// A glow with a hot core (fire, flashes).
pub const PARTICLE_GLOW: f32 = 1.0;

impl ParticleMaterial {
    pub fn puff(color: Vec3, opacity: f32, seed: f32) -> Self {
        ParticleMaterial {
            particle: ParticleUniform {
                color: color.extend(opacity),
                params: Vec4::new(PARTICLE_PUFF, seed, 0.0, 0.0),
            },
            additive: false,
        }
    }

    pub fn glow(color: Vec3, opacity: f32) -> Self {
        ParticleMaterial {
            particle: ParticleUniform {
                color: color.extend(opacity),
                params: Vec4::new(PARTICLE_GLOW, 0.0, 0.0, 0.0),
            },
            additive: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParticleKey {
    additive: bool,
}

impl From<&ParticleMaterial> for ParticleKey {
    fn from(m: &ParticleMaterial) -> Self {
        ParticleKey {
            additive: m.additive,
        }
    }
}

impl Material for ParticleMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/particle.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        if self.additive {
            AlphaMode::Add
        } else {
            AlphaMode::Blend
        }
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                // (Additive: the shader gives its colour times its alpha.)
                target.blend = Some(if key.bind_group_data.additive {
                    BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent::OVER,
                    }
                } else {
                    BlendState::ALPHA_BLENDING
                });
            }
        }
        Ok(())
    }
}

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MaterialPlugin::<AccentMaterial>::default(),
            MaterialPlugin::<GlowMaterial>::default(),
            MaterialPlugin::<ParticleMaterial>::default(),
        ))
        .init_resource::<NeonGlobals>()
        .add_systems(PreStartup, make_palette)
        .add_systems(Update, animate_border)
        .add_systems(PostUpdate, animate);
    }
}

fn make_palette(
    mut commands: Commands,
    mut std: ResMut<Assets<StandardMaterial>>,
    mut accents: ResMut<Assets<AccentMaterial>>,
    mut glows: ResMut<Assets<GlowMaterial>>,
) {
    let body = std.add(StandardMaterial {
        base_color: Color::linear_rgb(0.012, 0.012, 0.016),
        perceptual_roughness: 0.45,
        reflectance: 0.35,
        ..default()
    });
    let suit = std.add(StandardMaterial {
        base_color: Color::linear_rgb(0.03, 0.03, 0.035),
        perceptual_roughness: 0.6,
        ..default()
    });
    let avatar = std.add(StandardMaterial {
        base_color: Color::linear_rgb(0.06, 0.06, 0.068),
        perceptual_roughness: 0.6,
        ..default()
    });
    let fur = std.add(StandardMaterial {
        base_color: Color::linear_rgb(0.14, 0.14, 0.16),
        perceptual_roughness: 0.8,
        ..default()
    });
    let sclera = std.add(StandardMaterial {
        base_color: Color::linear_rgb(0.62, 0.62, 0.66),
        emissive: LinearRgba::new(0.05, 0.05, 0.06, 1.0),
        perceptual_roughness: 0.35,
        reflectance: 0.5,
        ..default()
    });
    let fixed = |c: Vec3, i: f32| NeonUniform::fixed(c, i);
    let colors = [RED, BLUE, GREEN, WHITE];
    let plain = |std: &mut Assets<StandardMaterial>, c: Vec3, rough: f32, metal: f32| {
        std.add(StandardMaterial {
            base_color: Color::linear_rgb(c.x, c.y, c.z),
            perceptual_roughness: rough,
            metallic: metal,
            reflectance: 0.45,
            ..default()
        })
    };
    // Armour plate: a mid grey with a breath of the team's colour; paint: the colour itself,
    // dulled (the neutral ones a pale grey).
    let grey = Vec3::new(0.15, 0.155, 0.165);
    let armor = colors.map(|c| {
        let tint = if c == WHITE {
            grey
        } else {
            grey.lerp(c * 0.3, 0.22)
        };
        plain(&mut std, tint, 0.5, 0.35)
    });
    let paint = colors.map(|c| {
        let tint = if c == WHITE {
            Vec3::splat(0.3)
        } else {
            c * 0.32 + Vec3::splat(0.012)
        };
        plain(&mut std, tint, 0.55, 0.15)
    });
    let metal = plain(&mut std, Vec3::new(0.3, 0.31, 0.33), 0.32, 0.9);
    let rubber = plain(&mut std, Vec3::splat(0.012), 0.95, 0.0);
    let polymer = plain(&mut std, Vec3::new(0.035, 0.036, 0.04), 0.75, 0.0);
    let glass = std.add(StandardMaterial {
        base_color: Color::linear_rgb(0.02, 0.05, 0.08),
        perceptual_roughness: 0.06,
        metallic: 0.9,
        reflectance: 0.8,
        ..default()
    });
    let heat = std.add(StandardMaterial {
        base_color: Color::linear_rgb(0.1, 0.06, 0.04),
        emissive: LinearRgba::new(0.25, 0.06, 0.01, 1.0),
        perceptual_roughness: 0.4,
        metallic: 0.8,
        ..default()
    });
    commands.insert_resource(Palette {
        armor,
        paint,
        metal,
        rubber,
        glass,
        heat,
        polymer,
        body,
        suit,
        avatar,
        fur,
        sclera,
        arena_accent: accents.add(AccentMaterial {
            overlay: false,
            neon: NeonUniform::team_zoned(3.0).dimmable(),
        }),
        arena_glow: glows.add(GlowMaterial {
            overlay: false,
            neon: NeonUniform::team_zoned(2.0).dimmable(),
        }),
        border_accent: accents.add(AccentMaterial {
            overlay: false,
            neon: fixed(RED, BORDER_ACCENT).dimmable(),
        }),
        border_glow: glows.add(GlowMaterial {
            overlay: false,
            neon: fixed(RED, BORDER_GLOW).dimmable(),
        }),
        accent: colors.map(|c| {
            accents.add(AccentMaterial {
                overlay: false,
                neon: fixed(c, 3.0),
            })
        }),
        glow: colors.map(|c| {
            glows.add(GlowMaterial {
                overlay: false,
                neon: fixed(c, 2.0),
            })
        }),
        bolt: colors.map(|c| {
            accents.add(AccentMaterial {
                overlay: false,
                neon: fixed(c, 9.0),
            })
        }),
        eyes: colors.map(|c| {
            accents.add(AccentMaterial {
                overlay: false,
                neon: fixed(c, 4.0),
            })
        }),
        flow: colors.map(|c| {
            accents.add(AccentMaterial {
                overlay: false,
                neon: NeonUniform::flow(c, 3.0, 0.8),
            })
        }),
    });
}

/// The border walls' resting intensities (accents, halos), and the seconds they take to go
/// once round the rainbow.
const BORDER_ACCENT: f32 = 2.6;
const BORDER_GLOW: f32 = 1.6;
const RAINBOW_PERIOD: f32 = 45.0;

/// Turns the border walls slowly through the rainbow.
fn animate_border(
    time: Res<Time>,
    palette: Res<Palette>,
    mut accents: ResMut<Assets<AccentMaterial>>,
    mut glows: ResMut<Assets<GlowMaterial>>,
) {
    let hue = (time.elapsed_secs_wrapped() / RAINBOW_PERIOD).fract() * 360.0;
    let c = Color::hsv(hue, 0.9, 1.0).to_linear();
    let rgb = Vec3::new(c.red, c.green, c.blue);
    if let Some(mut m) = accents.get_mut(&palette.border_accent) {
        m.neon.color = rgb.extend(BORDER_ACCENT * 0.65);
    }
    if let Some(mut m) = glows.get_mut(&palette.border_glow) {
        m.neon.color = rgb.extend(BORDER_GLOW * 0.5);
    }
}

fn animate(
    time: Res<Time>,
    game: Option<Res<crate::game::Game>>,
    mut globals: ResMut<NeonGlobals>,
    mut accents: ResMut<Assets<AccentMaterial>>,
    mut glows: ResMut<Assets<GlowMaterial>>,
) {
    globals.time = time.elapsed_secs_wrapped();
    let _ = game;
    for (_, m) in accents.iter_mut() {
        m.neon.apply(&globals, false);
    }
    for (_, m) in glows.iter_mut() {
        m.neon.apply(&globals, true);
    }
}
