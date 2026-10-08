import QtQuick
import QtQuick.Controls
import QtQuick.Window
import "."
ApplicationWindow {
    id: root
    required property var groupData
    property var canvasView
    visible: true
    title: "DrawVerse · "+Workspace.panelDefinition(groupData.active).title
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
    minimumWidth: 240; minimumHeight: Math.max(200,panelGroup.minimumPanelHeight+12)
    PanelGroup { id:panelGroup;anchors.fill: parent; anchors.margins: 6; groupData: root.groupData; canvasView: root.canvasView }
    function remember() { if (visible) Workspace.updateGeometry(groupData.id,x,y,width,height) }
    onXChanged: remember()
    onYChanged: remember()
    onWidthChanged: remember()
    onHeightChanged: remember()
    onClosing: event => {
        if (PaintClient.closing) { event.accepted=true; return }
        event.accepted=false; Qt.callLater(() => Workspace.returnGroup(groupData.id))
    }
}
