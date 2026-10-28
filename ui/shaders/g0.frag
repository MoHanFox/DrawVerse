#version 440
layout(location=0) in vec2 uv;
layout(location=0) out vec4 color;
layout(std140,binding=0) uniform Parameters {vec4 p;};
layout(binding=1) uniform sampler2D source;
layout(binding=2) uniform sampler2D auxiliary;
void main() {
    vec4 s=texture(source,uv);
    if(p.x<0.5)color=s*(1.0-texture(auxiliary,uv).a*p.y);
    else if(p.x<1.5)color=s+texture(auxiliary,uv)*(1.0-s.a);
    else {
        float a=clamp(s.a,0.0,1.0);
        vec3 v=a>0.0?clamp(s.rgb/a,0.0,1.0):vec3(0);
        vec3 encoded=mix(1.055*pow(v,vec3(1.0/2.4))-0.055,12.92*v,lessThanEqual(v,vec3(0.0031308)));
        color=vec4(encoded*a,a);
    }
}
