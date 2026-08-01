#version 330 core

layout (location = 0) in vec2 aPos;
layout (location = 1) in vec2 aUV;

out vec2 uv;


uniform vec4 uSourceRect;

void main() {
    vec2 rectPos = vec2(uSourceRect.xy);
    vec2 rectUV = vec2(uSourceRect.zw);

    uv = (rectPos + (aUV * rectUV));
    gl_Position = vec4(aPos, 0.0, 1.0);
}