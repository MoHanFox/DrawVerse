import QtQuick
import QtQuick.Controls
import "."

Rectangle {
    id: root
    objectName: "toolStrip"
    property var canvasView
    color: Theme.strip
    implicitWidth: 36; implicitHeight: 270
    Item {
        objectName: "toolStripGrip"
        width: parent.width; height: 10
        Rectangle {anchors.centerIn:parent;width:12;height:1;color:Theme.muted}
        MouseArea {
            anchors.fill:parent;cursorShape:Qt.SizeAllCursor
            property point origin
            onPressed:mouse=>origin=Qt.point(mouse.x,mouse.y)
            onPositionChanged:mouse=>{if(pressed && Math.abs(mouse.x-origin.x)+Math.abs(mouse.y-origin.y)>8)Qt.callLater(()=>Workspace.beginToolStripDrag())}
            onDoubleClicked:{if(Workspace.toolsFloating)Workspace.dockToolStrip("drawverse-tools-v1");else Workspace.floatToolStrip(mapToGlobal(20,20).x,mapToGlobal(20,20).y)}
            ToolTip.visible:containsMouse;ToolTip.delay:600;ToolTip.text:"拖动工具条 · 双击浮动 / 归位";hoverEnabled:true
        }
    }
    Column {
        anchors.horizontalCenter:parent.horizontalCenter;anchors.top:parent.top;anchors.topMargin:12;spacing:4
        IconButton {objectName:"moveLayerTool";width:26;height:26;glyph:"move";checkable:true;checked:PaintClient.moveTool;onClicked:PaintClient.moveTool=true;tooltip:"移动图层 V"}
        IconButton {objectName:"brushTool";width:26;height:26;glyph:"brush";checkable:true;checked:!PaintClient.eraser && !PaintClient.moveTool;onClicked:PaintClient.eraser=false;tooltip:"画笔 B"}
        IconButton {objectName:"eraserTool";width:26;height:26;glyph:"eraser";checkable:true;checked:PaintClient.eraser && !PaintClient.moveTool;onClicked:PaintClient.eraser=true;tooltip:"橡皮擦 E"}
        Rectangle {width:24;height:1;color:Theme.line;anchors.horizontalCenter:parent.horizontalCenter}
        IconButton {width:26;height:26;glyph:"undo";enabled:PaintClient.undoDepth>0 && !PaintClient.drawing;onClicked:PaintClient.undo();tooltip:"撤销 Ctrl+Z"}
        IconButton {width:26;height:26;glyph:"redo";enabled:PaintClient.redoDepth>0 && !PaintClient.drawing;onClicked:PaintClient.redo();tooltip:"重做 Ctrl+Shift+Z"}
        IconButton {width:26;height:26;glyph:"fit";onClicked:root.canvasView.fitToView();tooltip:"适合窗口 F"}
    }
    Rectangle {width:22;height:22;color:PaintClient.brushColor;border.color:Theme.muted;anchors.bottom:parent.bottom;anchors.bottomMargin:12;anchors.horizontalCenter:parent.horizontalCenter}
}
