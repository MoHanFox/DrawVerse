#version 440
layout(location=0) in vec4 a;
layout(location=1) in vec4 b;
layout(location=2) in vec4 c;
layout(location=0) out vec2 uv;
layout(location=1) flat out vec4 triangleA;
layout(location=2) flat out vec4 triangleB;
layout(location=3) flat out vec4 triangleC;
layout(std140,binding=0) uniform Parameters { ivec4 dimensions; };
void main(){
    triangleA=a;triangleB=b;triangleC=c;
    vec2 lo=floor(min(a.xy,min(b.xy,c.xy))),hi=ceil(max(a.xy,max(b.xy,c.xy)));
    int corners[6]=int[6](0,1,2,0,2,3);int corner=corners[gl_VertexIndex];
    vec2 position=vec2(corner==1 || corner==2?hi.x:lo.x,corner>=2?hi.y:lo.y);
    uv=position/vec2(dimensions.xy);
    gl_Position=vec4(uv.x*2.0-1.0,1.0-uv.y*2.0,0,1);
}
