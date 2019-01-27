uniform sampler2D tex;
uniform float tick;
uniform vec2 bounce;
uniform vec2 rtrim;
uniform vec2 trim;
uniform vec4 tint;

in vec2 fuv;
out vec4 color;

void main()
{
    float x = fuv.x * 2.0f - 1.0f;
    float xx = x * x;
    float xxxx = xx * xx;
    float texcox = -xxxx + 1.0f;
    float stexcox = fuv.x + texcox * sin(tick + fuv.y * 5.0) * bounce.x;

    float y = fuv.x * 2.0f - 1.0f;
    float yy = y * y;
    float yyyy = yy * yy;
    float texcoy = -yyyy + 1.0f;
    float stexcoy = fuv.y + texcoy * sin(tick + fuv.x * 5.0)  * bounce.y;

    vec4 c = texture(tex, vec2(stexcox, stexcoy)); 
    if((1.0 - fuv.x) > rtrim.x) discard;
    if(fuv.y > rtrim.y) discard;
    if(fuv.x > trim.x) discard;
    if((1.0 - fuv.y) > trim.y) discard;
    if(c.a < 0.1) discard;
    color = vec4(c.r * tint.r, c.g * tint.g, c.b * tint.b, c.a * tint.a);
}
