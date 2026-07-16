#version 330 core

// --- STANDARD VARYINGS IN ---
in vec2 texCoord;
in vec2 vTexOffset;
in vec2 vRegionSize;
{{extended_varyings_in}}

// --- STANDARD OUTPUT ---
out vec4 FragColor;

// --- STANDARD UNIFORMS (always present, set by UniformProvider) ---
uniform sampler2D uTexture;
{{extended_uniforms}}

void main() {
    // UV wrapping within the atlas region
    vec2 localUV = fract(texCoord);
    vec2 atlasUV = vTexOffset + localUV * vRegionSize;

    vec4 color = texture(uTexture, atlasUV);

    {{fragment_main_hook}}

    FragColor = color;
}