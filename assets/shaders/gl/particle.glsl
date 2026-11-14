// Soft particles for the OpenGL headset renderer (src/glvr): the GL port of
// assets/shaders/particle.wgsl (keep the two in step). Compiled with VERTEX or FRAGMENT
// defined; both eyes in one pass (GL_OVR_multiview2). Smoke is drawn blended (SRC_ALPHA,
// ONE_MINUS_SRC_ALPHA), fire and flashes added (ONE, ONE): src/glvr/render.rs sets which.

uniform mat4 u_view_proj[2];
uniform mat4 u_model;
// rgb: colour (HDR), a: opacity.
uniform vec4 u_colour;
// x: the look (0 puff, 1 glow), y: the noise's seed.
uniform vec4 u_params;

#ifdef VERTEX
layout(num_views = NUM_VIEWS) in;
layout(location = 0) in vec3 a_position;
layout(location = 2) in vec2 a_uv;
out vec2 v_uv;

void main() {
    v_uv = a_uv;
    gl_Position = u_view_proj[VIEW] * (u_model * vec4(a_position, 1.0));
}
#endif

#ifdef FRAGMENT
in vec2 v_uv;
layout(location = 0) out vec4 o_colour;

float hash2(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float vnoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    float a = hash2(i);
    float b = hash2(i + vec2(1.0, 0.0));
    float c = hash2(i + vec2(0.0, 1.0));
    float d = hash2(i + vec2(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// As neon.glsl's: HDR to the display's range, the hue kept.
vec3 tone(vec3 c) {
    c = max(c, vec3(0.0));
    float m = max(max(c.r, c.g), c.b);
    if (m <= 1e-5) {
        return vec3(0.0);
    }
    float mapped = 1.0 - exp(-m);
    vec3 hue = c * (mapped / m);
    float white = smoothstep(2.5, 14.0, m) * 0.55;
    return mix(hue, vec3(mapped), white);
}

void main() {
    vec2 p = v_uv * 2.0 - 1.0;
    float d = length(p);
    vec4 c = u_colour;
    if (u_params.x > 0.5) {
        float a = pow(clamp(1.0 - d, 0.0, 1.0), 1.8);
        float core = pow(a, 3.0);
        vec3 rgb = c.rgb * (1.0 + 2.5 * core) + vec3(core * 1.5);
        o_colour = vec4(tone(rgb * a * c.a), a * c.a);
        return;
    }
    float s = u_params.y;
    float n = vnoise(p * 2.3 + vec2(s, s * 1.7)) * 0.65 + vnoise(p * 5.1 - vec2(s * 2.3, s)) * 0.35;
    float body = clamp(1.0 - d * d, 0.0, 1.0);
    float a = clamp(body * (0.55 + 0.9 * n) - 0.12, 0.0, 1.0);
    vec3 rgb = c.rgb * (0.75 + 0.5 * n) * (1.0 + 0.4 * body);
    o_colour = vec4(tone(rgb), a * c.a);
}
#endif
