#version 440
layout(location=0) in vec2 uv;
layout(location=0) out vec4 color;
layout(std140,binding=0) uniform Parameters {
    ivec4 dimensions; // document width/height, output width/height
    ivec4 sourceInfo; // offset x/y, implicit white width/height
    ivec4 maskInfo;
    ivec4 sourceOrigin;
    ivec4 maskOrigin;
    vec4 factors; // raster opacity*fill, mask density, LOD block, display pass
};
layout(binding=1) uniform sampler2D backdrop;
layout(binding=2) uniform sampler2D atlas;
layout(binding=3) uniform sampler2D sourceMap;
layout(binding=4) uniform sampler2D maskMap;
vec4 sampleSource(ivec2 point,ivec4 info,ivec2 origin,sampler2D pageMap){
    ivec2 local=point-info.xy;
    ivec2 tile=ivec2(floor(vec2(local)/64.0));
    ivec2 page=tile-origin;
    int slot=-1;
    if(all(greaterThanEqual(page,ivec2(0))) && all(lessThan(page,textureSize(pageMap,0))))
        slot=int(texelFetch(pageMap,page,0).r)-1;
    if(slot>=0)return texelFetch(atlas,ivec2(slot%32,slot/32)*64+local-tile*64,0);
    return all(greaterThanEqual(local,ivec2(0))) && all(lessThan(local,info.zw))?vec4(1):vec4(0);
}
void main(){
    if(factors.w>3.5){ivec2 p=ivec2(uv*vec2(dimensions.xy));vec4 s=texelFetch(atlas,p,0);color=s+texelFetch(backdrop,p,0)*(1.0-s.a);return;}
    if(factors.w>1.5){color=vec4(0);return;}
    if(factors.w<0.5){
        ivec2 point=ivec2(uv*vec2(dimensions.xy));
        vec4 s=sampleSource(point,sourceInfo,sourceOrigin.xy,sourceMap);
        float hidden=factors.y>0.0?sampleSource(point,maskInfo,maskOrigin.xy,maskMap).a*factors.y:0.0;
        s*=factors.x*(1.0-hidden);
        color=s+texelFetch(backdrop,point,0)*(1.0-s.a);
    }else{
        ivec2 outputPoint=ivec2(uv*vec2(dimensions.zw));
        // Axis tables are prepared using the CPU reference's f64 pixel-center
        // mapping. Average the already-composited source, never individual layers.
        ivec2 point=ivec2(int(texelFetch(atlas,ivec2(outputPoint.x,0),0).r),
                         int(texelFetch(sourceMap,ivec2(outputPoint.y,0),0).r));
        if(any(lessThan(point,ivec2(0)))){color=vec4(0);return;}
        int block=int(factors.z);
        ivec2 begin=(point/block)*block;
        ivec2 end=min(begin+ivec2(block),dimensions.xy);
        vec4 s=vec4(0);
        float weight=1.0/float((end.x-begin.x)*(end.y-begin.y));
        for(int y=begin.y;y<end.y;++y)for(int x=begin.x;x<end.x;++x)
            s+=texelFetch(backdrop,ivec2(x,y),0)*weight;
        float a=clamp(s.a,0.0,1.0);
        vec3 v=a>0.0?clamp(s.rgb/a,0.0,1.0):vec3(0);
        vec3 encoded=mix(1.055*pow(v,vec3(1.0/2.4))-0.055,12.92*v,lessThanEqual(v,vec3(0.0031308)));
        color=vec4(encoded*a,a);
    }
}
