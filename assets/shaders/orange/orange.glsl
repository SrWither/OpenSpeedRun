#version 460 core

out vec4 fragColor;

uniform float u_time;
uniform vec2 u_resolution;

float square(vec2 p, vec2 s)
{
    return length(max(vec2(0.0), abs(p) - s));
}

const int n = 6;


vec2 rotate(vec2 p, float angle)
{
    float c = cos(angle);
    float s = sin(angle);
    return vec2(c * p.x - s * p.y, s * p.x + c * p.y);
}

float greybars(vec2 p, float t)
{
    float d = 1e10;
    
    vec2 pr = rotate(p, radians(45.0));
    
    for (int i = 0; i < n; i++)
    {
        float i2 = float(i) / float(n);
        float ofs = abs(cos(i2 * 70.0));
        
        vec2 o = vec2(pow(2.0 * (-1.0 + t * 2.0) - ofs, 3.0), (-0.8 + 2.0 * i2) * 0.6);
        
        d = min(d, square(pr - o, vec2(0.8, 0.07)));
    }

    return smoothstep(0.01, 0.0, d) * 0.15;
}

float shade(vec2 p)
{
    p.x += u_time * mix(0.2, -0.2, step(p.y, 0.0));
    p.y = floor(p.y * 30.0 + cos(p.x * 30.0) * 0.1) / 30.0;
    return (smoothstep(-1.0, -0.7, p.y) - smoothstep(0.7, 1.0, p.y)) * 0.5 + 0.5;
}

void main()
{
    vec2 t = (gl_FragCoord.xy / u_resolution.xy - vec2(0.5)) * 2.0 * vec2(u_resolution.x / u_resolution.y, 1.0);

    float tt = u_time * 0.2;
    float zoom = fract(tt);

    float f = greybars(t, zoom);
    float s = shade(t);

    vec3 baseColor = mix(vec3(0.7, 0.35, 0.05), vec3(1.0, 0.6, 0.1), f) * 0.8;
    vec3 color = pow(baseColor * s, vec3(0.9));

    fragColor = vec4(color, 1.0);
}
