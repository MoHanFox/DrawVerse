import QtQuick
Item {
    id: root
    property var points: [[0,0],[1,1]]
    property var modes: []
    property var bins: []
    property color lineColor: "#d7d7d7"
    readonly property var samples: PaintClient.pressureCurveSamples(points,true,modes)
    property int selected: 0
    signal edited(var points,var modes)
    function copy(){return points.map(p=>[p[0],p[1]])}
    function copyModes(){return points.map((p,i)=>modes.length===points.length?modes[i]:true)}
    function move(index,x,y){let p=copy();if(index===0 || index===p.length-1)x=p[index][0];else x=Math.max(p[index-1][0]+.0001,Math.min(p[index+1][0]-.0001,x));p[index]=[x,y];edited(p,copyModes())}
    onPointsChanged: graph.requestPaint()
    onModesChanged: graph.requestPaint()
    onSamplesChanged: graph.requestPaint()
    onLineColorChanged: graph.requestPaint()
    Rectangle {x:0;y:5;width:10;height:parent.height-24;gradient:Gradient {GradientStop {position:0;color:"white"} GradientStop {position:1;color:"black"}}}
    Rectangle {x:24;y:parent.height-10;width:parent.width-29;height:10;rotation:0;gradient:Gradient {orientation:Gradient.Horizontal;GradientStop {position:0;color:"black"} GradientStop {position:1;color:"white"}}}
    AdjustmentHistogram {id:hist;x:23;y:4;width:parent.width-27;height:parent.height-27;bins:root.bins}
    Canvas {
        id:graph;anchors.fill:hist
        onWidthChanged:requestPaint();onHeightChanged:requestPaint()
        function px(v){return 1+v*(width-2)}
        function py(v){return height-1-v*(height-2)}
        onPaint:{const c=getContext("2d");c.reset();c.strokeStyle="#1e1e1e";c.lineWidth=1
            for(let i=1;i<4;i++){c.beginPath();c.moveTo(px(i/4),0);c.lineTo(px(i/4),height);c.moveTo(0,py(i/4));c.lineTo(width,py(i/4));c.stroke()}
            c.strokeStyle=root.lineColor;c.lineWidth=1.6;c.beginPath()
            for(let i=0;i<root.samples.length;i++){let x=px(i/(root.samples.length-1)),y=py(root.samples[i]);if(i===0)c.moveTo(x,y);else c.lineTo(x,y)}c.stroke()
            for(let i=0;i<root.points.length;i++){let p=root.points[i];c.fillStyle=i===root.selected?"#999999":"#333333";c.fillRect(px(p[0])-3.5,py(p[1])-3.5,7,7);c.strokeRect(px(p[0])-3.5,py(p[1])-3.5,7,7)}}
        MouseArea {
            objectName:"adjustmentCurveDrag";anchors.fill:parent;preventStealing:true;property bool moving:false
            function nearest(mouse){let at=-1,d=144;for(let i=0;i<root.points.length;i++){let dx=mouse.x-graph.px(root.points[i][0]),dy=mouse.y-graph.py(root.points[i][1]);if(dx*dx+dy*dy<d){d=dx*dx+dy*dy;at=i}}return at}
            onPressed:mouse=>{let at=nearest(mouse);moving=at>=0;if(moving){root.selected=at;root.forceActiveFocus();graph.requestPaint();if(mouse.modifiers&Qt.ControlModifier){let m=root.copyModes();m[at]=!m[at];root.edited(root.copy(),m);moving=false}}}
            onPositionChanged:mouse=>{if(pressed && moving)root.move(root.selected,Math.max(0,Math.min(1,(mouse.x-1)/(width-2))),Math.max(0,Math.min(1,1-(mouse.y-1)/(height-2))))}
            onReleased:moving=false
            onCanceled:moving=false
            onDoubleClicked:mouse=>{if(nearest(mouse)>=0 || root.points.length>=17)return;let x=Math.max(.0001,Math.min(.9999,mouse.x/width)),y=Math.max(0,Math.min(1,1-mouse.y/height)),p=root.copy(),m=root.copyModes(),i=0;while(i<p.length && p[i][0]<x)i++;if(i===0 || i===p.length || x-p[i-1][0]<.0001 || p[i][0]-x<.0001)return;p.splice(i,0,[x,y]);m.splice(i,0,true);root.selected=i;root.edited(p,m)}
        }
    }
    Keys.onDeletePressed:{if(selected<=0 || selected>=points.length-1)return;let p=copy(),m=copyModes();p.splice(selected,1);m.splice(selected,1);selected--;edited(p,m)}
}
