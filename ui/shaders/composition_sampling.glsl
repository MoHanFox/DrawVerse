layout(location=0) in vec2 uv;
#ifdef WARP
layout(location=1) flat in vec4 triangleA;
layout(location=2) flat in vec4 triangleB;
layout(location=3) flat in vec4 triangleC;
#endif
layout(location=0) out vec4 color;
layout(std140,binding=0) uniform Parameters {
    ivec4 dimensions;
    ivec4 sourceInfo;
    ivec4 maskInfo;
    ivec4 sourceOrigin;
    ivec4 maskOrigin;
    vec4 factors;
    vec4 transformBounds;
    vec4 transformInfo;
    dvec4 inverse0;
    dvec4 inverse1;
    dvec4 inverse2;
};
layout(binding=1) uniform sampler2D backdrop;
layout(binding=2) uniform sampler2D atlas;
layout(binding=3) uniform sampler2D sourceMap;
layout(binding=4) uniform sampler2D maskMap;
vec4 sampleSource(ivec2 point){
    ivec2 local=point-sourceInfo.xy;
    ivec2 tile=ivec2(floor(dvec2(local)/64.0));
    ivec2 page=tile-sourceOrigin.xy;int slot=-1;
    if(all(greaterThanEqual(page,ivec2(0))) && all(lessThan(page,textureSize(sourceMap,0))))slot=int(texelFetch(sourceMap,page,0).r)-1;
    if(slot>=0)return texelFetch(atlas,ivec2(slot%32,slot/32)*64+local-tile*64,0);
    return all(greaterThanEqual(local,ivec2(0))) && all(lessThan(local,sourceInfo.zw))?vec4(1):vec4(0);
}
double cubic(double x,double a){
    x=abs(x);
    if(a==0.0){if(x<1.0)return (4.0-6.0*x*x+3.0*x*x*x)/6.0;if(x<2.0){double v=2.0-x;return v*v*v/6.0;}return 0.0;}
    if(x<1.0)return (a+2.0)*x*x*x-(a+3.0)*x*x+1.0;
    if(x<2.0)return a*x*x*x-5.0*a*x*x+8.0*a*x-4.0*a;
    return 0.0;
}
vec4 pixelMix(vec4 a,vec4 b,float t){
    precise vec4 s=a*(1.0-t)+b*t;s.rgb=min(s.rgb,vec3(s.a));return s;
}
vec4 transformed(dvec2 coordinates){
    precise dvec2 p=dvec2(transformBounds.xy)+coordinates*dvec2(transformBounds.zw)-0.5;
    int kind=int(transformInfo.z);
    if(kind==0)return sampleSource(ivec2(floor(p+0.5)));
    ivec2 origin=ivec2(floor(p));precise dvec2 f=p-dvec2(origin);
    if(kind==1)return pixelMix(pixelMix(sampleSource(origin),sampleSource(origin+ivec2(1,0)),float(f.x)),pixelMix(sampleSource(origin+ivec2(0,1)),sampleSource(origin+ivec2(1,1)),float(f.x)),float(f.y));
    double a=kind==3?0.0:(kind==4?-0.75:-0.5);precise vec4 sum=vec4(0);
    for(int y=-1;y<=2;++y)for(int x=-1;x<=2;++x){
        precise float weight=float(cubic(double(x)-f.x,a)*cubic(double(y)-f.y,a));
        sum+=sampleSource(origin+ivec2(x,y))*weight;
    }
    sum.a=clamp(sum.a,0.0,1.0);sum.rgb=clamp(sum.rgb,vec3(0),vec3(sum.a));return sum;
}
#ifdef WARP
double edge(dvec4 a,dvec4 b,dvec2 p){precise double v=(b.x-a.x)*(p.y-a.y)-(b.y-a.y)*(p.x-a.x);return v;}
bool topLeft(dvec4 a,dvec4 b){return b.y<a.y || (b.y==a.y && b.x>a.x);}
bool outside(double value,dvec4 a,dvec4 b){return value<0.0 || (value==0.0 && !topLeft(a,b));}
#endif
void main(){
    precise dvec2 p=dvec2(gl_FragCoord.xy);precise dvec2 coordinates;
#ifdef WARP
    dvec4 a=dvec4(triangleA),b=dvec4(triangleB),c=dvec4(triangleC);
    precise double area=edge(a,b,c.xy);
    if(abs(area)<1e-12lf)discard;
    if(area<0.0){dvec4 temp=b;b=c;c=temp;area=-area;}
    precise double e0=edge(b,c,p),e1=edge(c,a,p),e2=edge(a,b,p);
    if(outside(e0,b,c) || outside(e1,c,a) || outside(e2,a,b))discard;
    coordinates=(e0*a.zw+e1*b.zw+e2*c.zw)/area;
    color=transformed(coordinates)*factors.x;
#else
    precise double z=inverse2.x*p.x+inverse2.y*p.y+inverse2.z;
    coordinates=dvec2(inverse0.x*p.x+inverse0.y*p.y+inverse0.z,inverse1.x*p.x+inverse1.y*p.y+inverse1.z)/z;
    vec4 s=vec4(0);
    if(all(greaterThanEqual(coordinates,dvec2(0))) && all(lessThanEqual(coordinates,dvec2(1))))s=transformed(coordinates)*factors.x;
    color=s+texelFetch(backdrop,ivec2(p),0)*(1.0-s.a);
#endif
}
