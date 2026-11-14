// The continents' land (src/terrain.rs): the standard material (dark ground, vertex coloured by
// the theme) with glowing lines drawn over it: contour lines every few metres of height and a
// faint grid on the map, in the theme's colours, fading with distance. Contours pulse slowly.
// Roads (ps_sim::roads): dark paving edged with the contour colour, from the distance to the
// nearest road's middle in the vertex colour's alpha (0.5 on it, 1 at ROAD_NEAR or further).

#import bevy_pbr::{
    pbr_functions::alpha_discard,
    pbr_fragment::pbr_input_from_standard_material,
    mesh_view_bindings::view,
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

struct TerrainLines {
    // rgb: contour colour, a: intensity.
    contour: vec4<f32>,
    // rgb: grid colour, a: intensity.
    grid: vec4<f32>,
    // x: contour spacing (m), y: grid spacing (m), z: time (s), w: fade distance (m).
    params: vec4<f32>,
    // x: sea level glow band height, yzw: shore colour.
    shore: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> lines: TerrainLines;

// ps_sim::roads::{NEAR_MAX, HALF_WIDTH}.
const ROAD_NEAR: f32 = 31.875;
const ROAD_HALF: f32 = 7.0;

fn hash2(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(0.1031, 0.1030));
    let r = q + dot(q, q.yx + 33.33);
    return fract((r.x + r.y) * r.x);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash2(i), hash2(i + vec2(1.0, 0.0)), u.x),
               mix(hash2(i + vec2(0.0, 1.0)), hash2(i + vec2(1.0, 1.0)), u.x), u.y);
}

// 1 on a line of `f`'s integer values, anti-aliased, `w` pixels wide.
fn line_at(f: f32, w: f32) -> f32 {
    let d = abs(fract(f - 0.5) - 0.5) / max(fwidth(f), 1e-5);
    return 1.0 - clamp(d / w, 0.0, 1.0);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    let p = in.world_position.xyz;
    let dist = length(view.world_position - p);
    // Detail: fine grain and blotches up close, strata on the slopes, fading with distance.
    let near = 1.0 - smoothstep(30.0, 220.0, dist);
    let grain = vnoise(p.xz * 1.7) * 0.5 + vnoise(p.xz * 6.0) * 0.5;
    let blotch = vnoise(p.xz * 0.08) * 0.6 + vnoise(p.xz * 0.31) * 0.4;
    let slope = 1.0 - abs(normalize(in.world_normal).y);
    let strata = 0.5 + 0.5 * sin(p.y * 2.3 + vnoise(p.xz * 0.2) * 4.0);
    let detail = mix(1.0, 0.75 + 0.5 * grain, near)
        * (0.8 + 0.4 * blotch)
        * mix(1.0, 0.7 + 0.5 * strata, smoothstep(0.35, 0.8, slope));
    var base = pbr_input.material.base_color.rgb * detail;
    let fade = 1.0 - smoothstep(lines.params.w * 0.35, lines.params.w, dist);
#ifdef VERTEX_COLORS
    let road_d = (in.color.a - 0.5) * 2.0 * ROAD_NEAR;
#else
    let road_d = ROAD_NEAR;
#endif
    let aa = max(fwidth(road_d), 0.05);
    let on_road = 1.0 - smoothstep(ROAD_HALF - aa, ROAD_HALF + aa, road_d);
    // The paving: dark, smooth, faintly speckled.
    let paving = vec3<f32>(0.012, 0.012, 0.016) * (0.85 + 0.3 * vnoise(p.xz * 3.0));
    base = mix(base, paving, on_road);
    pbr_input.material.base_color = vec4<f32>(base, 1.0);
    pbr_input.material.perceptual_roughness = mix(pbr_input.material.perceptual_roughness, 0.45, on_road);
    // Its edges: a neon line either side, a faint glow inside.
    let edge = 1.0 - smoothstep(0.12, 0.12 + aa * 1.5, abs(road_d - (ROAD_HALF - 0.7)));
    let road_fade = 1.0 - smoothstep(lines.params.w * 0.6, lines.params.w * 1.6, dist);
    let road_glow = lines.contour.rgb * (edge * 2.2 + on_road * 0.06) * road_fade;
    // Contours: every `params.x` metres up, a pulse running up the hills.
    let c = line_at(p.y / lines.params.x, 1.2) * (1.0 - on_road);
    let pulse = 0.6 + 0.4 * sin(p.y * 0.08 - lines.params.z * 0.7);
    // Grid: faint, on the map's squares.
    let g = max(line_at(p.x / lines.params.y, 1.0), line_at(p.z / lines.params.y, 1.0))
        * (1.0 - on_road);
    // The shoreline glows.
    let shore = 1.0 - smoothstep(0.0, lines.shore.x, abs(p.y - 0.1));
    let glow = lines.contour.rgb * lines.contour.a * c * pulse * fade
        + lines.grid.rgb * lines.grid.a * g * fade
        + lines.shore.yzw * shore * 2.0
        + road_glow;
    pbr_input.material.emissive = vec4<f32>(pbr_input.material.emissive.rgb + glow, 1.0);

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif
    return out;
}
