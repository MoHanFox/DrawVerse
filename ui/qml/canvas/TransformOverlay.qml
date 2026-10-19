import QtQuick
import QtQuick.Controls
import "../menus"
Item {
    id:root;objectName:"transformOverlay"
    required property var controller
    required property var canvasView
    visible:controller.transformActive
    readonly property bool linked:controller.transformLinked
    property var startPoints:[]
    property point pressPoint
    property int handle:-1
    property bool rotating:false
    property int mode:0
    property bool dragging:false
    readonly property var points:controller.transformPoints
    function doc(p){return Qt.point((p.x-canvasView.documentRect.x)/canvasView.zoom,(p.y-canvasView.documentRect.y)/canvasView.zoom)}
    function screen(p){return Qt.point(canvasView.documentRect.x+p.x*canvasView.zoom,canvasView.documentRect.y+p.y*canvasView.zoom)}
    function center(p){let i=controller.transformReference,u=(i%3)/2,v=Math.floor(i/3)/2;return mix(mix(p[0],p[3],u),mix(p[12],p[15],u),v)}
    function corners(p){return [p[0],p[3],p[15],p[12]]}
    function mix(a,b,t){return Qt.point(a.x+(b.x-a.x)*t,a.y+(b.y-a.y)*t)}
    function fill(q){let out=[];for(let i=0;i<16;i++)out.push(mix(mix(q[0],q[1],(i%4)/3),mix(q[3],q[2],(i%4)/3),Math.floor(i/4)/3));return out}
    function nodes(p){if(controller.transformWarp)return p;let q=corners(p);return [q[0],mix(q[0],q[1],.5),q[1],mix(q[1],q[2],.5),q[2],mix(q[2],q[3],.5),q[3],mix(q[3],q[0],.5)]}
    function translate(p,dx,dy){return p.map(v=>Qt.point(v.x+dx,v.y+dy))}
    function affine(p,c,a,b,d,e){return p.map(v=>Qt.point(c.x+a*(v.x-c.x)+b*(v.y-c.y),c.y+d*(v.x-c.x)+e*(v.y-c.y)))}
    function contains(q,p){let inside=false;for(let i=0,j=3;i<4;j=i++){if((q[i].y>p.y)!==(q[j].y>p.y) && p.x<(q[j].x-q[i].x)*(p.y-q[i].y)/(q[j].y-q[i].y)+q[i].x)inside=!inside}return inside}
    function hit(local){let n=nodes(points);for(let i=0;i<n.length;i++){let p=screen(n[i]);if(Math.hypot(local.x-p.x,local.y-p.y)<=8)return i}return -1}
    function mapped(u,v) {
        function node(x,y,axis) {function row(yy){if(x<0)return 2*points[yy*4][axis]-points[yy*4+1][axis];if(x>3)return 2*points[yy*4+3][axis]-points[yy*4+2][axis];return points[yy*4+x][axis]}if(y<0)return 2*row(0)-row(1);if(y>3)return 2*row(3)-row(2);return row(y)}
        function cubic(a,t){return .5*(2*a[1]+(-a[0]+a[2])*t+(2*a[0]-5*a[1]+4*a[2]-a[3])*t*t+(-a[0]+3*a[1]-3*a[2]+a[3])*t*t*t)}
        const ix=Math.min(2,Math.floor(u*3)),iy=Math.min(2,Math.floor(v*3)),tx=u*3-ix,ty=v*3-iy;
        function axis(key){let rows=[];for(let r=0;r<4;r++){let a=[];for(let k=0;k<4;k++)a.push(node(ix+k-1,iy+r-1,key));rows.push(cubic(a,tx))}return cubic(rows,ty)}
        return Qt.point(axis("x"),axis("y"));
    }
    function move(local,modifiers){
        let p=doc(local),s=startPoints,c=center(s),dx=p.x-pressPoint.x,dy=p.y-pressPoint.y,out;
        if(rotating){let angle=Math.atan2(p.y-c.y,p.x-c.x)-Math.atan2(pressPoint.y-c.y,pressPoint.x-c.x);if(modifiers&Qt.ShiftModifier)angle=Math.round(angle/(Math.PI/12))*(Math.PI/12);out=affine(s,c,Math.cos(angle),-Math.sin(angle),Math.sin(angle),Math.cos(angle))}
        else if(handle<0){if(modifiers&Qt.ShiftModifier){if(Math.abs(dx)>Math.abs(dy))dy=0;else dx=0}out=translate(s,dx,dy)}
        else if(controller.transformWarp){out=s.slice();out[handle]=Qt.point(s[handle].x+dx,s[handle].y+dy)}
        else {
            let q=corners(s),ctrl=(modifiers&Qt.ControlModifier)||mode===3||mode===4||mode===5,alt=(modifiers&Qt.AltModifier)||mode===5,shift=(modifiers&Qt.ShiftModifier)||mode===3||mode===5;
            if(ctrl){
                if(handle%2===0){let i=handle/2;if(alt&&shift){if(Math.abs(dx)>=Math.abs(dy)){let j=i%2===0?(i+1)%4:(i+3)%4;q[i]=Qt.point(q[i].x+dx,q[i].y);q[j]=Qt.point(q[j].x-dx,q[j].y)}else{let j=3-i;q[i]=Qt.point(q[i].x,q[i].y+dy);q[j]=Qt.point(q[j].x,q[j].y-dy)}}else if(shift){let j;if(Math.abs(dx)>=Math.abs(dy)){dy=0;j=i%2===0?(i+1)%4:(i+3)%4}else{dx=0;j=3-i}q[i]=Qt.point(q[i].x+dx,q[i].y+dy);q[j]=Qt.point(q[j].x+dx,q[j].y+dy)}else q[i]=Qt.point(q[i].x+dx,q[i].y+dy)}
                else {let i=Math.floor(handle/2),j=(i+1)%4;let a=q[j].x-q[i].x,b=q[j].y-q[i].y;if(shift){let t=(dx*a+dy*b)/(a*a+b*b);dx=t*a;dy=t*b}q[i]=Qt.point(q[i].x+dx,q[i].y+dy);q[j]=Qt.point(q[j].x+dx,q[j].y+dy)}out=fill(q);
            } else {
                let n=nodes(s),anchor=alt?c:n[(handle+4)%8],ex=Qt.point(q[1].x-q[0].x,q[1].y-q[0].y),ey=Qt.point(q[3].x-q[0].x,q[3].y-q[0].y),det=ex.x*ey.y-ex.y*ey.x;
                let vx=n[handle].x-anchor.x,vy=n[handle].y-anchor.y,ux=(vx*ey.y-vy*ey.x)/det,uy=(ex.x*vy-ex.y*vx)/det;
                let tx=((p.x-anchor.x)*ey.y-(p.y-anchor.y)*ey.x)/det,ty=(ex.x*(p.y-anchor.y)-ex.y*(p.x-anchor.x))/det;
                let sx=Math.abs(ux)>.0001?tx/ux:1,sy=Math.abs(uy)>.0001?ty/uy:1;
                if(handle%2===0 && (linked!==!!shift)){if(Math.abs(sx-1)>Math.abs(sy-1))sy=sx;else sx=sy}
                if(Math.abs(sx)<.0001 || Math.abs(sy)<.0001)return;
                out=s.map(v=>{let x=v.x-anchor.x,y=v.y-anchor.y,u=(x*ey.y-y*ey.x)/det,w=(ex.x*y-ex.y*x)/det;return Qt.point(anchor.x+ex.x*u*sx+ey.x*w*sy,anchor.y+ex.y*u*sx+ey.y*w*sy)});
            }
        }
        controller.updateTransform(out,controller.transformWarp,controller.transformInterpolation);
    }
    Canvas {
        id:outline;anchors.fill:parent
        Connections {target:root.controller;function onTransformChanged(){outline.requestPaint()}function onStateChanged(){outline.requestPaint()}}
        Connections {target:root.canvasView;function onViewChanged(){outline.requestPaint()}}
        onPaint:{let ctx=getContext("2d");ctx.reset();if(!root.visible || root.points.length!==16)return;ctx.strokeStyle="#6598ff";ctx.lineWidth=1;let p=root.points;
            if(root.controller.transformWarp){for(let axis=0;axis<2;axis++)for(let i=0;i<4;i++){ctx.beginPath();for(let j=0;j<=48;j++){let v=root.screen(root.mapped(axis?i/3:j/48,axis?j/48:i/3));if(j===0)ctx.moveTo(v.x,v.y);else ctx.lineTo(v.x,v.y)}ctx.stroke()}}
            else {let q=root.corners(p);ctx.beginPath();for(let i=0;i<4;i++){let v=root.screen(q[i]);if(i===0)ctx.moveTo(v.x,v.y);else ctx.lineTo(v.x,v.y)}ctx.closePath();ctx.stroke()}
            let ref=root.screen(root.center(p));ctx.beginPath();ctx.arc(ref.x,ref.y,4,0,Math.PI*2);ctx.moveTo(ref.x-6,ref.y);ctx.lineTo(ref.x+6,ref.y);ctx.moveTo(ref.x,ref.y-6);ctx.lineTo(ref.x,ref.y+6);ctx.stroke();
            let n=root.nodes(p);for(let v of n){v=root.screen(v);ctx.fillStyle=root.controller.transformWarp?"#6598ff":"white";if(root.controller.transformWarp){ctx.beginPath();ctx.arc(v.x,v.y,3.5,0,Math.PI*2);ctx.fill();ctx.stroke()}else{ctx.fillRect(v.x-4,v.y-4,8,8);ctx.strokeRect(v.x-4,v.y-4,8,8)}}
        }
    }
    MouseArea {
        id:mouse;anchors.fill:parent;acceptedButtons:Qt.LeftButton|Qt.RightButton;hoverEnabled:true
        enabled:root.visible && !root.canvasView.spacePanning && root.points.length===16
        cursorShape: {
            if(root.points.length!==16)return Qt.ArrowCursor;
            const h=root.dragging?root.handle:root.hit(Qt.point(mouseX,mouseY));
            if(root.controller.transformWarp)return h>=0?Qt.SizeAllCursor:Qt.OpenHandCursor;
            if(h>=0)return h%4===1?Qt.SizeVerCursor:h%4===3?Qt.SizeHorCursor:h%4===0?Qt.SizeFDiagCursor:Qt.SizeBDiagCursor;
            return root.contains(root.corners(root.points),root.doc(Qt.point(mouseX,mouseY)))?Qt.SizeAllCursor:Qt.CrossCursor;
        }
        onPressed:event=>{if(event.button===Qt.RightButton){context.popup();return}root.canvasView.forceActiveFocus();root.startPoints=root.points.slice();root.pressPoint=root.doc(Qt.point(event.x,event.y));root.handle=root.hit(Qt.point(event.x,event.y));root.rotating=root.mode===2 || root.handle<0 && !root.contains(root.corners(root.startPoints),root.pressPoint);root.dragging=true}
        onPositionChanged:event=>{if(root.dragging)root.move(Qt.point(event.x,event.y),event.modifiers)}
        onReleased:event=>{if(root.dragging){root.dragging=false;root.controller.checkpointTransform()}}
        onCanceled:{if(root.dragging){root.dragging=false;root.controller.updateTransform(root.startPoints,root.controller.transformWarp,root.controller.transformInterpolation);root.controller.checkpointTransform()}}
        onClicked:event=>{if(event.button===Qt.LeftButton && !root.dragging && root.startPoints.length===16 && !root.contains(root.corners(root.startPoints),root.pressPoint) && Math.hypot(root.doc(Qt.point(event.x,event.y)).x-root.pressPoint.x,root.doc(Qt.point(event.x,event.y)).y-root.pressPoint.y)<.5)root.controller.finishTransform(true)}
        onDoubleClicked:root.controller.finishTransform(true)
    }
    Repeater {model:[{key:"Left",dx:-1,dy:0},{key:"Right",dx:1,dy:0},{key:"Up",dx:0,dy:-1},{key:"Down",dx:0,dy:1}];delegate:Item {required property var modelData;Shortcut {sequence:modelData.key;context:Qt.ApplicationShortcut;enabled:root.visible && root.canvasView.activeFocus && !root.dragging;onActivated:{root.controller.updateTransform(root.translate(root.points,modelData.dx,modelData.dy),root.controller.transformWarp,root.controller.transformInterpolation);root.controller.checkpointTransform()}}Shortcut {sequence:"Shift+"+modelData.key;context:Qt.ApplicationShortcut;enabled:root.visible && root.canvasView.activeFocus && !root.dragging;onActivated:{root.controller.updateTransform(root.translate(root.points,10*modelData.dx,10*modelData.dy),root.controller.transformWarp,root.controller.transformInterpolation);root.controller.checkpointTransform()}}}}
    GlassMenu {id:context;objectName:"transformContextMenu";
        Repeater {model:["自由变换","缩放","旋转","斜切","扭曲","透视"];delegate:GlassMenuItem{required property int index;required property string modelData;text:modelData;checkable:true;checked:root.mode===index && !root.controller.transformWarp;onTriggered:{root.mode=index;if(root.controller.transformWarp){root.controller.updateTransform(root.points,false,root.controller.transformInterpolation);root.controller.checkpointTransform()}}}}
        MenuSeparator {}
GlassMenuItem {text:root.controller.transformWarp?"自由变换":"变形";onTriggered:{root.controller.updateTransform(root.points,!root.controller.transformWarp,root.controller.transformInterpolation);root.controller.checkpointTransform()}}GlassMenuItem {text:"水平翻转";onTriggered:{root.controller.updateTransform(root.affine(root.points,root.center(root.points),-1,0,0,1),root.controller.transformWarp,root.controller.transformInterpolation);root.controller.checkpointTransform()}}GlassMenuItem {text:"垂直翻转";onTriggered:{root.controller.updateTransform(root.affine(root.points,root.center(root.points),1,0,0,-1),root.controller.transformWarp,root.controller.transformInterpolation);root.controller.checkpointTransform()}}MenuSeparator{}GlassMenuItem{text:"取消";onTriggered:root.controller.finishTransform(false)}GlassMenuItem{text:"确定";onTriggered:root.controller.finishTransform(true)}}
}
