#version 460 core

out vec4 fragColor;

uniform float u_time;
uniform vec2 u_resolution;

float WEIGHT;

float line(vec2 p, vec2 p0, vec2 p1, float w) {
    vec2 d = p1 - p0;
    // Nota: en el original el clamp tenía valores extraños, corregí a rango [0.0, 1.0]
    float t = clamp(dot(d, p - p0) / dot(d, d), 0.0, 1.0);
    vec2 proj = p0 + d * t;
    float dist = length(p - proj);
    dist = 1.0 / dist * WEIGHT * w;
    return min(dist * dist, 1.0);
}

vec3 hsv(float h, float s, float v) {
    vec4 t = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(vec3(h) + t.xyz) * 6.0 - vec3(t.w));
    return v * mix(vec3(t.x), clamp(p - vec3(t.x), 0.0, 1.0), s);
}

void main() {
    WEIGHT = 57.0 / u_resolution.x;

    vec2 uv = gl_FragCoord.xy / u_resolution.xy;
    uv = uv * 2.0 - 1.0;
    uv.x *= u_resolution.x / u_resolution.y;

    float line_width = 0.4;
    float timeVal = u_time * 0.31415 + sin(length(uv) + u_time * 0.2) / length(uv) * 0.1;

    vec3 c = vec3(0.0);

    for (float i = 8.0; i < 24.0; i += 2.0) {
        float f = line(uv, vec2(cos(timeVal * i) / 2.0, sin(timeVal * i) / 2.0), vec2(sin(timeVal * i) / 2.0, -cos(timeVal * i) / 2.0), 0.5);
        c += hsv(i / 24.0, 1.0, 1.0) * f;
    }

    fragColor = vec4(c, 1.0);
}
