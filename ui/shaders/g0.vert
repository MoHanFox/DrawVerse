#version 440
layout(location=0) out vec2 uv;
void main() {
    uv=vec2((gl_VertexIndex<<1)&2,gl_VertexIndex&2);
    // G0 is explicitly D3D11-only: texture row 0 is the top row, NDC +Y up.
    gl_Position=vec4(uv.x*2.0-1.0,1.0-uv.y*2.0,0,1);
}
