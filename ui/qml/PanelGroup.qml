import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Rectangle {
    id: root
    objectName: "dockGroup:"+groupData.id
    required property var groupData
    property var canvasView
    property string selected: groupData.active
    readonly property int minimumPanelHeight: Workspace.panelDefinition(selected).kind === "layers" ? 360 : 180
    color: Theme.surface
    border.color: drop.containsDrag ? Theme.accent : Theme.line
    radius: 7
    clip: true
    implicitHeight: 320
    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        Rectangle {
            Layout.fillWidth: true
            height: 24
            color: Theme.raised
            Text { anchors.centerIn: parent; text: "···"; color: Theme.muted; font.pixelSize: 18 }
            MouseArea {
                anchors.fill: parent
                property point start
                onPressed: mouse => { start = Qt.point(mouse.x, mouse.y) }
                onPositionChanged: mouse => {
                    if (pressed && Math.abs(mouse.x-start.x)+Math.abs(mouse.y-start.y)>8) {
                        Workspace.beginDrag(root.groupData.id, "", true)
                    }
                }
            }
            ToolButton {
                anchors.right: parent.right; height: 24; width: 28
                text: root.groupData.location === "floating" ? "↙" : "↗"
                ToolTip.visible: hovered
                ToolTip.text: root.groupData.location === "floating" ? "返回工作区" : "浮动整个面板组"
                onClicked: {
                    if (root.groupData.location === "floating") Workspace.returnGroup(root.groupData.id)
                    else Workspace.detachGroup(root.groupData.id)
                }
            }
        }
        Flickable {
            Layout.fillWidth: true
            implicitHeight: 36
            contentWidth: tabs.width
            contentHeight: 36
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            Row {
                id: tabs
                height: 36
                Repeater {
                    model: root.groupData.panels
                    Rectangle {
                        required property string modelData
                        width: Math.max(66, label.implicitWidth+24)
                        height: 36
                        color: root.selected === modelData ? Theme.surface : Theme.raised
                        Text { id: label; anchors.centerIn: parent; text: Workspace.panelDefinition(modelData).title; color: root.selected === modelData ? Theme.accent : Theme.muted; font.pixelSize: 12 }
                        Rectangle { anchors.bottom: parent.bottom; height: 2; width: parent.width; color: Theme.accent; visible: root.selected === modelData }
                        MouseArea {
                            anchors.fill: parent
                            property point start
                            onPressed: mouse => { start=Qt.point(mouse.x,mouse.y); root.selected=modelData; Workspace.setActive(root.groupData.id,modelData) }
                            onPositionChanged: mouse => { if (pressed && Math.abs(mouse.x-start.x)+Math.abs(mouse.y-start.y)>8) Workspace.beginDrag(root.groupData.id,modelData,false) }
                            onDoubleClicked: Workspace.detachPanel(root.groupData.id,modelData)
                        }
                    }
                }
            }
        }
        PanelContent { Layout.fillWidth: true; Layout.fillHeight: true; panelId: root.selected; canvasView: root.canvasView }
    }
    DropArea {
        id: drop
        anchors.fill: parent
        keys: ["application/x-drawverse-panel"]
        onDropped: d => {
            if (Workspace.dockPayload(d.getDataAsString("application/x-drawverse-panel"),root.groupData.location,root.groupData.id)) d.acceptProposedAction()
        }
    }
}
