#version 330 core

in vec2 texCoord;

out vec4 FragColor;

uniform sampler2D uTexture;
uniform vec4 uColor;

void main() {
    vec4 color = (uColor.rgb, uColor.a * (texture(uTexture, texCoord)).a);
    FragColor = color;
}