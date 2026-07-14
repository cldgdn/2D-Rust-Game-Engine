#version 330 core

// --- STANDARD VERTEX ATTRIBUTES (always present, renderer-managed) ---
layout (location = 0) in vec3 aPos;          // xy = local position, z = depth
layout (location = 1) in vec2 aTexCoord;
layout (location = 2) in vec2 aTexOffset;    // atlas region origin
layout (location = 3) in vec2 aRegionSize;   // atlas region extents
layout (location = 4) in mat4 aModel;        // occupies locations 4, 5, 6, 7
{{extended_attributes}}

// --- STANDARD UNIFORMS (always present, set by UniformProvider) ---
uniform mat4 uViewProjection;
{{extended_uniforms}}

// --- STANDARD VARYINGS OUT ---
out vec2 texCoord;
out vec2 vTexOffset;
out vec2 vRegionSize;
{{extended_varyings_out}}

void main() {
    vec4 worldPos = aModel * vec4(aPos.xy, 0.0, 1.0);
    gl_Position = uViewProjection * vec4(worldPos.xy, aPos.z, 1.0);

    texCoord = aTexCoord;
    vTexOffset = aTexOffset;
    vRegionSize = aRegionSize;
    {{vertex_main_hook}}
}