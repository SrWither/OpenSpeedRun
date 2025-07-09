#version 100
precision mediump float;

uniform float u_time;
uniform vec2 u_resolution;

void main() {
    vec2 uv = gl_FragCoord.xy / u_resolution;
    uv = uv * 2.0 - 1.0; // remap to [-1, 1]
    uv.x *= u_resolution.x / u_resolution.y;

    // Moving sun center
    float sun = smoothstep(0.3, 0.0, length(uv - vec2(0.0, -0.2)));

    // Sunset gradient
    vec3 sunset = mix(vec3(0.1, 0.0, 0.2), vec3(1.0, 0.4, 0.3), uv.y + 0.5);

    // Horizon glow
    float glow = exp(-pow(uv.y * 4.0, 2.0)) * 0.7;

    // Scanlines
    float lines = sin((uv.y + u_time * 0.5) * 80.0) * 0.1;

    // Combine
    vec3 color = sunset + vec3(glow) + sun * vec3(1.0, 0.6, 0.3);
    color += lines;

    gl_FragColor = vec4(color, 1.0);
}
