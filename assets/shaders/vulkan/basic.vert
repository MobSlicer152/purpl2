#version 460

layout (binding = 0) uniform Ubo {
    mat4 Model;
    mat4 View;
    mat4 Projection;
} UniformBufferObject;

layout (location = 0) in vec3 InputPosition;
layout (location = 1) in vec4 InputColour;

layout (location = 0) out vec4 FragmentColour;

void
main()
{
    mat4 Mvp = UniformBufferObject.Projection * UniformBufferObject.View * UniformBufferObject.Model;
    gl_Position = Mvp * vec4(InputPosition, 1);
    FragmentColour = InputColour;
}
