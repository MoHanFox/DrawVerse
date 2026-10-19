#version 440
layout(location=0) in vec2 qt_TexCoord0;
layout(location=0) out vec4 fragColor;
layout(std140,binding=0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    vec2 texel;
} ubuf;
layout(binding=1) uniform sampler2D background;
layout(binding=2) uniform sampler2D mask;
void main() {
    vec2 uv=qt_TexCoord0;
    float coverage=texture(mask,uv).a;
    float halo=max(max(texture(mask,uv+vec2(ubuf.texel.x,0)).a,texture(mask,uv-vec2(ubuf.texel.x,0)).a),
                   max(texture(mask,uv+vec2(0,ubuf.texel.y)).a,texture(mask,uv-vec2(0,ubuf.texel.y)).a));
    vec3 color=texture(background,uv).rgb;
    // Black/white keeps contrast at middle grey, unlike 1 - background.
    float ink=dot(color,vec3(.2126,.7152,.0722))<.5?1.:0.;
    float alpha=coverage+(1.-coverage)*halo*.8;
    float premul=ink*coverage+(1.-ink)*(1.-coverage)*halo*.8;
    fragColor=vec4(vec3(premul),alpha)*ubuf.qt_Opacity;
}
