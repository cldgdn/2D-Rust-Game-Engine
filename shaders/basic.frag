#version 330 core

in vec2 texCoord;

out vec4 FragColor;

uniform sampler2D uTexture;
uniform vec2 uTexOffset;
uniform vec2 uRegionSize;

void main() {
    vec2 localUV = fract(texCoord);
    vec2 atlasUV = uTexOffset + localUV * uRegionSize;

    vec4 color = texture(uTexture, atlasUV);
    
    FragColor = color;
}