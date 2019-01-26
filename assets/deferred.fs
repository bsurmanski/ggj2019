uniform sampler2D tex;

in vec2 fuv;
out vec4 color;

void main()
{
    vec4 c = texture(tex, fuv); 
    if(c.a < 0.1) discard;
    color = c;
}
