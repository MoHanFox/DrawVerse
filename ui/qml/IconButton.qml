import QtQuick
import QtQuick.Controls
import "."
ToolButton {
    id: root
    property string glyph: "brush"
    property string tooltip: ""
    implicitWidth: 28; implicitHeight: 28
    padding: 4
    hoverEnabled: true
    ToolTip.visible: hovered && tooltip.length>0
    ToolTip.delay: 600
    ToolTip.text: tooltip
    Accessible.name: tooltip
    background: Rectangle {
        radius: 2
        color: root.checked ? Theme.selected : root.down ? Theme.raised : root.hovered ? Theme.hover : "transparent"
    }
    contentItem: Icon { name: root.glyph; color: root.enabled ? Theme.text : Theme.disabled }
}
