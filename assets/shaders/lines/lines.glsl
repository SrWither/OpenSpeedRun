#version 330 core

in vec2 v_uv;
out vec4 FragColor;

uniform float u_time;
uniform vec2 u_resolution;

mat2 rotate(float a) {
    float s = sin(a), c = cos(a);
    return mat2(c, -s, s, c);
}

float field(vec2 p) {
    float a = 0.0;
    float t = u_time * 0.2;
    for (int i = 1; i < 7; i++) {
        float fi = float(i);
        vec2 rp = p * fi + vec2(sin(t * fi), cos(t * fi));
        a += sin(rp.x + rp.y + t) / fi;
    }
    return a;
}

void main() {
    vec2 uv = v_uv * 2.0 - 1.0;
    uv.x *= u_resolution.x / u_resolution.y;

    vec2 p = uv * 2.5;
    p *= rotate(u_time * 0.1);

    float f = field(p);
    float colorFactor = smoothstep(0.0, 1.0, f * 0.5 + 0.5);

    vec3 color = mix(vec3(0.1, 0.0, 0.2), vec3(0.5, 1.0, 2.0), colorFactor);
    color += pow(abs(f), 3.0) * 0.3;

    FragColor = vec4(color, 1.0);
}