// Structures and vehicles (src/hull.rs): the standard material with a procedural plating
// pattern: triplanar panels with darker seams, each panel a slightly different tone, grime
// gathering low down, small rivets. Positions are taken in the object's frame (`frame`: world
// to object; the identity for the static structures) so the panels ride along with vehicles.

#import bevy_pbr::{
    pbr_functions::alpha_discard,
    pbr_fragment::pbr_input_from_standard_material,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

struct Hull {
    frame: mat4x4<f32>,
    // x: panel size (m), y: seam darkness, z: grime, w: tone variation.
    params: vec4<f32>,
    // rgb: seam light colour (faint team glow in the seams), a: its strength.
    seam_light: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> hull: Hull;

fn hash3(p: vec3<f32>) -> f32 {
    let q = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let r = q + dot(q, q.yxz + 33.33);
    return fract((r.x + r.y) * r.z);
}

fn noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = mix(mix(hash3(i), hash3(i + vec3(1.0, 0.0, 0.0)), u.x),
                mix(hash3(i + vec3(0.0, 1.0, 0.0)), hash3(i + vec3(1.0, 1.0, 0.0)), u.x), u.y);
    let b = mix(mix(hash3(i + vec3(0.0, 0.0, 1.0)), hash3(i + vec3(1.0, 0.0, 1.0)), u.x),
                mix(hash3(i + vec3(0.0, 1.0, 1.0)), hash3(i + vec3(1.0, 1.0, 1.0)), u.x), u.y);
    return mix(a, b, u.z);
}

// Panels on a plane: (seam line 0..1, the panel's own tone 0..1, rivet).
fn panels(uv: vec2<f32>, size: f32) -> vec3<f32> {
    // Panels in rows offset every other row, twice as wide as high.
    var g = uv / vec2<f32>(size * 2.0, size);
    let row = floor(g.y);
    g.x += 0.5 * (row - 2.0 * floor(row * 0.5));
    let cell = floor(g);
    let f = fract(g);
    let edge = min(min(f.x, 1.0 - f.x) * 2.0, min(f.y, 1.0 - f.y));
    let w = max(fwidth(g.y), 1e-4) * 1.5;
    let seam = 1.0 - smoothstep(0.0, w + 0.02, edge);
    let tone = hash3(vec3<f32>(cell, 7.0));
    // Rivets near the corners.
    let c = abs(f - vec2<f32>(0.5)) - vec2<f32>(0.47, 0.42);
    let rivet = 1.0 - smoothstep(0.0, 0.015, length(max(c, vec2<f32>(0.0))));
    return vec3<f32>(seam, tone, rivet * step(0.0, min(c.x, c.y) + 0.03));
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    let p = (hull.frame * vec4<f32>(in.world_position.xyz, 1.0)).xyz;
    let n = normalize((hull.frame * vec4<f32>(in.world_normal, 0.0)).xyz);
    let an = abs(n);
    let size = hull.params.x;
    // Triplanar: the plane the face mostly lies in.
    var pa: vec3<f32>;
    if an.y > an.x && an.y > an.z {
        pa = panels(p.xz, size);
    } else if an.x > an.z {
        pa = panels(p.zy, size);
    } else {
        pa = panels(p.xy, size);
    }
    let tone = 1.0 + (pa.y - 0.5) * hull.params.w;
    let grime = mix(1.0, 0.7 + 0.3 * noise3(p * 0.7), hull.params.z * (1.0 - smoothstep(0.0, 3.0, p.y)) );
    let streaks = 0.9 + 0.1 * noise3(vec3<f32>(p.x * 2.0, p.y * 0.15, p.z * 2.0));
    let shade = tone * grime * streaks * (1.0 - pa.x * hull.params.y) * (1.0 + pa.z * 0.6);
    pbr_input.material.base_color = vec4<f32>(pbr_input.material.base_color.rgb * shade, pbr_input.material.base_color.a);
    pbr_input.material.perceptual_roughness = clamp(pbr_input.material.perceptual_roughness + (pa.y - 0.5) * 0.2 + pa.x * 0.2, 0.05, 1.0);
    pbr_input.material.emissive = vec4<f32>(pbr_input.material.emissive.rgb + hull.seam_light.rgb * hull.seam_light.a * pa.x, 1.0);

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif
    return out;
}
