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
// Same linear, premultiplied formulas and branch boundaries as
// paint-core/layers/appearance.rs. Dissolve is not negotiated by this backend.
float luminance(vec3 c){return c.r*.3+c.g*.59+c.b*.11;}
float saturation(vec3 c){return max(0.0,max(c.r,max(c.g,c.b)))-min(1.0,min(c.r,min(c.g,c.b)));}
vec3 setLum(vec3 c,float l){
    c+=l-luminance(c);float light=luminance(c),lo=min(c.r,min(c.g,c.b)),hi=max(c.r,max(c.g,c.b));
    if(lo<0.0)c=light+(c-light)*light/(light-lo);
    if(hi>1.0)c=light+(c-light)*(1.0-light)/(hi-light);
    return clamp(c,0.0,1.0);
}
vec3 setSat(vec3 c,float s){float lo=min(1.0,min(c.r,min(c.g,c.b))),hi=max(0.0,max(c.r,max(c.g,c.b)));return hi<=lo?vec3(0):(c-lo)*s/(hi-lo);}
float burn(float b,float s){return b>=1.0?1.0:(s<=0.0?0.0:1.0-min((1.0-b)/s,1.0));}
float dodge(float b,float s){return b<=0.0?0.0:(s>=1.0?1.0:min(b/(1.0-s),1.0));}
float hard(float b,float s){return s<=.5?2.0*b*s:1.0-2.0*(1.0-b)*(1.0-s);}
float vivid(float b,float s){return s<=.5?burn(b,2.0*s):dodge(b,2.0*s-1.0);}
float channel(float b,float s,int mode){
    if(mode==2)return min(b,s);if(mode==3)return b*s;
    if(mode==4)return burn(b,s);if(mode==5)return max(b+s-1.0,0.0);
    if(mode==7)return max(b,s);if(mode==8)return b+s-b*s;
    if(mode==9)return dodge(b,s);if(mode==10)return min(b+s,1.0);
    if(mode==12)return hard(s,b);
    if(mode==13){if(s<=.5)return b-(1.0-2.0*s)*b*(1.0-b);float f=b<=.25?((16.0*b-12.0)*b+4.0)*b:sqrt(b);return b+(2.0*s-1.0)*(f-b);}
    if(mode==14)return hard(b,s);if(mode==15)return vivid(b,s);
    if(mode==16)return clamp(b+2.0*s-1.0,0.0,1.0);
    if(mode==17)return s<=.5?min(b,2.0*s):max(b,2.0*s-1.0);
    if(mode==18)return vivid(b,s)<.5?0.0:1.0;
    if(mode==19)return abs(b-s);if(mode==20)return b+s-2.0*b*s;
    if(mode==21)return max(b-s,0.0);if(mode==22)return b==0.0?0.0:(s==0.0?1.0:min(b/s,1.0));
    return s;
}
vec4 blend(vec4 s,vec4 d,int mode){
    if(mode==0)return s+d*(1.0-s.a);
    float a=s.a,da=d.a;if(a==0.0)return d;
    vec3 sc=s.rgb/a,dc=da>0.0?d.rgb/da:vec3(0),mixed;
    if(mode==23)mixed=setLum(setSat(sc,saturation(dc)),luminance(dc));
    else if(mode==24)mixed=setLum(setSat(dc,saturation(sc)),luminance(dc));
    else if(mode==25)mixed=setLum(sc,luminance(dc));
    else if(mode==26)mixed=setLum(dc,luminance(sc));
    else if(mode==6)mixed=luminance(sc)<luminance(dc)?sc:dc;
    else if(mode==11)mixed=luminance(sc)>luminance(dc)?sc:dc;
    else mixed=vec3(channel(dc.r,sc.r,mode),channel(dc.g,sc.g,mode),channel(dc.b,sc.b,mode));
    float alpha=a+da*(1.0-a);
    return vec4(clamp((1.0-a)*d.rgb+(1.0-da)*a*sc+da*a*mixed,0.0,alpha),alpha);
}
void main(){
    if(factors.w>4.5){
        ivec2 p=ivec2(uv*vec2(dimensions.xy));vec4 d=texelFetch(backdrop,p,0);
        if(factors.w<5.5){color=d;return;}
        vec4 s=texelFetch(sourceMap,p,0);
        float mask=1.0-(factors.y>0.0?sampleSource(p,maskInfo,maskOrigin.xy,maskMap).a*factors.y:0.0);
        if(factors.w<6.5){s*=mask;s*=factors.x;color=blend(s,d,sourceOrigin.z);return;}
        if(all(equal(d,s))){color=d;return;}
        float amount=clamp(factors.x*mask,0.0,1.0);
        float alpha=d.a*(1.0-amount)+s.a*amount;
        color=vec4(min(d.rgb*(1.0-amount)+s.rgb*amount,vec3(alpha)),alpha);return;
    }
    if(factors.w>3.5){ivec2 p=ivec2(uv*vec2(dimensions.xy));vec4 s=texelFetch(atlas,p,0);color=s+texelFetch(backdrop,p,0)*(1.0-s.a);return;}
    if(factors.w>1.5){color=vec4(0);return;}
    if(factors.w<0.5){
        ivec2 point=ivec2(uv*vec2(dimensions.xy));
        vec4 s=sampleSource(point,sourceInfo,sourceOrigin.xy,sourceMap);
        float hidden=factors.y>0.0?sampleSource(point,maskInfo,maskOrigin.xy,maskMap).a*factors.y:0.0;
        s*=1.0-hidden;
        s*=factors.x;
        color=blend(s,texelFetch(backdrop,point,0),sourceOrigin.z);
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
