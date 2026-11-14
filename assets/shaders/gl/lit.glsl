// Plain lit surfaces for the OpenGL headset renderer (src/glvr): the StandardMaterial meshes
// (the arena's dark body, the suits). The base colour lit by the ambient light and the one
// directional light, with a faint highlight, tone mapped as `--vr`'s; not Bevy's PBR.
// Compiled with VERTEX or FRAGMENT defined; both eyes in one pass (GL_OVR_multiview2).

uniform mat4 u_view_proj[2];
uniform vec3 u_eye[2];
uniform mat4 u_model;
uniform mat3 u_normal;
// Linear RGBA.
uniform vec4 u_base;
// Towards the light (world), its colour times its strength, the ambient light.
uniform vec3 u_light_dir;
uniform vec3 u_light_colour;
uniform vec3 u_ambient;
// Point lights (the bolts'): xyz position, w range; colour times strength at 1 m; how many.
uniform vec4 u_point_pos[16];
uniform vec3 u_point_colour[16];
uniform int u_point_count;
// The surface's detail (src/glvr/extract.rs LitItem): 0 plain (u_detail[0] metal, roughness,
// [1] what it emits), 1 hull plating (hull.wgsl:
// u_frame world to object, u_detail[0] params, [1] seam light), 2 land (terrain.wgsl:
// contour, grid, params, shore; its roads from the vertex colour's alpha).
uniform int u_kind;
uniform mat4 u_frame;
uniform vec4 u_detail[4];

#ifdef VERTEX
layout(num_views = NUM_VIEWS) in;
layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;
// Vertex colours (the land's), alpha 0 where the mesh has none.
layout(location = 3) in vec4 a_colour;
out vec3 v_world;
out vec3 v_normal;
out vec4 v_colour;

void main() {
    vec4 world = u_model * vec4(a_position, 1.0);
    v_world = world.xyz;
    v_colour = a_colour;
    v_normal = u_normal * a_normal;
    gl_Position = u_view_proj[VIEW] * world;
}
#endif

#ifdef FRAGMENT
in vec3 v_world;
in vec3 v_normal;
in vec4 v_colour;
layout(location = 0) out vec4 o_colour;

float hash2(vec2 p) {
    vec2 q = fract(p * vec2(0.1031, 0.1030));
    vec2 r = q + dot(q, q.yx + 33.33);
    return fract((r.x + r.y) * r.x);
}

float vnoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash2(i), hash2(i + vec2(1.0, 0.0)), u.x),
               mix(hash2(i + vec2(0.0, 1.0)), hash2(i + vec2(1.0, 1.0)), u.x), u.y);
}

float hash3(vec3 p) {
    vec3 q = fract(p * vec3(0.1031, 0.1030, 0.0973));
    vec3 r = q + dot(q, q.yxz + 33.33);
    return fract((r.x + r.y) * r.z);
}

float noise3(vec3 p) {
    vec3 i = floor(p);
    vec3 f = fract(p);
    vec3 u = f * f * (3.0 - 2.0 * f);
    float a = mix(mix(hash3(i), hash3(i + vec3(1, 0, 0)), u.x),
                  mix(hash3(i + vec3(0, 1, 0)), hash3(i + vec3(1, 1, 0)), u.x), u.y);
    float b = mix(mix(hash3(i + vec3(0, 0, 1)), hash3(i + vec3(1, 0, 1)), u.x),
                  mix(hash3(i + vec3(0, 1, 1)), hash3(i + vec3(1, 1, 1)), u.x), u.y);
    return mix(a, b, u.z);
}

// Hull plating on a plane: (seam 0..1, the panel's tone 0..1, rivet).
vec3 panels(vec2 uv, float size) {
    vec2 g = uv / vec2(size * 2.0, size);
    float row = floor(g.y);
    g.x += 0.5 * (row - 2.0 * floor(row * 0.5));
    vec2 cell = floor(g);
    vec2 f = fract(g);
    float edge = min(min(f.x, 1.0 - f.x) * 2.0, min(f.y, 1.0 - f.y));
    float w = max(fwidth(g.y), 1e-4) * 1.5;
    float seam = 1.0 - smoothstep(0.0, w + 0.02, edge);
    float tone = hash3(vec3(cell, 7.0));
    vec2 c = abs(f - vec2(0.5)) - vec2(0.47, 0.42);
    float rivet = 1.0 - smoothstep(0.0, 0.015, length(max(c, vec2(0.0))));
    return vec3(seam, tone, rivet * step(0.0, min(c.x, c.y) + 0.03));
}

// 1 on a line of `f`'s whole values, `w` pixels wide.
float line_at(float f, float w) {
    float d = abs(fract(f - 0.5) - 0.5) / max(fwidth(f), 1e-5);
    return 1.0 - clamp(d / w, 0.0, 1.0);
}

