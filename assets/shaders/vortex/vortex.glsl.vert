#version 460 core

layout(location = 0) in vec2 a_pos;

out vec2 v_uv;
out vec2 surfacePosition;

void main() {
    v_uv = a_pos * 0.5 + 0.5;
    surfacePosition = a_pos;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
