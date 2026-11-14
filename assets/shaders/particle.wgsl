// Soft particles (src/materials.rs `ParticleMaterial`): smoke, dust, fire and flashes on quads
// turned to the viewer, drawn here without a texture. The GL port for the headset renderer is
// assets/shaders/gl/particle.glsl: keep the two in step.
//
// A puff (params.x 0): a round billow whose edge is ragged with noise (seeded by params.y, and
// turned by the quad's roll), alpha blended. A glow (params.x 1): a soft disc with a hot core,
// its colour times its alpha (added over the world). A streak (params.x 2): a glow drawn out
// along the quad's x, hot and wide at its +x end and thinning to nothing at the other (a
// tracer's light, a spark, a muzzle flash's spike). A ring (params.x 3): a glowing band at
// the quad's edge, params.z of its radius wide (a blast's shockwave).
//
// With a depth prepass (the flat screen's camera) they're soft particles: fading out where the
// quad nears what's behind it, so they don't end in a hard line where they cut into the ground
// or a wall. (The GL port has no depth to read: there they're kept small instead.) Not those
// with params.w set: what lies flat on the ground (a scorch mark, a shockwave's ring) would
// fade away altogether.

#import bevy_pbr::forward_io::VertexOutput
#ifdef DEPTH_PREPASS
#import bevy_pbr::prepass_utils
#import bevy_pbr::view_transformations::depth_ndc_to_view_z
#endif

struct Particle {
    // rgb: colour (HDR), a: opacity.
    color: vec4<f32>,
    // x: the look (0 puff, 1 glow, 2 streak, 3 ring), y: the noise's seed, z: a ring's width
    // (of its radius), w: 1 for no fading into what's behind.
    params: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> particle: Particle;

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

// Smooth value noise, about 0..1.
fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// How far (m) before what's behind it a particle has faded out.
const SOFT_DEPTH: f32 = 0.6;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.uv * 2.0 - 1.0;
    let d = length(p);
    var c = particle.color;
#ifdef DEPTH_PREPASS
    // (The first sample's depth: not shading every sample.)
    let scene = depth_ndc_to_view_z(prepass_utils::prepass_depth(in.position, 0u));
    let here = depth_ndc_to_view_z(in.position.z);
    if (particle.params.w < 0.5) {
        c.a *= clamp((here - scene) / SOFT_DEPTH, 0.0, 1.0);
    }
#endif
    if (particle.params.x > 2.5) {
        // The ring: a band just inside the quad's edge, brightest along its middle.
        let w = max(particle.params.z, 0.02);
        let a = pow(clamp(1.0 - abs(d - (1.0 - w)) / w, 0.0, 1.0), 1.6);
        let rgb = c.rgb * (1.0 + 1.5 * a * a);
        return vec4<f32>(rgb * a * c.a, a * c.a);
    }
    if (particle.params.x > 1.5) {
        // The streak: its head at +x (rounded off), its tail thinning out behind.
        let t = clamp(p.x * 0.5 + 0.5, 0.0, 1.0);
        let across = pow(clamp(1.0 - abs(p.y) / mix(0.3, 1.0, t), 0.0, 1.0), 2.0);
        let a = across * pow(t, 1.4) * clamp((1.0 - t) / 0.12, 0.0, 1.0);
        let core = pow(a, 3.0);
        let rgb = c.rgb * (1.0 + 2.5 * core) + vec3<f32>(core * 1.5);
        return vec4<f32>(rgb * a * c.a, a * c.a);
    }
    if (particle.params.x > 0.5) {
        // The glow: soft to the edge, white hot in the middle.
        let a = pow(clamp(1.0 - d, 0.0, 1.0), 1.8);
        let core = pow(a, 3.0);
        let rgb = c.rgb * (1.0 + 2.5 * core) + vec3<f32>(core * 1.5);
        return vec4<f32>(rgb * a * c.a, a * c.a);
    }
    // The puff: lumpy, its edge eaten away by the noise; a little lighter towards the middle.
    let s = particle.params.y;
    let n = vnoise(p * 2.3 + vec2<f32>(s, s * 1.7)) * 0.65 + vnoise(p * 5.1 - vec2<f32>(s * 2.3, s)) * 0.35;
    let body = clamp(1.0 - d * d, 0.0, 1.0);
    let a = clamp(body * (0.55 + 0.9 * n) - 0.12, 0.0, 1.0);
    let rgb = c.rgb * (0.75 + 0.5 * n) * (1.0 + 0.4 * body);
    return vec4<f32>(rgb, a * c.a);
}
