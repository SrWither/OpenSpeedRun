#version 460 core

uniform float u_time;
uniform vec2 u_resolution;

out vec4 fragColor;

// ---------- [Funciones auxiliares y constantes] ----------

float PI = 4.0 * atan(1.0);
vec3 sunLight = normalize(vec3(0.35, 0.2, 0.3));
vec3 sunColour = vec3(1.0, 0.75, 0.6);
const mat2 rotate2D = mat2(1.932, 1.623, -1.623, 1.95);

float Hash(float n) {
    return fract(sin(n) * 43758.5453123);
}

float Hash(vec2 p) {
    return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}

float Noise(in vec2 x) {
    vec2 p = floor(x);
    vec2 f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    float n = p.x + p.y * 57.0;
    return mix(mix(Hash(n + 0.0), Hash(n + 1.0), f.x),
               mix(Hash(n + 57.0), Hash(n + 58.0), f.x), f.y);
}

vec2 Voronoi(in vec2 x) {
    vec2 p = floor(x);
    vec2 f = fract(x);
    float res = 100.0, id;
    for (int j = -1; j <= 1; j++)
    for (int i = -1; i <= 1; i++) {
        vec2 b = vec2(float(i), float(j));
        vec2 r = b - f + Hash(p + b);
        float d = dot(r, r);
        if (d < res) {
            res = d;
            id = Hash(p + b);
        }
    }
    return vec2(max(0.4 - sqrt(res), 0.0), id);
}

vec2 Terrain(in vec2 p) {
    float type = 0.0;
    vec2 pos = p * 0.003;
    float w = 50.0;
    float f = 0.0;
    for (int i = 0; i < 3; i++) {
        f += Noise(pos) * w;
        w *= 0.62;
        pos *= 2.5;
    }
    return vec2(f, type);
}

vec2 Map(in vec3 p) {
    vec2 h = Terrain(p.xz);
    return vec2(p.y - h.x, h.y);
}

float FractalNoise(in vec2 xy) {
    float w = 0.7;
    float f = 0.0;
    for (int i = 0; i < 3; i++) {
        f += Noise(xy) * w;
        w *= 0.6;
        xy *= 2.0;
    }
    return f;
}

vec3 GetSky(in vec3 rd) {
    float sunAmount = max(dot(rd, sunLight), 0.0);
    float v = pow(1.0 - max(rd.y, 0.0), 6.0);
    vec3 sky = mix(vec3(0.1, 0.2, 0.3), vec3(0.32, 0.32, 0.32), v);
    sky += sunColour * sunAmount * sunAmount * 0.25;
    sky += sunColour * min(pow(sunAmount, 800.0) * 1.5, 0.3);
    return clamp(sky, 0.0, 1.0);
}

vec3 ApplyFog(in vec3 rgb, in float dis, in vec3 dir) {
    float fogAmount = clamp(dis * dis * 0.0000012, 0.0, 1.0);
    return mix(rgb, GetSky(dir), fogAmount);
}

vec3 DE(vec3 p) {
    float base = Terrain(p.xz).x - 1.9;
    float height = Noise(p.xz * 2.0) * 0.75 + Noise(p.xz) * 0.35 + Noise(p.xz * 0.5) * 0.2;
    float y = p.y - base - height;
    y = y * y;
    vec2 ret = Voronoi(p.xz * 2.5 + sin(y * 4.0 + p.zx * 12.3) * 0.12 +
        vec2(sin(u_time * 2.3 + 1.5 * p.z), sin(u_time * 3.6 + 1.5 * p.x)) * y * 0.5);
    float f = ret.x * 0.6 + y * 0.58;
    return vec3(y - f * 1.4, clamp(f * 1.5, 0.0, 1.0), ret.y);
}

float CircleOfConfusion(float t) {
    return max(t * 0.04, (2.0 / u_resolution.y) * (1.0 + t));
}

float Linstep(float a, float b, float t) {
    return clamp((t - a) / (b - a), 0.0, 1.0);
}

vec3 GrassBlades(in vec3 rO, in vec3 rD, in vec3 mat, in float dist) {
    float d = 0.0;
    float f;
    float rCoC = CircleOfConfusion(dist * 0.3);
    float alpha = 0.0;
    vec4 col = vec4(mat * 0.15, 0.0);

    for (int i = 0; i < 15; i++) {
        if (col.w > 0.99) break;
        vec3 p = rO + rD * d;
        vec3 ret = DE(p);
        ret.x += 0.5 * rCoC;
        if (ret.x < rCoC) {
            alpha = (1.0 - col.y) * Linstep(-rCoC, rCoC, -ret.x);
            f = clamp(ret.y, 0.0, 1.0);
            vec3 gra = mix(mat, vec3(0.35, 0.35, min(pow(ret.z, 4.0) * 35.0, 0.35)), pow(ret.y, 9.0) * 0.7) * ret.y;
            col += vec4(gra * alpha, alpha);
        }
        d += max(ret.x * 0.7, 0.1);
    }

    if (col.w < 0.2)
        col.xyz = vec3(0.1, 0.15, 0.05);
    return col.xyz;
}

void DoLighting(inout vec3 mat, in vec3 pos, in vec3 normal, in vec3 eyeDir, in float dis) {
    float h = dot(sunLight, normal);
    mat *= sunColour * (max(h, 0.0) + 0.2);
}

