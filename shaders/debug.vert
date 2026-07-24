#version 330 core

layout (location = 0) in vec2 aPos;

uniform mat4 uViewProjection;
uniform float uDepth;

void main() {
    gl_Position = uViewProjection * vec4(aPos, uDepth, 1.0);
}