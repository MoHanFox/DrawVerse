#version 440
layout(location=0) out vec2 uv;
void main(){
    uv=vec2((gl_VertexIndex<<1)&2,gl_VertexIndex&2);
    // This adapter explicitly supports D3D11 only.
    gl_Position=vec4(uv.x*2.0-1.0,1.0-uv.y*2.0,0,1);
}
