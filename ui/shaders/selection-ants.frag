#version 440
layout(location=0) in vec2 qt_TexCoord0;
layout(location=0) out vec4 fragColor;
layout(std140,binding=0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    vec4 documentRect;
    vec2 viewportSize;
    float phase;
} ubuf;
layout(binding=1) uniform sampler2D mask;
float coverage(vec2 uv) {
    float inside=step(0.,uv.x)*step(0.,uv.y)*step(uv.x,1.)*step(uv.y,1.);
    return texture(mask,clamp(uv,vec2(0.),vec2(1.))).a*inside;
}
float band(vec2 uv,vec2 pixel) {
    float lo=coverage(uv),hi=lo;
    for(int y=-1;y<=1;y++)for(int x=-1;x<=1;x++) {
        float a=coverage(uv+vec2(x,y)*pixel);
        lo=min(lo,a);hi=max(hi,a);
    }
    return hi-lo;
}
void main() {
    vec2 uv=(qt_TexCoord0*ubuf.viewportSize-ubuf.documentRect.xy)/max(ubuf.documentRect.zw,vec2(.0001));
    vec2 pixel=max(fwidth(uv),vec2(.000000001));
    float thin=band(uv,pixel*.5);
    float alpha=max(band(uv,pixel),thin);
    float stripe=step(4.,mod(gl_FragCoord.x+gl_FragCoord.y+ubuf.phase,8.));
    float white=thin*stripe;
    fragColor=vec4(vec3(white),alpha)*ubuf.qt_Opacity;
}
