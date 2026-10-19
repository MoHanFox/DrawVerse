#version 440
layout(location=0) in vec2 position;
layout(location=1) in vec2 sourceUv;
layout(location=2) in vec2 documentUv;
layout(location=0) out vec2 sourceCoord;
layout(location=1) out vec2 documentCoord;
layout(std140,binding=0) uniform State {mat4 matrix;float opacity;float focus;} ub;
void main(){sourceCoord=sourceUv;documentCoord=documentUv;gl_Position=ub.matrix*vec4(position,0.0,1.0);}
