import QtQuick
import DrawVerse 1.0
import QtQuick.Window
import QtQuick.Controls
import "../controls"
import "../theme"

Rectangle {
    id: root
    objectName: "toolStrip"
    readonly property var resizeEditor:canvasView && ("canvasResizeEditor" in canvasView)?canvasView.canvasResizeEditor:null
    property var canvasView
    function focusCanvas() {
        if(canvasView) {canvasView.forceActiveFocus();canvasView.Window.window.requestActivate()}
    }
    color: Theme.strip
    implicitWidth: 36; implicitHeight: 380
    Item {
        objectName: "toolStripGrip"
        width: parent.width; height: 10
        Rectangle {anchors.centerIn:parent;width:12;height:1;color:Theme.muted}
        MouseArea {
            anchors.fill:parent;cursorShape:Qt.SizeAllCursor
            property point origin
            onPressed:mouse=>origin=Qt.point(mouse.x,mouse.y)
            onPositionChanged:mouse=>{if(pressed && !PaintClient.drawing && !PaintClient.liquifyActive && Math.abs(mouse.x-origin.x)+Math.abs(mouse.y-origin.y)>8)Qt.callLater(()=>Workspace.beginToolStripDrag())}
            onDoubleClicked:{if(PaintClient.drawing || PaintClient.liquifyActive)return;if(Workspace.toolsFloating)Workspace.dockToolStrip("drawverse-tools-v1");else Workspace.floatToolStrip(mapToGlobal(20,20).x,mapToGlobal(20,20).y)}
        }
    }
    Column {
        anchors.horizontalCenter:parent.horizontalCenter;anchors.top:parent.top;anchors.topMargin:12;spacing:4
        IconButton {objectName:"moveLayerTool";width:26;height:26;glyph:"move";checkable:true;enabled:!PaintClient.liquifyActive;checked:!PaintClient.liquifyActive && PaintClient.moveTool;onClicked:{PaintClient.activateMoveTool();root.focusCanvas()} tooltip:"移动图层 V"}
        // Rectangle and ellipse stay one slot with a right-click shape picker. The lasso and the
        // magic wand are separate tools, not shapes of the marquee, so each gets its own slot.
        IconButton {
            id: selectionToolButton
            objectName:"selectionTool";width:26;height:26
            glyph:PaintClient.marqueeShape===2?"ellipseSelection":"rectangleSelection"
            checkable:true;enabled:!PaintClient.liquifyActive;checked:!PaintClient.liquifyActive && (PaintClient.selectionTool===1||PaintClient.selectionTool===2)
            onClicked:{PaintClient.activateMarquee();root.focusCanvas()}
            tooltip:"选框工具 · 右键切换矩形/椭圆"
            hasVariants:true
            function openVariants() {
                variants.toolTitle="选框工具"
                const rows=[]
                rows.push({id:"rectangle",label:"矩形选框",icon:"rectangleSelection",pending:false,selected:PaintClient.marqueeShape===1,apply:function(){PaintClient.selectionTool=1}})
                rows.push({id:"ellipse",label:"椭圆选框",icon:"ellipseSelection",pending:false,selected:PaintClient.marqueeShape===2,apply:function(){PaintClient.selectionTool=2}})
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
        IconButton {
            id:lassoButton;objectName:"lassoTool";width:26;height:26
            glyph:PaintClient.lassoShape===4?"polygonSelection":"lassoSelection"
            checkable:true;enabled:!PaintClient.liquifyActive;checked:!PaintClient.liquifyActive && (PaintClient.selectionTool===3||PaintClient.selectionTool===4);hasVariants:true
            onClicked:{PaintClient.activateLasso();root.focusCanvas()} tooltip:"套索工具 " + ShortcutSettings.bindings.lasso + " · 切换子类 " + ShortcutSettings.bindings.cycleLasso + " / 右键"
            function openVariants() {
                lassoVariants.toolTitle="套索工具"
                lassoVariants.entries=[
                    {id:"lasso",label:"套索",icon:"lassoSelection",pending:false,selected:PaintClient.lassoShape===3,apply:function(){PaintClient.selectionTool=3}},
                    {id:"polygon",label:"多边形套索",icon:"polygonSelection",pending:false,selected:PaintClient.lassoShape===4,apply:function(){PaintClient.selectionTool=4}}
                ]
                lassoVariants.open()
            }
            TapHandler {acceptedButtons:Qt.RightButton;onTapped:lassoButton.openVariants()}
            ToolVariantsPanel {id:lassoVariants;objectName:"lassoVariantPanel";parent:root;canvasView:root.canvasView}
        }
        IconButton {objectName:"wandTool";width:26;height:26;glyph:"wand";checkable:true;enabled:!PaintClient.liquifyActive;checked:!PaintClient.liquifyActive && PaintClient.selectionTool===5;onClicked:{PaintClient.activateWand();root.focusCanvas()} tooltip:"魔棒 W" }
        Rectangle {width:24;height:1;color:Theme.line;anchors.horizontalCenter:parent.horizontalCenter}
        // Every slot clears the others: leaving a second tool active made the strip show the brush
        // while the bucket still owned the canvas.
        IconButton {objectName:"brushTool";width:26;height:26;glyph:"brush";checkable:true;enabled:!PaintClient.liquifyActive;checked:!PaintClient.liquifyActive && !PaintClient.eraser && !PaintClient.moveTool && !PaintClient.selectionTool && !PaintClient.eyedropperTool && !PaintClient.bucketTool && !PaintClient.smudgeTool;onClicked:{PaintClient.activateBrush();root.focusCanvas()} tooltip:"画笔 B"}
        IconButton {objectName:"eraserTool";width:26;height:26;glyph:"eraser";checkable:true;enabled:!PaintClient.liquifyActive;checked:!PaintClient.liquifyActive && PaintClient.eraser && !PaintClient.moveTool && !PaintClient.selectionTool;onClicked:{PaintClient.activateEraser();root.focusCanvas()} tooltip:"橡皮擦 E"}
        // The bucket is a painting tool, not a selection shape, so it gets its own slot.
        IconButton {objectName:"bucketTool";width:26;height:26;glyph:"bucket";checkable:true;enabled:!PaintClient.liquifyActive;checked:!PaintClient.liquifyActive && PaintClient.bucketTool;onClicked:{PaintClient.activateBucket();root.focusCanvas()} tooltip:"油漆桶 G"}
        IconButton {objectName:"smudgeTool";width:26;height:26;glyph:"smudge";checkable:true;enabled:!PaintClient.liquifyActive;checked:!PaintClient.liquifyActive && PaintClient.smudgeTool;onClicked:{PaintClient.activateSmudge();root.focusCanvas()} tooltip:"涂抹 R"}
        IconButton {objectName:"eyedropperTool";width:26;height:26;glyph:"eyedropper";checkable:true;enabled:!PaintClient.liquifyActive;checked:!PaintClient.liquifyActive && PaintClient.eyedropperTool;onClicked:{PaintClient.activateEyedropper();root.focusCanvas()} tooltip:"吸色 I · Alt 临时吸色"}
        Rectangle {width:24;height:1;color:Theme.line;anchors.horizontalCenter:parent.horizontalCenter}
        IconButton {objectName:"undoTool";width:26;height:26;glyph:"undo";enabled:root.resizeEditor?root.resizeEditor.canUndo:!PaintClient.liquifyActive && PaintClient.undoDepth>0 && !PaintClient.drawing;onClicked:if(root.resizeEditor)root.resizeEditor.undo();else PaintClient.undo();tooltip:"撤销 " + ShortcutSettings.bindings.undo}
        IconButton {objectName:"redoTool";width:26;height:26;glyph:"redo";enabled:root.resizeEditor?root.resizeEditor.canRedo:!PaintClient.liquifyActive && PaintClient.redoDepth>0 && !PaintClient.drawing;onClicked:if(root.resizeEditor)root.resizeEditor.redo();else PaintClient.redo();tooltip:"重做 " + ShortcutSettings.bindings.redo}
    }
    Rectangle {width:22;height:22;color:PaintClient.brushColor;border.color:Theme.line;anchors.bottom:parent.bottom;anchors.bottomMargin:12;anchors.horizontalCenter:parent.horizontalCenter}
}
