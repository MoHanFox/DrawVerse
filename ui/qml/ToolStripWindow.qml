import QtQuick
import QtQuick.Controls
import "."

ApplicationWindow {
    id: root
    objectName: "floatingToolStrip"
    property var canvasView
    property bool initializing: true
    visible: true
    flags: Qt.Tool | Qt.FramelessWindowHint
    title: "DrawVerse · 工具"
    width: 38; height: 274; minimumWidth:38;maximumWidth:38;minimumHeight:274;maximumHeight:274
    x:Workspace.toolStripX;y:Workspace.toolStripY
    color:Theme.panelBar
    ToolStrip {anchors.fill:parent;anchors.margins:1;canvasView:root.canvasView}
    Component.onCompleted:initializing=false
    onXChanged:if(visible && !initializing)Workspace.updateToolStripPosition(x,y)
    onYChanged:if(visible && !initializing)Workspace.updateToolStripPosition(x,y)
    onClosing:event=>{if(PaintClient.closing){event.accepted=true;return}event.accepted=false;Qt.callLater(()=>Workspace.dockToolStrip("drawverse-tools-v1"))}
}
