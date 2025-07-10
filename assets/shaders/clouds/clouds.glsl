#version 460 core

out vec4 fragColor;

uniform float u_time;
uniform vec2 u_resolution;

#define TAU 6.28318530718

const vec3 BackColor  = vec3(0.1, 0.7, 0.9);
const vec3 CloudColor = vec3(0.10, 0.6, 0.9);

float Func(float pX)
{
    return 0.6 * (0.5 * sin(0.1 * pX) + 0.5 * sin(0.553 * pX) + 0.7 * sin(1.2 * pX));
}

float FuncR(float pX)
{
    return 0.5 + 0.25 * (1.0 + sin(mod(40.0 * pX, TAU)));
}

float Layer(vec2 pQ, float pT)
{
    vec2 Qt = 3.5 * pQ;
    pT *= 0.5;
    Qt.x += pT;

    float Xi = floor(Qt.x);
    float Xf = Qt.x - Xi - 0.5;

    vec2 C;
    float Yi;
    float D = 1.0 - step(Qt.y, Func(Qt.x));

    // Disk:
    Yi = Func(Xi + 0.5);
    C = vec2(Xf, Qt.y - Yi);
    D = min(D, length(C) - FuncR(Xi + pT / 80.0));

    // Previous disk:
    Yi = Func(Xi + 1.0 + 0.5);
    C = vec2(Xf - 1.0, Qt.y - Yi);
    D = min(D, length(C) - FuncR(Xi + 1.0 + pT / 80.0));

    // Next Disk:
    Yi = Func(Xi - 1.0 + 0.5);
    C = vec2(Xf + 1.0, Qt.y - Yi);
    D = min(D, length(C) - FuncR(Xi - 1.0 + pT / 80.0));

    return min(1.0, D);
}

void main()
{
    vec2 fragCoord = gl_FragCoord.xy;
    vec2 UV = 2.0 * (fragCoord - u_resolution * 0.5) / min(u_resolution.x, u_resolution.y);

    vec3 Color = BackColor;

    for (float J = 0.0; J <= 1.0; J += 0.2)
    {
        float Lt = u_time * (0.5 + 1.0 * J) * (1.0 + 0.1 * sin(226.0 * J)) + 17.0 * J;
        vec2 Lp = vec2(0.0, 0.3 + 1.5 * (J - 0.5));
        float L = Layer(UV + Lp, Lt);

        float Blur = 1.0 * (0.5 * abs(2.0 - 5.0 * J)) / (11.0 - 5.0 * J);

        float V = mix(0.0, 1.0, 1.0 - smoothstep(0.0, 0.01 + 0.2 * Blur, L));
        vec3 Lc = mix(CloudColor, vec3(1.0), J);

        Color = mix(Color, Lc, V);
    }

    fragColor = vec4(Color, 1.0);
}
