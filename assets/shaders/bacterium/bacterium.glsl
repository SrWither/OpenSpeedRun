#version 460 core

out vec4 fragColor;

uniform float u_time;
uniform vec2 u_resolution;

#define SAMPLES 10
#define FOCAL_DISTANCE 4.0
#define FOCAL_RANGE 0.0

mat2 m(float a) {
    float c = cos(a), s = sin(a);
    return mat2(c, -s, s, c);
}

float map(vec3 p) {
    float t = u_time;
    p.xz *= m(t * 0.4);
    p.xy *= m(t * 0.3);
    vec3 q = p * 2.0 + t;
    return length(p + vec3(sin(t * 0.7))) * log(length(p) + 1.0) + sin(q.x + sin(q.z + sin(q.y))) * 0.5 - 1.0;
}

vec3 hslToRgb(vec3 hsl) {
    vec3 rgb = clamp(abs(mod(hsl.x * 6.0 + vec3(0.0, 4.0, 2.0), 6.0) - 3.0) - 1.0, 0.0, 1.0);
    return hsl.z + hsl.y * (rgb - 0.5) * (1.0 - abs(2.0 * hsl.z - 1.0));
}

vec3 getColor(in vec2 fragCoord, in float depth) {
    float t = u_time;
    vec2 p = fragCoord.xy / u_resolution.y - vec2(0.0, 0.5);  // centrar en X
    
    vec3 cl = vec3(0.0);
    float d = depth;

    for (int i = 0; i <= 5; i++) {
        vec3 pos = vec3(0.0, 0.0, 5.0) + normalize(vec3(p, -1.0)) * d;
        float rz = map(pos);
        float f = clamp((rz - map(pos + 0.1)) * 0.5, -0.1, 1.0);

        float hue = mod(t * 0.125 + float(i) / 5.0, 1.0);
        float hueRange = 0.5;
        float hueShift = 0.3;
        hue = mix(0.0, 1.0, smoothstep(0.0, hueRange, hue)) + hueShift;

        vec3 color = hslToRgb(vec3(hue, 1.0, 0.5));

        vec3 l = color + vec3(5.0, 2.5, 3.0) * f;
        cl = cl * l + smoothstep(2.5, 0.0, rz) * 0.7 * l;

        d += min(rz, 1.0);
    }

    return cl;
}



void main() {
    vec3 color = vec3(0.0);
    float depthSum = 0.0;

    for (int i = 0; i < SAMPLES; i++) {
        float depth = FOCAL_DISTANCE + (float(i) / float(SAMPLES - 1)) * FOCAL_RANGE;
        vec3 sampleColor = getColor(gl_FragCoord.xy, depth);
        float weight = 1.0 / (1.0 + abs(depth - FOCAL_DISTANCE));

        color += sampleColor * weight;
        depthSum += weight;
    }

    color /= depthSum;

    fragColor = vec4(color.r, color.g * 0.125, color.b, 1.0);
}
