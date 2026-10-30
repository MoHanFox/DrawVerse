import QtQuick
import QtQuick.Controls
import "../theme"
Item {
    id:root;objectName:"canvasResizeOverlay"
    required property var controller
    required property var canvasView
    signal finished()
    property int cropX:0
    property int cropY:0
    property int pixelWidth:1
    property int pixelHeight:1
    property int dpi:72
    property int units:0
    property real ratioWidth:0
    property real ratioHeight:0
    property real originalRatio:1
    property bool deletePixels:false
    property bool selectedColor:false
    property bool submitting:false
    property bool inputsValid:true
    property string errorText:""
    readonly property real ratio:ratioWidth>0 && ratioHeight>0?ratioWidth/ratioHeight:originalRatio
    readonly property rect frame:Qt.rect(canvasView.documentRect.x+cropX*canvasView.zoom,canvasView.documentRect.y+cropY*canvasView.zoom,pixelWidth*canvasView.zoom,pixelHeight*canvasView.zoom)
    property var draftHistory:[]
    property int historyIndex:0
    property bool restoring:true
    property bool spaceHeld:false
    readonly property bool canUndo:!submitting && (historyIndex>0 || historyTimer.running)
    readonly property bool canRedo:!submitting && !historyTimer.running && historyIndex+1<draftHistory.length
    function snapshot(){return {cropX:cropX,cropY:cropY,pixelWidth:pixelWidth,pixelHeight:pixelHeight,dpi:dpi,units:units,ratioWidth:ratioWidth,ratioHeight:ratioHeight,originalRatio:originalRatio,deletePixels:deletePixels,selectedColor:selectedColor}}
    function changed(){if(!restoring && visible && !submitting)historyTimer.restart()}
    function checkpoint(){
        historyTimer.stop()
        if(restoring || !draftHistory.length)return
        const next=snapshot()
        if(JSON.stringify(next)===JSON.stringify(draftHistory[historyIndex]))return
        let entries=draftHistory.slice(0,historyIndex+1);entries.push(next)
        if(entries.length>256)entries.splice(1,1)
        draftHistory=entries;historyIndex=entries.length-1
    }
    function restore(index){
        if(submitting || index<0 || index>=draftHistory.length)return
        historyTimer.stop();restoring=true
        const state=draftHistory[index]
        for(const key in state)root[key]=state[key]
        historyIndex=index;restoring=false;errorText=""
    }
    function undo(){checkpoint();if(historyIndex>0)restore(historyIndex-1)}
    function redo(){if(canRedo)restore(historyIndex+1)}
    Timer {id:historyTimer;interval:250;onTriggered:if(!mouse.pressed)root.checkpoint()}
    onCropXChanged:changed()
    onCropYChanged:changed()
    onPixelWidthChanged:changed()
    onPixelHeightChanged:changed()
    onDpiChanged:changed()
    onUnitsChanged:changed()
    onRatioWidthChanged:changed()
    onRatioHeightChanged:changed()
    onOriginalRatioChanged:changed()
    onDeletePixelsChanged:changed()
    onSelectedColorChanged:changed()
    Keys.onPressed:event=>{if(event.key===Qt.Key_Space){spaceHeld=true;event.accepted=true}}
    Keys.onReleased:event=>{if(event.key===Qt.Key_Space){spaceHeld=false;event.accepted=true}}
    onVisibleChanged:if(visible)root.forceActiveFocus();else{spaceHeld=false;historyTimer.stop()}
    function begin(){restoring=true;historyTimer.stop();cropX=0;cropY=0;pixelWidth=controller.documentWidth;pixelHeight=controller.documentHeight;dpi=controller.documentDpi;originalRatio=pixelWidth/pixelHeight;units=0;ratioWidth=0;ratioHeight=0;deletePixels=false;selectedColor=false;submitting=false;errorText="";restoring=false;draftHistory=[snapshot()];historyIndex=0;forceActiveFocus()}
    function setSize(w,h){pixelWidth=Math.max(1,Math.min(1000000,Math.round(w)));pixelHeight=Math.max(1,Math.min(1000000,Math.round(h)))}
    function swap(){if(units===0)setSize(pixelHeight,pixelWidth);else{const a=ratioWidth;ratioWidth=ratioHeight;ratioHeight=a;if(ratioWidth===0 && ratioHeight===0)originalRatio=1/originalRatio;setSize(pixelWidth,pixelWidth/ratio)}}
    function confirm(){if(submitting)return;if(!inputsValid){errorText="请输入有效的尺寸、比例和 DPI";return}if(units===1 && ((ratioWidth===0)!==(ratioHeight===0))){errorText="比例须同时为 0 或同时大于 0";return}errorText="";submitting=true;if(!controller.resizeCanvas(cropX,cropY,pixelWidth,pixelHeight,dpi,deletePixels,selectedColor)){submitting=false;errorText="无法提交调整，请稍后重试"}}
    Connections {target:root.controller
        function onCanvasResizeFinished(success){if(!root.submitting)return;root.submitting=false;if(success)root.finished();else root.errorText=root.controller.lastError.length>0?root.controller.lastError:"调整失败，原画布已保留"}
        function onErrorChanged(){if(root.submitting && root.controller.lastError.length>0){root.submitting=false;root.errorText=root.controller.lastError}}
    }
    Repeater {
        model:{const f=root.frame,d=root.canvasView.documentRect;const l=Math.max(f.x,Math.min(f.x+f.width,d.x)),t=Math.max(f.y,Math.min(f.y+f.height,d.y)),r=Math.max(l,Math.min(f.x+f.width,d.x+d.width)),b=Math.max(t,Math.min(f.y+f.height,d.y+d.height));return [Qt.rect(f.x,f.y,f.width,t-f.y),Qt.rect(f.x,b,f.width,f.y+f.height-b),Qt.rect(f.x,t,l-f.x,b-t),Qt.rect(r,t,f.x+f.width-r,b-t)]}
        Rectangle {required property var modelData;x:modelData.x;y:modelData.y;width:Math.max(0,modelData.width);height:Math.max(0,modelData.height);color:root.selectedColor?root.controller.brushColor:"white"}
    }
    Rectangle {x:0;y:0;width:parent.width;height:Math.max(0,root.frame.y);color:"#66000000"}
    Rectangle {x:0;y:Math.max(0,root.frame.y+root.frame.height);width:parent.width;height:Math.max(0,parent.height-y);color:"#66000000"}
    Rectangle {x:0;y:Math.max(0,root.frame.y);width:Math.max(0,root.frame.x);height:root.frame.height;color:"#66000000"}
    Rectangle {x:Math.max(0,root.frame.x+root.frame.width);y:Math.max(0,root.frame.y);width:Math.max(0,parent.width-x);height:root.frame.height;color:"#66000000"}
    Rectangle {x:root.frame.x;y:root.frame.y;width:root.frame.width;height:root.frame.height;color:"transparent";border.color:"#dddddd";border.width:1
        Repeater {model:mouse.pressed?2:0;Rectangle {required property int index;x:(index+1)*parent.width/3;width:1;height:parent.height;color:"#66ffffff"}}
        Repeater {model:mouse.pressed?2:0;Rectangle {required property int index;y:(index+1)*parent.height/3;height:1;width:parent.width;color:"#66ffffff"}}
        Repeater {model:8;Rectangle {required property int index;readonly property var positions:[[0,0],[.5,0],[1,0],[1,.5],[1,1],[.5,1],[0,1],[0,.5]];x:positions[index][0]*parent.width-width/2;y:positions[index][1]*parent.height-height/2;width:index%2===0?18:(index===1 || index===5?20:4);height:index%2===0?4:(index===3 || index===7?20:4);color:"white"}}
        Repeater {model:4;Rectangle {required property int index;x:index===1 || index===2?parent.width-2:-2;y:index>=2?parent.height-18:-2;width:4;height:20;color:"white"}}
    }
    MouseArea {id:mouse;objectName:"canvasResizeHandles";anchors.fill:parent;acceptedButtons:Qt.LeftButton | Qt.MiddleButton;hoverEnabled:true;enabled:!root.submitting
        property bool navigating:false
        property int handle:-1
        property point start
        property rect bounds
        function hit(p){const f=root.frame;const pts=[[f.x,f.y],[f.x+f.width/2,f.y],[f.x+f.width,f.y],[f.x+f.width,f.y+f.height/2],[f.x+f.width,f.y+f.height],[f.x+f.width/2,f.y+f.height],[f.x,f.y+f.height],[f.x,f.y+f.height/2]];for(let i=0;i<8;i++)if(Math.abs(p.x-pts[i][0])<=12 && Math.abs(p.y-pts[i][1])<=12)return i;return p.x>=f.x && p.x<=f.x+f.width && p.y>=f.y && p.y<=f.y+f.height?8:-1}
        cursorShape:{if(root.spaceHeld || root.canvasView.spacePanning || navigating)return Qt.OpenHandCursor;const h=pressed?handle:hit(Qt.point(mouseX,mouseY));return h===8?Qt.SizeAllCursor:h===1 || h===5?Qt.SizeVerCursor:h===3 || h===7?Qt.SizeHorCursor:h===0 || h===4?Qt.SizeFDiagCursor:h===2 || h===6?Qt.SizeBDiagCursor:Qt.ArrowCursor}
        onPressed:e=>{root.forceActiveFocus();navigating=e.button===Qt.MiddleButton || root.spaceHeld || root.canvasView.spacePanning;root.checkpoint();handle=hit(Qt.point(e.x,e.y));start=Qt.point(e.x,e.y);bounds=Qt.rect(root.cropX,root.cropY,root.pixelWidth,root.pixelHeight)}
        onReleased:{navigating=false;root.checkpoint()}
        onCanceled:{navigating=false;root.checkpoint()}
        onWheel:event=>{if(event.angleDelta.y!==0)root.canvasView.zoomBy(Math.pow(1.0015,event.angleDelta.y));event.accepted=true}
        onPositionChanged:e=>{if(!pressed)return;if(navigating){root.canvasView.panBy(e.x-start.x,e.y-start.y);start=Qt.point(e.x,e.y);return}if(handle<0)return;const dx=Math.round((e.x-start.x)/root.canvasView.zoom),dy=Math.round((e.y-start.y)/root.canvasView.zoom);let l=bounds.x,t=bounds.y,r=l+bounds.width,b=t+bounds.height;
            if(handle===8){root.cropX=Math.max(-1000000,Math.min(1000000,l+dx));root.cropY=Math.max(-1000000,Math.min(1000000,t+dy));return}
            if(handle===0 || handle===6 || handle===7)l=Math.min(r-1,l+dx)
            if(handle===2 || handle===3 || handle===4)r=Math.max(l+1,r+dx)
            if(handle===0 || handle===1 || handle===2)t=Math.min(b-1,t+dy)
            if(handle===4 || handle===5 || handle===6)b=Math.max(t+1,b+dy)
            if(root.units===1){let w=r-l,h=b-t;if(handle===1 || handle===5)w=h*root.ratio;else h=w/root.ratio;if(handle===0 || handle===6 || handle===7)l=r-w;else r=l+w;if(handle<=2)t=b-h;else b=t+h}
            root.cropX=Math.max(-1000000,Math.min(1000000,Math.round(l)));root.cropY=Math.max(-1000000,Math.min(1000000,Math.round(t)));root.setSize(r-l,b-t)
        }
    }
    Rectangle {visible:mouse.pressed;x:Math.min(parent.width-width-8,mouse.mouseX+18);y:Math.min(parent.height-height-8,mouse.mouseY+18);width:116;height:58;radius:3;color:"#e6333333";border.color:"#777777";Text {anchors.centerIn:parent;color:"white";text:"W:  "+root.pixelWidth+" px\nH:  "+root.pixelHeight+" px"}}
    Label {anchors.horizontalCenter:parent.horizontalCenter;anchors.bottom:parent.bottom;anchors.bottomMargin:20;text:root.errorText;color:"#ff7777";visible:text.length>0}
    Shortcut {sequences:["Return","Enter"];enabled:root.visible && !root.submitting;context:Qt.ApplicationShortcut;onActivated:root.confirm()}
    Shortcut {sequence:"Escape";enabled:root.visible && !root.submitting;context:Qt.ApplicationShortcut;onActivated:root.finished()}
}
