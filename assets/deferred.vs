in vec4 position;
in vec2 uv;

uniform mat2 transform;
uniform vec2 offset;

out vec2 fuv;

void main()
{
    gl_Position = vec4(transform * position.xy + offset, 1.0, 1.0);
    fuv = uv;
}
