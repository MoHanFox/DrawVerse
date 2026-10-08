import QtQuick
import QtQuick.Controls
import QtQuick.Window
import "."
ApplicationWindow {
    id: root
    required property var groupData
    property var canvasView
    property int expandedHeight: groupData.height
    visible: true
    flags: Qt.Tool | Qt.FramelessWindowHint
    font.family: Qt.platform.os==="windows" ? "Microsoft YaHei UI" : "sans-serif"
    font.pixelSize: 10
    title: "DrawVerse · "+Workspace.panelDefinition(panelGroup.selected).title
    color: Theme.background
    palette.window: Theme.surface
    palette.windowText: Theme.text
    palette.base: Theme.background
    palette.text: Theme.text
    palette.button: Theme.raised
    palette.buttonText: Theme.text
    palette.highlight: Theme.selected
    palette.highlightedText: Theme.accent
    x: groupData.x; y: groupData.y; width: groupData.width; height: groupData.height
    minimumWidth: 150; minimumHeight: panelGroup.minimumPanelHeight+4
    PanelGroup { id:panelGroup;anchors.fill: parent; anchors.margins: 2; groupData: root.groupData; canvasView: root.canvasView }
    Connections {
        target: panelGroup
        function onCollapsedChanged() {
            if(panelGroup.collapsed) root.expandedHeight=root.height
            Qt.callLater(() => { root.height=panelGroup.collapsed ? panelGroup.minimumPanelHeight+4 : root.expandedHeight })
        }
    }
    ResizeFrame {targetWindow:root}
    Component.onCompleted: if(panelGroup.collapsed) height=panelGroup.minimumPanelHeight+4
    function updateLayout(data) {
        groupData=data; x=data.x; y=data.y; width=data.width; expandedHeight=data.height
        height=panelGroup.collapsed ? panelGroup.minimumPanelHeight+4 : data.height
    }
    function remember() { if (visible) Workspace.updateGeometry(groupData.id,x,y,width,panelGroup.collapsed ? expandedHeight : height) }
    onXChanged: remember()
    onYChanged: remember()
    onWidthChanged: remember()
    onHeightChanged: remember()
    onClosing: event => {
        if (PaintClient.closing) { event.accepted=true; return }
        event.accepted=false; Qt.callLater(() => Workspace.returnGroup(groupData.id))
    }
}