vec3 TerrainColour(vec3 pos, vec3 dir, vec3 normal, float dis, float type) {
    vec3 mat;
    if (type == 0.0) {
        mat = mix(vec3(0.0, 0.3, 0.0), vec3(0.2, 0.3, 0.0), Noise(pos.xz * 0.025));
        float t = FractalNoise(pos.xz * 0.1) + 0.5;
        mat = GrassBlades(pos, dir, mat, dis) * t;
        DoLighting(mat, pos, normal, dir, dis);
    }
    return ApplyFog(mat, dis, dir);
}

float BinarySubdivision(in vec3 rO, in vec3 rD, float t, float oldT) {
    float halfwayT = 0.0;
    for (int n = 0; n < 5; n++) {
        halfwayT = (oldT + t) * 0.5;
        if (Map(rO + halfwayT * rD).x < 0.05)
            t = halfwayT;
        else
            oldT = halfwayT;
    }
    return t;
}

bool Scene(in vec3 rO, in vec3 rD, out float resT, out float type) {
    float t = 5.0;
    float oldT = 0.0;
    vec2 h;
    bool hit = false;
    for (int j = 0; j < 80; j++) {
        vec3 p = rO + t * rD;
        if (p.y < 105.0 && !hit) {
            h = Map(p);
            if (h.x < 0.05) {
                resT = BinarySubdivision(rO, rD, t, oldT);
                type = h.y;
                hit = true;
            } else {
                float delta = max(0.04, 0.35 * h.x) + (t * 0.04);
                oldT = t;
                t += delta;
            }
        }
    }
    return hit;
}

vec3 CameraPath(float t) {
    vec2 p = vec2(200.0 * sin(3.54 * t), 200.0 * cos(2.0 * t));
    return vec3(p.x + 55.0, 12.0 + sin(t * 0.3) * 6.5, -94.0 + p.y);
}

vec3 PostEffects(vec3 rgb, vec2 xy) {
    rgb = pow(rgb, vec3(0.45));
    #define CONTRAST 1.1
    #define SATURATION 1.3
    #define BRIGHTNESS 1.3
    rgb = mix(vec3(0.5), mix(vec3(dot(vec3(0.2125, 0.7154, 0.0721), rgb * BRIGHTNESS)), rgb * BRIGHTNESS, SATURATION), CONTRAST);
    rgb *= 0.4 + 0.5 * pow(40.0 * xy.x * xy.y * (1.0 - xy.x) * (1.0 - xy.y), 0.2);
    return rgb;
}

void main() {
    float gTime = (u_time * 5.0 + 2352.0) * 0.006;
    vec2 xy = gl_FragCoord.xy / u_resolution.xy;
    vec2 uv = (-1.0 + 2.0 * xy) * vec2(u_resolution.x / u_resolution.y, 1.0);

    vec3 cameraPos = CameraPath(gTime);
    vec3 camTar = CameraPath(gTime + 0.009);
    cameraPos.y += Terrain(camTar.xz).x;
    camTar.y = cameraPos.y;

    float roll = 0.4 * sin(gTime + 0.5);
    vec3 cw = normalize(camTar - cameraPos);
    vec3 cp = vec3(sin(roll), cos(roll), 0.0);
    vec3 cu = cross(cw, cp);
    vec3 cv = cross(cu, cw);
    vec3 dir = normalize(uv.x * cu + uv.y * cv + 1.3 * cw);

    vec3 col;
    float distance;
    float type;
    if (!Scene(cameraPos, dir, distance, type)) {
        col = GetSky(dir);
    } else {
        vec3 pos = cameraPos + distance * dir;
        vec2 p = vec2(0.1, 0.0);
        vec3 nor = vec3(0.0, Terrain(pos.xz).x, 0.0);
        vec3 v2 = nor - vec3(p.x, Terrain(pos.xz + p).x, 0.0);
        vec3 v3 = nor - vec3(0.0, Terrain(pos.xz - p.yx).x, -p.x);
        nor = normalize(cross(v2, v3));
        col = TerrainColour(pos, dir, nor, distance, type);
    }

    float bri = dot(cw, sunLight) * 0.75;
    if (bri > 0.0) {
        vec2 sunPos = vec2(dot(sunLight, cu), dot(sunLight, cv));
        vec2 uvT = uv - sunPos;
        uvT *= length(uvT);
        bri = pow(bri, 6.0) * 0.8;

        float glare1 = max(dot(normalize(vec3(dir.x, dir.y + 0.3, dir.z)), sunLight), 0.0) * 1.4;
        float glare2 = max(1.0 - length(uvT + sunPos * 0.5) * 4.0, 0.0);
        uvT = mix(uvT, uv, -2.3);
        float glare3 = max(1.0 - length(uvT + sunPos * 5.0) * 1.2, 0.0);

        col += bri * vec3(1.0, 0.0, 0.0) * pow(glare1, 12.5) * 0.05;
        col += bri * vec3(1.0, 1.0, 0.2) * pow(glare2, 2.0) * 2.5;
        col += bri * sunColour * pow(glare3, 2.0) * 3.0;
    }

    col = PostEffects(col, xy);
    fragColor = vec4(col, 1.0);
}
