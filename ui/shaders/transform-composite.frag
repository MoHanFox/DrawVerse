#version 440
layout(location=0) in vec2 qt_TexCoord0;
layout(location=0) out vec4 fragColor;
layout(std140,binding=0) uniform State {mat4 qt_Matrix;float qt_Opacity;} ub;
layout(binding=1) uniform sampler2D source;
layout(binding=2) uniform sampler2D belowImage;
layout(binding=3) uniform sampler2D aboveImage;
vec4 decode(vec4 c){
    if(c.a<=0.0)return vec4(0.0);
    vec3 s=clamp(c.rgb/c.a,0.0,1.0);
    vec3 l=mix(s/12.92,pow((s+0.055)/1.055,vec3(2.4)),step(vec3(0.04045),s));
    return vec4(l*c.a,c.a);
}
vec4 over(vec4 a,vec4 b){return a+b*(1.0-a.a);}
void main(){
    vec2 uv=qt_TexCoord0;
    vec4 c=over(decode(texture(aboveImage,uv)),over(decode(texture(source,uv)),decode(texture(belowImage,uv))));
    if(c.a<=0.0){fragColor=vec4(0.0);return;}
    vec3 l=clamp(c.rgb/c.a,0.0,1.0);
    vec3 s=mix(12.92*l,1.055*pow(l,vec3(1.0/2.4))-0.055,step(vec3(0.0031308),l));
    fragColor=vec4(s*c.a,c.a)*ub.qt_Opacity;
}
