import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../controls"
import "../theme"

ColumnLayout {
    id:root;objectName:prefix+"Editor"
    required property var library
    required property string target
    property var curve:[]
    property bool showMinimum:true
    property real graphHeight:84
    property real minimum:0
    property bool smooth:true
    property var nodeModes:[]
    property var setControl:null
    property var resetControl:null
    property string caption:"速度响应曲线"
    property string curveHelp:"拖动控制点；双击增加控制点。横轴为归一化速度，纵轴为输出比例。"
    property string minimumHelp:"最低比例与曲线输出组合。"
    property string prefix:target
    readonly property var points:curve.length>=2?curve:[[0,0],[1,1]]
    readonly property var samples:PaintClient.pressureCurveSamples(points,smooth,nodeModes)
    property int selected:0
    readonly property var selectedPoint:points[Math.min(selected,points.length-1)]
    Layout.fillWidth:true;spacing:4
    function setPressureControl(points,minimum,modes){return setControl?setControl(points,minimum,modes):library.setSpeedControl(target,points,minimum,modes)}
    function resetPressureControl(){return resetControl?resetControl():library.resetSpeedControl(target)}
    function copyModes(){var modes=[];for(var i=0;i<points.length;++i)modes.push(nodeModes.length===points.length?Boolean(nodeModes[i]):true);return modes}
    function toggleNode(index){var modes=copyModes();if(nodeModes.length!==points.length)for(var i=0;i<modes.length;++i)modes[i]=smooth;modes[index]=!modes[index];return setPressureControl(copyPoints(),minimum,modes)}
    function copyPoints(){var result=[];for(var i=0;i<points.length;++i)result.push([Number(points[i][0]),Number(points[i][1])]);return result}
    function movePoint(index,x,y) {
        var data=copyPoints()
        if(index===0 || index===data.length-1)x=data[index][0]
        if(index>0 && index<data.length-1){const gap=Math.min(0.0001,(data[index+1][0]-data[index-1][0])/4);x=Math.max(data[index-1][0]+gap,Math.min(data[index+1][0]-gap,x))}
        data[index]=[x,y]
        return setPressureControl(data,minimum,copyModes())
    }
    function addPoint(x,y) {
        if(points.length>=17 || x<=0 || x>=1)return
        var data=copyPoints(),index=0
        while(index<data.length && data[index][0]<x)index++
        var modes=copyModes();modes.splice(index,0,true)
        data.splice(index,0,[x,y])
        if(setPressureControl(data,minimum,modes))selected=index
    }
    function addMiddle(){var i=Math.min(selected,points.length-2);addPoint((points[i][0]+points[i+1][0])/2,(points[i][1]+points[i+1][1])/2)}
    function removePoint(){if(selected<=0 || selected>=points.length-1)return;var data=copyPoints(),modes=copyModes();data.splice(selected,1);modes.splice(selected,1);if(setPressureControl(data,minimum,modes))selected=Math.max(0,selected-1)}
    Label {text:root.caption;color:Theme.text;font.pixelSize:10}
    Canvas {
        id:graph;objectName:root.prefix+"Curve";Layout.fillWidth:true;Layout.preferredHeight:root.graphHeight
        readonly property real margin:7
        function px(x){return margin+x*(width-2*margin)}
        function py(y){return height-margin-y*(height-2*margin)}
        function inputX(x){return Math.max(0,Math.min(1,(x-margin)/Math.max(1,width-2*margin)))}
        function inputY(y){return Math.max(0,Math.min(1,(height-margin-y)/Math.max(1,height-2*margin)))}
        onPaint:{
            var ctx=getContext("2d");ctx.reset();ctx.fillStyle=Theme.input;ctx.fillRect(0,0,width,height)
            ctx.strokeStyle=Theme.line;ctx.lineWidth=1
            for(var i=0;i<=4;++i){ctx.beginPath();ctx.moveTo(px(i/4),py(0));ctx.lineTo(px(i/4),py(1));ctx.stroke();ctx.beginPath();ctx.moveTo(px(0),py(i/4));ctx.lineTo(px(1),py(i/4));ctx.stroke()}
            ctx.strokeStyle=Theme.text;ctx.lineWidth=1.5;ctx.beginPath()
            for(i=0;i<root.samples.length;++i){var x=i/(root.samples.length-1),y=root.samples[i];if(i===0)ctx.moveTo(px(x),py(y));else ctx.lineTo(px(x),py(y))}ctx.stroke()
            for(i=0;i<root.points.length;++i){var p=root.points[i];ctx.fillStyle=i===root.selected?"#4e96ff":Theme.text;ctx.beginPath();if(root.nodeModes.length===root.points.length?root.nodeModes[i]:root.smooth)ctx.arc(px(p[0]),py(p[1]),3,0,Math.PI*2);else ctx.rect(px(p[0])-3,py(p[1])-3,6,6);ctx.fill()}
        }
        onWidthChanged:requestPaint()
        onHeightChanged:requestPaint()
        MouseArea {
            objectName:root.prefix+"DragArea";anchors.fill:parent;hoverEnabled:true;preventStealing:true;property bool moving:false
            ToolTip.delay:1000;ToolTip.visible:containsMouse;ToolTip.text:root.curveHelp+" Ctrl+点击节点切换直线/曲线；圆点为曲线，方点为直线。"
            onPressed:mouse=>{var nearest=-1,distance=100;for(var i=0;i<root.points.length;++i){var dx=mouse.x-graph.px(root.points[i][0]),dy=mouse.y-graph.py(root.points[i][1]);var d=dx*dx+dy*dy;if(d<distance){distance=d;nearest=i}}moving=nearest>=0;if(moving){root.selected=nearest;if(mouse.modifiers & Qt.ControlModifier){root.toggleNode(nearest);moving=false}}}
            onPositionChanged:mouse=>{if(pressed && moving)root.movePoint(root.selected,graph.inputX(mouse.x),graph.inputY(mouse.y))}
            onReleased:moving=false
            onCanceled:moving=false
            onDoubleClicked:mouse=>{if(!(mouse.modifiers & Qt.ControlModifier))root.addPoint(graph.inputX(mouse.x),graph.inputY(mouse.y))}
        }
    }
    RowLayout {
        Layout.fillWidth:true;spacing:3
        IconButton {objectName:root.prefix+"Add";glyph:"plus";implicitWidth:22;implicitHeight:20;enabled:root.points.length<17;tooltip:"在选中点后的区间增加控制点";onClicked:root.addMiddle()}
        IconButton {objectName:root.prefix+"Delete";glyph:"trash";implicitWidth:22;implicitHeight:20;enabled:root.selected>0 && root.selected<root.points.length-1;tooltip:"删除选中的内部控制点";onClicked:root.removePoint()}
        Button {objectName:root.prefix+"Reset";text:"默认";implicitHeight:20;Layout.fillWidth:true;onClicked:{root.resetPressureControl();root.selected=0}}
    }
    RowLayout {
        Layout.fillWidth:true;spacing:3
        Label {text:"X";color:Theme.muted;font.pixelSize:9}
        CompactTextField {objectName:root.prefix+"X";Layout.fillWidth:true;implicitHeight:20;text:(root.selectedPoint[0]*100).toFixed(3);enabled:root.selected>0 && root.selected<root.points.length-1;validator:DoubleValidator {bottom:0;top:100;decimals:3;locale:"C"} onEditingFinished:root.movePoint(root.selected,Number(text)/100,root.selectedPoint[1])}
        Label {text:"Y";color:Theme.muted;font.pixelSize:9}
        CompactTextField {objectName:root.prefix+"Y";Layout.fillWidth:true;implicitHeight:20;text:(root.selectedPoint[1]*100).toFixed(3);validator:DoubleValidator {bottom:0;top:100;decimals:3;locale:"C"} onEditingFinished:root.movePoint(root.selected,root.selectedPoint[0],Number(text)/100)}
    }
    RowLayout {
        Layout.fillWidth:true
        visible:root.showMinimum
        Label {text:"最低比例";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
        CompactSpinBox {objectName:root.prefix+"Minimum";from:0;to:100;editable:true;implicitWidth:66;implicitHeight:20;value:Math.round(root.minimum*100);onValueModified:root.setPressureControl(root.points,value/100,root.copyModes());tooltip:root.minimumHelp}
        Label {text:"%";color:Theme.muted;font.pixelSize:9}
    }
    Connections {target:root.library;function onSettingsChanged(){root.selected=Math.min(root.selected,root.points.length-1);graph.requestPaint()}}
    onSamplesChanged:graph.requestPaint()
    onSelectedChanged:graph.requestPaint()
    onCurveChanged:{root.selected=Math.min(root.selected,root.points.length-1);graph.requestPaint()}
    Component.onCompleted:graph.requestPaint()
}
