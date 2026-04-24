#version 330 core

in vec2 texCoord;

out vec4 FragColor;

uniform vec4 uColor;

void main() {
    FragColor = uColor;
}