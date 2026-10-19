// Wire layout is authored by paint_ops::pack. No document pointers cross this boundary.
@group(0) @binding(0) var<storage,read> data:array<vec4<f32>>;
@group(0) @binding(1) var<storage,read_write> output:array<vec4<f32>>;
@group(0) @binding(2) var<storage,read> source_pixels:array<vec4<f32>>;
fn u(v:f32)->u32 {return bitcast<u32>(v);}
fn linear(a:f32,b:f32,t:f32)->f32 {return a*(1.0-t)+b*t;}
fn at(x:i32,y:i32)->vec2<f32> {
    let w=i32(u(data[0].z));let h=i32(u(data[0].w));
    if x<0 || y<0 || x>=w || y>=h {return vec2(0.0);}
    return data[12u+u32(y*w+x)].xy;
}
fn grid(p:vec2<f32>)->vec2<f32> {
    let v=(p-data[1].xy)/data[1].z;let q=vec2<i32>(floor(v));let f=fract(v);
    return (at(q.x,q.y)*(1.0-f.x)+at(q.x+1,q.y)*f.x)*(1.0-f.y)
        +(at(q.x,q.y+1)*(1.0-f.x)+at(q.x+1,q.y+1)*f.x)*f.y;
}
fn elastic(i:u32)->vec4<f32> {
    let size=u32(data[3].z);let p=data[1].xy+(data[3].xy+vec2<f32>(f32(i%size),f32(i/size)))*data[1].z;
    let v=p-data[4].xy;let r=length(v)/data[5].x;let before=grid(p);
    let inner=data[5].z*0.85;let t=clamp((r-inner)/(1.0-inner),0.0,1.0);
    let weight=data[5].y*(1.0-t*t*(3.0-2.0*t))*data[u(data[2].x)+i].x;
    if weight<=0.0 {return vec4(before,0.0,0.0);}
    let tool=u(data[6].y);
    if tool==3u {var d=before*max(1.0-weight,0.0);if abs(d.x)<0.0001 {d.x=0.0;}if abs(d.y)<0.0001 {d.y=0.0;}return vec4(d,0.0,0.0);}
    var shift=-data[4].zw*weight;
    if tool==1u {shift=v*(exp(-weight*data[6].x*data[5].w*2.0)-1.0);}
    if tool==2u {let a=-weight*data[6].x*data[5].w*6.28318530718;let s=sin(a);let c=cos(a);shift=vec2(v.x*c-v.y*s-v.x,v.x*s+v.y*c-v.y);}
    let q=p+shift;let d=grid(q);let h=data[1].z;
    let mean=(grid(q-vec2(h,0.0))+grid(q+vec2(h,0.0))+grid(q-vec2(0.0,h))+grid(q+vec2(0.0,h)))*0.25;
    let amount=weight*0.12;
    return vec4(shift+d*(1.0-amount)+mean*amount,0.0,0.0);
}
fn source(p:vec2<i32>)->vec4<f32> {
    let key=vec2<i32>(floor(vec2<f32>(p)/64.0));
    var left=0u;var right=u(data[4].y);let table=u(data[4].x);
    loop {
        if left>=right {break;}
        let mid=(left+right)/2u;let item=data[table+mid];let k=vec2<i32>(item.xy);
        if k.x==key.x && k.y==key.y {
            let local=p-key*64;let address=u(item.z)+u32(local.y*64+local.x);
            if u(data[1].w)>0u {return source_pixels[address];}
            return data[address];
        }
        if k.x<key.x || (k.x==key.x && k.y<key.y) {left=mid+1u;} else {right=mid;}
    }
    if p.x>=0 && p.y>=0 && f32(p.x)<data[4].z && f32(p.y)<data[4].w {return vec4(1.0);}
    return vec4(0.0);
}
fn cubic(x0:f32)->f32 {
    let x=abs(x0);
    if x<1.0 {return 1.5*x*x*x-2.5*x*x+1.0;}
    if x<2.0 {return -0.5*x*x*x+2.5*x*x-4.0*x+2.0;}
    return 0.0;
}
fn sample_image(p:vec2<f32>)->vec4<f32> {
    let q=vec2<i32>(floor(p));let f=fract(p);
    if u(data[3].w)==0u {return (source(q)*(1.0-f.x)+source(q+vec2(1,0))*f.x)*(1.0-f.y)+(source(q+vec2(0,1))*(1.0-f.x)+source(q+vec2(1,1))*f.x)*f.y;}
    var sum=vec4(0.0);let first=source(q);var uniform=true;
    for(var y=-1;y<=2;y++){for(var x=-1;x<=2;x++){let value=source(q+vec2(x,y));uniform=uniform && all(value==first);sum+=value*(cubic(f32(x)-f.x)*cubic(f32(y)-f.y));}}
    if uniform {return first;}
    sum.a=clamp(sum.a,0.0,1.0);return vec4(clamp(sum.rgb,vec3(0.0),vec3(sum.a)),sum.a);
}
// One invocation per preview block, never one repeated sample per output pixel.
fn compact_block(i:u32)->vec4<f32> {
    let stride=u(data[3].z);let side=64u/stride;let blocks=side*side;
    let tile=vec2<i32>(data[u(data[3].x)+i/blocks].xy);
    let local=vec2<u32>((i%blocks)%side,(i%blocks)/side)*stride;
    let p=vec2<f32>(tile*64+vec2<i32>(local));let d=grid(p);
    if all(d==vec2(0.0)) {return vec4(0.0,0.0,0.0,-1.0);}
    return sample_image(p+d);
}
fn remap_block(i:u32) {
    let stride=u(data[3].z);let side=64u/stride;let blocks=side*side;
    let tile=vec2<i32>(data[u(data[3].x)+i/blocks].xy);
    let local=vec2<u32>((i%blocks)%side,(i%blocks)/side)*stride;
    let p=tile*64+vec2<i32>(local);let d=grid(vec2<f32>(p));
    let identity=all(d==vec2(0.0));var sampled=vec4(0.0);
    if !identity {sampled=sample_image(vec2<f32>(p)+d);}
    for(var y=0u;y<stride;y++){for(var x=0u;x<stride;x++){
        let index=(i/blocks)*4096u+(local.y+y)*64u+local.x+x;
        var selected=1.0;if u(data[2].y)>0u {selected=data[u(data[2].x)+index].x;}
        if identity || selected==0.0 {output[index]=source(p+vec2<i32>(i32(x),i32(y)));}
        else if selected==1.0 {output[index]=sampled;}
        else {output[index]=source(p+vec2<i32>(i32(x),i32(y)))*(1.0-selected)+sampled*selected;}
    }}
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
    let i=id.x+id.y*65535u*64u;
    if u(data[0].x)==0u {if i<u(data[0].y) {output[i]=elastic(i);}}
    else if u(data[0].x)==2u {if i<u(data[0].y) {output[i]=compact_block(i);}}
    else {let stride=u(data[3].z);if i<u(data[0].y)/(stride*stride) {remap_block(i);}}
}
