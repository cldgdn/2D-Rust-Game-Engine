#version 330 core

layout (location = 0) in vec2 aPos;
layout (location = 1) in vec2 aTexCoord;

uniform mat4 uViewProjection;
uniform mat4 uModel;

uniform vec2 uCameraPos;

out vec2 texCoord;

void main(){
    vec4 worldPos = uModel * vec4(aPos, 0.0, 1.0);

    float depth = worldPos.z;

    vec2 parallaxOffset = uCameraPos * (1.0 - depth);

    worldPos.xy -= parallaxOffset;

    gl_Position = uViewProjection * worldPos;

    texCoord = aTexCoord;
}