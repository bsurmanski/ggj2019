uniform sampler2D tex;
uniform float xtrim;
uniform float tick;
uniform vec2 bounce;

in vec2 fuv;
out vec4 color;

void main()
{
    float x = fuv.x * 2.0f - 1.0f;
    float xx = x * x;
    float xxxx = xx * xx;
    float texcox = -xxxx + 1.0f;
    float stexcox = fuv.x + texcox * sin(tick + fuv.y * 5) * bounce.x;

    float y = fuv.x * 2.0f - 1.0f;
    float yy = y * y;
    float yyyy = yy * yy;
    float texcoy = -yyyy + 1.0f;
    float stexcoy = fuv.y + texcoy * sin(tick + fuv.x * 5) * bounce.y;

    vec4 c = texture(tex, vec2(stexcox, stexcoy)); 
    if(fuv.x > xtrim) discard;
    if(c.a < 0.1) discard;
    color = vec4(stexcox, stexcoy, 1.0, 1.0);
}
