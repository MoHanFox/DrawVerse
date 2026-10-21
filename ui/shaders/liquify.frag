#version 440
layout(location=0) in vec2 sourceCoord;
layout(location=1) in vec2 documentCoord;
layout(location=0) out vec4 fragColor;
layout(std140,binding=0) uniform State {mat4 matrix;float opacity;float focus;float selected;vec4 identity;} ub;
layout(binding=1) uniform sampler2D sourceImage;
layout(binding=2) uniform sampler2D belowImage;
layout(binding=3) uniform sampler2D aboveImage;
layout(binding=4) uniform sampler2D selectionImage;
vec4 decode(vec4 c){
    if(c.a<=0.0)return vec4(0.0);
    vec3 s=clamp(c.rgb/c.a,0.0,1.0);
    vec3 l=mix(s/12.92,pow((s+0.055)/1.055,vec3(2.4)),step(vec3(0.04045),s));
    return vec4(l*c.a,c.a);
}
vec4 texel(sampler2D image,ivec2 p,ivec2 size){
    if(any(lessThan(p,ivec2(0))) || any(greaterThanEqual(p,size)))return vec4(0.0);
    return decode(texelFetch(image,p,0));
}
vec4 sampleLinear(sampler2D image,vec2 uv){
    ivec2 size=textureSize(image,0);
    vec2 p=uv*vec2(size)-0.5;
    ivec2 a=ivec2(floor(p));vec2 f=fract(p);
    return mix(mix(texel(image,a,size),texel(image,a+ivec2(1,0),size),f.x),
               mix(texel(image,a+ivec2(0,1),size),texel(image,a+ivec2(1,1),size),f.x),f.y);
}
vec4 over(vec4 a,vec4 b){return a+b*(1.0-a.a);}
void main(){
    if(any(lessThan(documentCoord,vec2(0.0))) || any(greaterThan(documentCoord,vec2(1.0))))discard;
    vec4 c=sampleLinear(sourceImage,sourceCoord);
    if(ub.selected>0.5)c=mix(sampleLinear(sourceImage,documentCoord*ub.identity.zw+ub.identity.xy),c,texture(selectionImage,documentCoord).r);
    if(ub.focus<0.5)c=over(sampleLinear(aboveImage,documentCoord),over(c,sampleLinear(belowImage,documentCoord)));
    if(c.a<=0.0){fragColor=vec4(0.0);return;}
    vec3 l=clamp(c.rgb/c.a,0.0,1.0);
    vec3 s=mix(12.92*l,1.055*pow(l,vec3(1.0/2.4))-0.055,step(vec3(0.0031308),l));
    fragColor=vec4(s*c.a,c.a)*ub.opacity;
}
