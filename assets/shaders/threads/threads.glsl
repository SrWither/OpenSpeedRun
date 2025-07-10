#version 460 core

uniform float u_time;
uniform vec2 u_resolution;
uniform sampler2D backbuffer;

out vec4 fragColor;

const vec3 diffuse = vec3(0.99, 0.65, 0.2);
const vec3 eps = vec3(0.001, 0.0, 0.0);
const int iter = 128;
float sq = sqrt(2.0) * 0.5;

float c(vec3 p) {
    vec3 q = abs(mod(p + vec3(cos(p.z * 0.5), cos(p.x * 0.5), cos(p.y * 0.5)), 2.0) - 1.0);
    float a = q.x + q.y + q.z - min(min(q.x, q.y), q.z) - max(max(q.x, q.y), q.z);
    q = vec3(p.x + p.y, p.y + p.z, p.z + p.x) * sq;
    q = abs(mod(q, 2.0) - 1.0);
    float b = q.x + q.y + q.z - min(min(q.x, q.y), q.z) - max(max(q.x, q.y), q.z);
    return min(a, b);
}

vec3 n(vec3 p) {
    float o = c(p);
    return normalize(o - vec3(
        c(p - eps),
        c(p - eps.zxy),
        c(p - eps.yzx)
    ));
}

void main() {
    float aspect = u_resolution.x / u_resolution.y;
    vec2 p = gl_FragCoord.xy / u_resolution * 2.0 - 1.0;
    p.x *= aspect;

    vec3 o = vec3(0.0, 0.0, u_time);
    vec3 s = vec3(0.0);
    vec3 b = vec3(11.0, 0.0, 0.0);
    vec3 d = vec3(p, 1.0) / 32.0;
    vec3 t = vec3(0.5);
    vec3 a;

    for (int i = 0; i < iter; ++i) {
        float h = c(b + s + o);
        b += h * 10.0 * d;
        t += h;
    }

    t /= float(iter);
    a = n(b + s + o);
    float x = dot(a, t);
    t = (t + pow(x, 4.0)) * (1.0 - t * 0.01) * diffuse;
    t *= b.z * 0.125;

    vec2 vig = p * 0.43;
    vig.y *= aspect;
    float vig_amount = 1.0 - length(vig);

    fragColor = vec4(t * 2.0, 1.0) * vig_amount;
}
