import QtQuick
import QtQuick.Controls
import "."

// Right-click switcher for a tool that has several shapes (Photoshop style). Entries the engine
// cannot do yet are listed disabled and labelled, never presented as a working shape.
Popup {
    id: root
    objectName: "toolVariantPanel"
    property var canvasView
    property string toolTitle: ""
    // Each entry: {id, label, icon, selected, pending}. Real entries never carry pending.
    property var entries: []
    padding: 5
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    background: Rectangle { color: Theme.surface; border.color: Theme.line; radius: 3 }
    function at(index) { return root.entries[index] }
    function pick(id) {
        const chosen = root.entries.find(entry => entry.id === id)
        if (!chosen || chosen.pending || !chosen.apply) return
        chosen.apply()
        close()
    }
    contentItem: Column {
        spacing: 3
        Label {
            text: root.toolTitle
            color: Theme.muted; font.pixelSize: 8
            leftPadding: 5; rightPadding: 5; bottomPadding: 2
        }
        Repeater {
            id: rowRepeater
            objectName: "toolVariantRows"
            model: root.entries
            delegate: Rectangle {
                id: entry
                required property int index
                readonly property var variant: root.at(entry.index)
                objectName: "variantEntry:" + entry.variant.id
                enabled: !entry.variant.pending
                width: 152; height: 30
                color: entry.variant.pending ? "transparent" : entry.variant.selected || entryMouse.containsMouse ? Theme.selected : "transparent"
                radius: 2
                Icon {
                    id: entryIcon
                    name: entry.variant.icon
                    x: 6; anchors.verticalCenter: parent.verticalCenter
                    width: 16; height: 16
                    color: entry.variant.pending ? Theme.disabled : Theme.text
                }
                Label {
                    anchors.left: entryIcon.right; anchors.leftMargin: 8
                    anchors.right: entryState.left; anchors.rightMargin: 6
                    anchors.verticalCenter: parent.verticalCenter
                    text: entry.variant.label
                    color: entry.variant.pending ? Theme.disabled : Theme.text
                    font.pixelSize: 9; elide: Text.ElideRight
                }
                Label {
                    id: entryState
                    objectName: "toolVariantState:" + entry.variant.id
                    anchors.right: parent.right; anchors.rightMargin: 6
                    anchors.verticalCenter: parent.verticalCenter
                    text: entry.variant.pending ? "待支持" : entry.variant.selected ? "✓" : ""
                    color: entry.variant.pending ? Theme.disabled : Theme.accent
                    font.pixelSize: 8
                }
                MouseArea {
                    id: entryMouse
                    objectName: "variantTrigger:" + entry.variant.id
                    anchors.fill: parent
                    hoverEnabled: true
                    enabled: !entry.variant.pending
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.pick(entry.variant.id)
                }
            }
        }
    }
}