// The eye cameras' tone mapping in `--vr` (ACES fitted, Narkowicz): its toe keeps the dark
// surfaces near black.
vec3 aces(vec3 x) {
    return clamp(x * (2.51 * x + 0.03) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}

void main() {
    vec3 n = v_normal;
    if (dot(n, n) < 1e-8) {
        n = cross(dFdx(v_world), dFdy(v_world));
    }
    n = normalize(n);
    if (!gl_FrontFacing) {
        n = -n;
    }
    vec3 base = v_colour.a > 0.0 ? u_base.rgb * v_colour.rgb : u_base.rgb;
    vec3 glow = vec3(0.0);
    float shine = 1.0;
    if (u_kind == 0) {
        // Plain surfaces: metal and gloss by their highlight alone, and their own light.
        shine = mix(1.0, 3.0, u_detail[0].x) * mix(1.5, 0.3, u_detail[0].y);
        glow = u_detail[1].rgb;
    } else if (u_kind == 1) {
        // Plating (hull.wgsl).
        vec3 p = (u_frame * vec4(v_world, 1.0)).xyz;
        vec3 on = abs(normalize(mat3(u_frame) * n));
        float size = u_detail[0].x;
        vec3 pa = (on.y > on.x && on.y > on.z) ? panels(p.xz, size)
            : (on.x > on.z ? panels(p.zy, size) : panels(p.xy, size));
        float tone = 1.0 + (pa.y - 0.5) * u_detail[0].w;
        float grime = mix(1.0, 0.7 + 0.3 * noise3(p * 0.7),
            u_detail[0].z * (1.0 - smoothstep(0.0, 3.0, p.y)));
        float streaks = 0.9 + 0.1 * noise3(vec3(p.x * 2.0, p.y * 0.15, p.z * 2.0));
        base *= tone * grime * streaks * (1.0 - pa.x * u_detail[0].y) * (1.0 + pa.z * 0.6);
        shine = 1.0 - pa.x * 0.7 + pa.z;
        glow = u_detail[1].rgb * u_detail[1].a * pa.x;
    } else if (u_kind == 2) {
        // The land (terrain.wgsl): grain, blotches, strata; glowing contours, grid, shore.
        vec3 p = v_world;
        float dist = length(u_eye[VIEW] - p);
        float near = 1.0 - smoothstep(30.0, 220.0, dist);
        float grain = vnoise(p.xz * 1.7) * 0.5 + vnoise(p.xz * 6.0) * 0.5;
        float blotch = vnoise(p.xz * 0.08) * 0.6 + vnoise(p.xz * 0.31) * 0.4;
        float slope = 1.0 - abs(n.y);
        float strata = 0.5 + 0.5 * sin(p.y * 2.3 + vnoise(p.xz * 0.2) * 4.0);
        base *= mix(1.0, 0.75 + 0.5 * grain, near) * (0.8 + 0.4 * blotch)
            * mix(1.0, 0.7 + 0.5 * strata, smoothstep(0.35, 0.8, slope));
        vec4 contour = u_detail[0];
        vec4 grid = u_detail[1];
        vec4 params = u_detail[2];
        vec4 shore = u_detail[3];
        float fade = 1.0 - smoothstep(params.w * 0.35, params.w, dist);
        // Roads: the distance to the nearest one's middle in the colour's alpha (0.5 on it, 1
        // at 31.875 m or further: ps_sim::roads::NEAR_MAX); 7 m wide either side.
        float road_d = (v_colour.a - 0.5) * 2.0 * 31.875;
        float aa = max(fwidth(road_d), 0.05);
        float on_road = 1.0 - smoothstep(7.0 - aa, 7.0 + aa, road_d);
        base = mix(base, vec3(0.012, 0.012, 0.016) * (0.85 + 0.3 * vnoise(p.xz * 3.0)), on_road);
        shine = mix(1.0, 2.0, on_road);
        float edge = 1.0 - smoothstep(0.12, 0.12 + aa * 1.5, abs(road_d - 6.3));
        float road_fade = 1.0 - smoothstep(params.w * 0.6, params.w * 1.6, dist);
        float c = line_at(p.y / params.x, 1.2) * (1.0 - on_road);
        float pulse = 0.6 + 0.4 * sin(p.y * 0.08 - params.z * 0.7);
        float g = max(line_at(p.x / params.y, 1.0), line_at(p.z / params.y, 1.0))
            * (1.0 - on_road);
        float s = 1.0 - smoothstep(0.0, shore.x, abs(p.y - 0.1));
        glow = contour.rgb * contour.a * c * pulse * fade + grid.rgb * grid.a * g * fade
            + shore.yzw * s * 2.0 + contour.rgb * (edge * 2.2 + on_road * 0.06) * road_fade;
    }
    vec3 to_eye = normalize(u_eye[VIEW] - v_world);
    float diffuse = max(dot(n, u_light_dir), 0.0);
    // A dull Blinn highlight, so the black surfaces show their shape.
    vec3 h = normalize(u_light_dir + to_eye);
    float spec = pow(max(dot(n, h), 0.0), 24.0) * 0.04;
    vec3 rgb = base * (u_ambient + u_light_colour * diffuse) + u_light_colour * spec * shine + glow;
    // Point lights: inverse square, windowed to their range as Bevy's; a sharper highlight
    // so the glossy black walls show the bolts passing.
    for (int i = 0; i < u_point_count; i++) {
        vec3 d = u_point_pos[i].xyz - v_world;
        float dist2 = max(dot(d, d), 1e-4);
        float r = u_point_pos[i].w;
        float k = dist2 / (r * r);
        float window = clamp(1.0 - k * k, 0.0, 1.0);
        vec3 l = d * inversesqrt(dist2);
        vec3 c = u_point_colour[i] * window * window / dist2;
        vec3 hp = normalize(l + to_eye);
        rgb += c * (base * max(dot(n, l), 0.0) + pow(max(dot(n, hp), 0.0), 48.0) * 0.35);
    }
    o_colour = vec4(aces(rgb), 1.0);
}
#endif
