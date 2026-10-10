import QtQuick
import QtQuick.Controls
import "."

Rectangle {
    id: root
    objectName: "toolStrip"
    property var canvasView
    color: Theme.strip
    implicitWidth: 36; implicitHeight: 300
    Item {
        objectName: "toolStripGrip"
        width: parent.width; height: 10
        Rectangle {anchors.centerIn:parent;width:12;height:1;color:Theme.muted}
        MouseArea {
            anchors.fill:parent;cursorShape:Qt.SizeAllCursor
            property point origin
            onPressed:mouse=>origin=Qt.point(mouse.x,mouse.y)
            onPositionChanged:mouse=>{if(pressed && !PaintClient.drawing && Math.abs(mouse.x-origin.x)+Math.abs(mouse.y-origin.y)>8)Qt.callLater(()=>Workspace.beginToolStripDrag())}
            onDoubleClicked:{if(PaintClient.drawing)return;if(Workspace.toolsFloating)Workspace.dockToolStrip("drawverse-tools-v1");else Workspace.floatToolStrip(mapToGlobal(20,20).x,mapToGlobal(20,20).y)}
        }
    }
    Column {
        anchors.horizontalCenter:parent.horizontalCenter;anchors.top:parent.top;anchors.topMargin:12;spacing:4
        IconButton {objectName:"moveLayerTool";width:26;height:26;glyph:"move";checkable:true;checked:PaintClient.moveTool;onClicked:PaintClient.moveTool=true;tooltip:"移动图层 V"}
        // Several shapes share one slot: click activates the current shape, right-click picks another.
        IconButton {
            id: selectionToolButton
            objectName:"selectionTool";width:26;height:26
            glyph:PaintClient.selectionTool===2?"ellipseSelection":PaintClient.selectionTool===3?"lassoSelection":PaintClient.selectionTool>=4?"wand":"rectangleSelection"
            checkable:true;checked:PaintClient.selectionTool>0
            onClicked:PaintClient.selectionTool=PaintClient.selectionTool>0?PaintClient.selectionTool:1
            tooltip:"选区工具 M · 右键切换形态"
            hasVariants:true
            function openVariants() {
                variants.toolTitle="选区工具"
                // Build a real list of entries; the panel looks each one up by index.
                const rows=[]
                rows.push({id:"rectangle",label:"矩形选框",icon:"rectangleSelection",pending:false,selected:PaintClient.selectionTool===1,apply:function(){PaintClient.selectionTool=1}})
                rows.push({id:"ellipse",label:"椭圆选框",icon:"ellipseSelection",pending:false,selected:PaintClient.selectionTool===2,apply:function(){PaintClient.selectionTool=2}})
                rows.push({id:"lasso",label:"套索",icon:"lassoSelection",pending:false,selected:PaintClient.selectionTool===3,apply:function(){PaintClient.selectionTool=3}})
                rows.push({id:"wand",label:"魔棒",icon:"wand",pending:false,selected:PaintClient.selectionTool===4,apply:function(){PaintClient.selectionTool=4}})
                variants.entries=rows
                variants.open()
            }
            TapHandler {
                objectName:"selectionToolVariantArea"
                acceptedButtons:Qt.RightButton
                onTapped:selectionToolButton.openVariants()
            }
            ToolVariantsPanel { id: variants; parent: root; canvasView: root.canvasView }
        }
        IconButton {objectName:"brushTool";width:26;height:26;glyph:"brush";checkable:true;checked:!PaintClient.eraser && !PaintClient.moveTool && !PaintClient.selectionTool && !PaintClient.bucketTool;onClicked:PaintClient.eraser=false;tooltip:"画笔 B"}
        IconButton {objectName:"eraserTool";width:26;height:26;glyph:"eraser";checkable:true;checked:PaintClient.eraser && !PaintClient.moveTool;onClicked:PaintClient.eraser=true;tooltip:"橡皮擦 E"}
        // The bucket is a painting tool, not a selection shape, so it gets its own slot.
        IconButton {objectName:"bucketTool";width:26;height:26;glyph:"bucket";checkable:true;checked:PaintClient.bucketTool;onClicked:PaintClient.bucketTool=true;tooltip:"油漆桶 G · 点击填充相近区域"}
        Rectangle {width:24;height:1;color:Theme.line;anchors.horizontalCenter:parent.horizontalCenter}
        IconButton {width:26;height:26;glyph:"undo";enabled:PaintClient.undoDepth>0 && !PaintClient.drawing;onClicked:PaintClient.undo();tooltip:"撤销 Ctrl+Z"}
        IconButton {width:26;height:26;glyph:"redo";enabled:PaintClient.redoDepth>0 && !PaintClient.drawing;onClicked:PaintClient.redo();tooltip:"重做 Ctrl+Shift+Z"}
    }
    Rectangle {width:22;height:22;color:PaintClient.brushColor;border.color:Theme.line;anchors.bottom:parent.bottom;anchors.bottomMargin:12;anchors.horizontalCenter:parent.horizontalCenter}
}
