#version 460

layout (binding = 0) uniform ubo {
    mat4 model;
    mat4 view;
    mat4 projection;
} uniform_buffer;

layout (location = 0) in vec3 in_position;
layout (location = 1) in vec4 in_color;

layout (location = 0) out vec4 fragment_color;

void main() {
    mat4 mvp = uniform_buffer.projection * uniform_buffer.view * uniform_buffer.model;
    gl_Position = mvp * vec4(in_position, 1);
    fragment_color = in_color;
}
