import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../controls"
import "../theme"
Flickable {
    id:root;objectName:"transformOptionsBar"
    required property var controller
    readonly property bool linked:controller.transformLinked
    contentWidth:row.implicitWidth;contentHeight:height;clip:true;flickableDirection:Flickable.HorizontalFlick
    property bool relative:false
    property point relativeOrigin:Qt.point(0,0)
    readonly property real skewH:controller.transformSkewH
    readonly property real skewV:controller.transformSkewV
    readonly property var p:controller.transformPoints
    readonly property var initial:controller.transformInitialPoints
    function center(a){let i=controller.transformReference,u=(i%3)/2,v=Math.floor(i/3)/2;let top=Qt.point(a[0].x+(a[3].x-a[0].x)*u,a[0].y+(a[3].y-a[0].y)*u),bottom=Qt.point(a[12].x+(a[15].x-a[12].x)*u,a[12].y+(a[15].y-a[12].y)*u);return Qt.point(top.x+(bottom.x-top.x)*v,top.y+(bottom.y-top.y)*v)}
    function length(a,b){return Math.hypot(a.x-b.x,a.y-b.y)}
    function value(key){if(p.length!==16 || initial.length!==16)return 0;if(key==="SH")return skewH;if(key==="SV")return skewV;if(key==="X")return center(p).x-(relative?relativeOrigin.x:0);if(key==="Y")return center(p).y-(relative?relativeOrigin.y:0);if(key==="W")return length(p[0],p[3])/length(initial[0],initial[3])*100;if(key==="H")return length(p[0],p[12])/length(initial[0],initial[12])*100;if(key==="A")return (Math.atan2(p[3].y-p[0].y,p[3].x-p[0].x)-Math.atan2(initial[3].y-initial[0].y,initial[3].x-initial[0].x))*180/Math.PI;return 0}
    function apply(key,v){if(!isFinite(v)||p.length!==16)return;let c=center(p),out=p.slice(),current=value(key),angle=0,sx=1,sy=1;
        if(key==="X"||key==="Y")out=out.map(q=>Qt.point(q.x+(key==="X"?v-current:0),q.y+(key==="Y"?v-current:0)));
        else if(key==="A"){angle=(v-current)*Math.PI/180;out=out.map(q=>Qt.point(c.x+Math.cos(angle)*(q.x-c.x)-Math.sin(angle)*(q.y-c.y),c.y+Math.sin(angle)*(q.x-c.x)+Math.cos(angle)*(q.y-c.y)))}
        else if(key==="W"||key==="H"){if(Math.abs(v)<.01)return;sx=key==="W"?v/current:1;sy=key==="H"?v/current:1;if(linked){if(key==="W")sy=sx;else sx=sy}let a=Math.atan2(p[3].y-p[0].y,p[3].x-p[0].x),co=Math.cos(a),si=Math.sin(a);out=out.map(q=>{let x=q.x-c.x,y=q.y-c.y,u=co*x+si*y,w=-si*x+co*y;return Qt.point(c.x+co*u*sx-si*w*sy,c.y+si*u*sx+co*w*sy)})}
        else {let t=Math.tan(v*Math.PI/180)-Math.tan(current*Math.PI/180);if(Math.abs(v)>=90)return;out=out.map(q=>Qt.point(q.x+(key==="SH"?t*(q.y-c.y):0),q.y+(key==="SV"?t*(q.x-c.x):0)))}
        controller.updateTransform(out,controller.transformWarp,controller.transformInterpolation,key==="SH"?v:skewH,key==="SV"?v:skewV);controller.checkpointTransform();
    }
    onVisibleChanged:if(visible)relative=false
    RowLayout {id:row;height:root.height;spacing:6
        Icon{name:"transform";Layout.preferredWidth:17;Layout.preferredHeight:17}
        GridLayout {columns:3;rowSpacing:1;columnSpacing:1;Repeater{model:9;delegate:Rectangle{required property int index;Layout.preferredWidth:5;Layout.preferredHeight:5;color:root.controller.transformReference===index?Theme.accent:Theme.disabled;MouseArea{anchors.fill:parent;onClicked:root.controller.transformReference=parent.index}}}}
        IconButton {glyph:"relativePosition";checkable:true;checked:root.relative;tooltip:"相对当前位置输入 X/Y";onClicked:{root.relative=!root.relative;root.relativeOrigin=root.center(root.p)}}
        Repeater {model:[{key:"X",title:"X:",unit:"px"},{key:"Y",title:"Y:",unit:"px"},{key:"W",title:"W:",unit:"%"},{key:"H",title:"H:",unit:"%"},{key:"A",title:"∠",unit:"°"},{key:"SH",title:"H:",unit:"°"},{key:"SV",title:"V:",unit:"°"}]
            delegate:RowLayout {required property var modelData;spacing:2
                Label{text:modelData.title;color:Theme.muted;font.pixelSize:9}
                CompactTextField {objectName:"transform"+modelData.key;Layout.preferredWidth:52;text:root.value(modelData.key).toFixed(2);validator:DoubleValidator{bottom:-2000000;top:2000000;decimals:2} onEditingFinished:{root.apply(modelData.key,Number(text));focus=false} ToolTip.visible:hovered;ToolTip.delay:1000;ToolTip.text:modelData.key==="X"||modelData.key==="Y"?"参考点位置（文档像素）":modelData.key==="W"||modelData.key==="H"?"相对本次变换起始尺寸；负值翻转":modelData.key==="A"?"相对本次起始角度；Shift 拖动按 15° 吸附":"斜切角度"}
                Label{text:modelData.unit;color:Theme.muted;font.pixelSize:9}
                IconButton{objectName:modelData.key==="W"?"transformLink":"";visible:modelData.key==="W";glyph:"link";checkable:true;checked:root.linked;tooltip:"保持宽高比例；Shift 临时反转";onClicked:root.controller.transformLinked=checked}
            }
        }
        Rectangle{Layout.preferredWidth:1;Layout.preferredHeight:22;color:Theme.line}
        Label{text:"插值:";color:Theme.muted;font.pixelSize:9}
        CompactComboBox {highPriorityMenu:true;objectName:"transformInterpolation";Layout.preferredWidth:120;implicitHeight:20;model:["邻近","两次线性","两次立方","两次立方（较平滑）","两次立方（较锐利）","两次立方（自动）"];currentIndex:controller.transformInterpolation;onActivated:index=>{controller.updateTransform(root.p,controller.transformWarp,index);controller.checkpointTransform()}}
        ToolButton {objectName:"transformWarp";checkable:true;checked:controller.transformWarp;implicitWidth:32;implicitHeight:26;contentItem:Icon{name:"warp"} onClicked:{controller.updateTransform(root.p,!controller.transformWarp,controller.transformInterpolation);controller.checkpointTransform()} ToolTip.visible:hovered;ToolTip.delay:1000;ToolTip.text:"切换自由变换与网格变形"}
        IconButton {objectName:"transformReset";glyph:"undo";tooltip:"重置本次变换";onClicked:{controller.updateTransform(controller.transformInitialPoints,false,controller.transformInterpolation,0,0);controller.checkpointTransform()}}
        IconButton {objectName:"transformCancel";glyph:"close";tooltip:"取消变换（Esc）";onClicked:controller.finishTransform(false)}
        IconButton {objectName:"transformCommit";glyph:"check";tooltip:"确定变换（Enter）";onClicked:controller.finishTransform(true)}
    }
}
