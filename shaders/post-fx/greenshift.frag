#version 330 core

in vec2 uv;

out vec4 FragColor;

uniform sampler2D uScene;

void main() {
    vec4 color = texture(uScene, uv);
    FragColor = vec4(color.r * 0.5, color.g, color.b * 0.5, color.a);
}