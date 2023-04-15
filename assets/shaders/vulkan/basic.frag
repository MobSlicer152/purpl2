#version 460

layout (location = 0) in vec4 FragmentColour;

layout (location = 0) out vec4 OutputColour;

void
main()
{
    OutputColour = FragmentColour;
}
