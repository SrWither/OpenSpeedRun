#version 100
precision mediump float;

uniform float u_time;
uniform vec2 u_resolution;

float noise(vec2 p) {
    return fract(sin(dot(p, vec2(12.9898,78.233))) * 43758.5453);
}

float smoothNoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);

    float a = noise(i);
    float b = noise(i + vec2(1.0, 0.0));
    float c = noise(i + vec2(0.0, 1.0));
    float d = noise(i + vec2(1.0, 1.0));

    vec2 u = f * f * (3.0 - 2.0 * f);

    return mix(a, b, u.x) + (c - a)* u.y * (1.0 - u.x) + (d - b) * u.x * u.y;
}

void main() {
    vec2 uv = gl_FragCoord.xy / u_resolution.xy * 3.0;

    float n = smoothNoise(uv + vec2(u_time * 0.5, u_time * 0.5));
    float intensity = smoothNoise(uv * 3.0 + vec2(n, n));

    vec3 color = vec3(1.0, 0.3, 0.0) * intensity;
    color += vec3(1.0, 0.6, 0.2) * pow(intensity, 3.0);

    gl_FragColor = vec4(color, 1.0);
}
