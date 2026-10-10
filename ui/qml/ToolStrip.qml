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
        IconButton {objectName:"moveLayerTool";width:26;height:26;glyph:"move";checkable:true;checked:PaintClient.moveTool;onClicked:PaintClient.activateMoveTool();tooltip:"移动图层 V"}
        // Rectangle and ellipse stay one slot with a right-click shape picker. The lasso and the
        // magic wand are separate tools, not shapes of the marquee, so each gets its own slot.
        IconButton {
            id: selectionToolButton
            objectName:"selectionTool";width:26;height:26
            glyph:PaintClient.selectionTool===2?"ellipseSelection":"rectangleSelection"
            checkable:true;checked:PaintClient.selectionTool===1||PaintClient.selectionTool===2
            onClicked:PaintClient.activateMarquee()
            tooltip:"选框工具 M · 右键切换矩形/椭圆"
            hasVariants:true
            function openVariants() {
                variants.toolTitle="选框工具"
                const rows=[]
                rows.push({id:"rectangle",label:"矩形选框",icon:"rectangleSelection",pending:false,selected:PaintClient.selectionTool===1,apply:function(){PaintClient.selectionTool=1}})
                rows.push({id:"ellipse",label:"椭圆选框",icon:"ellipseSelection",pending:false,selected:PaintClient.selectionTool===2,apply:function(){PaintClient.selectionTool=2}})
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
        // View rotation: rotates what the user sees without changing a single document pixel.
        IconButton {objectName:"rotateViewTool";width:26;height:26;glyph:"redo";checkable:true;checked:PaintClient.rotateViewTool;onClicked:PaintClient.activateRotateView();tooltip:"旋转视图 R · 只转视图不改图像，Alt 点击复位"}
        Rectangle {width:24;height:1;color:Theme.line;anchors.horizontalCenter:parent.horizontalCenter}
        IconButton {objectName:"lassoTool";width:26;height:26;glyph:"lassoSelection";checkable:true;checked:PaintClient.selectionTool===3;onClicked:PaintClient.activateLasso();tooltip:"套索 L · 自由手绘选区"}
        IconButton {objectName:"wandTool";width:26;height:26;glyph:"wand";checkable:true;checked:PaintClient.selectionTool===4;onClicked:PaintClient.activateWand();tooltip:"魔棒 W · 点击选取相近颜色"}
        // Every slot clears the others: leaving a second tool active made the strip show the brush
        // while the bucket still owned the canvas.
        IconButton {objectName:"brushTool";width:26;height:26;glyph:"brush";checkable:true;checked:!PaintClient.eraser && !PaintClient.moveTool && !PaintClient.selectionTool && !PaintClient.bucketTool;onClicked:PaintClient.activateBrush();tooltip:"画笔 B"}
        IconButton {objectName:"eraserTool";width:26;height:26;glyph:"eraser";checkable:true;checked:PaintClient.eraser && !PaintClient.moveTool && !PaintClient.selectionTool && !PaintClient.bucketTool;onClicked:PaintClient.activateEraser();tooltip:"橡皮擦 E"}
        // The bucket is a painting tool, not a selection shape, so it gets its own slot.
        IconButton {objectName:"bucketTool";width:26;height:26;glyph:"bucket";checkable:true;checked:PaintClient.bucketTool;onClicked:PaintClient.activateBucket();tooltip:"油漆桶 G · 点击填充相近区域"}
        Rectangle {width:24;height:1;color:Theme.line;anchors.horizontalCenter:parent.horizontalCenter}
        IconButton {width:26;height:26;glyph:"undo";enabled:PaintClient.undoDepth>0 && !PaintClient.drawing;onClicked:PaintClient.undo();tooltip:"撤销 Ctrl+Z"}
        IconButton {width:26;height:26;glyph:"redo";enabled:PaintClient.redoDepth>0 && !PaintClient.drawing;onClicked:PaintClient.redo();tooltip:"重做 Ctrl+Shift+Z"}
    }
    Rectangle {width:22;height:22;color:PaintClient.brushColor;border.color:Theme.line;anchors.bottom:parent.bottom;anchors.bottomMargin:12;anchors.horizontalCenter:parent.horizontalCenter}
}
