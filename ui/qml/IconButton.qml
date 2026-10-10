import QtQuick
import QtQuick.Controls
import QtQuick.Shapes
import "."

ToolButton {
    id: root
    property string glyph: "brush"
    property string tooltip: ""
    // Marks the tool as having several shapes; the small corner triangle is the affordance.
    property bool hasVariants: false
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
    // Bottom-right triangle, drawn outward so it never covers the glyph.
    Shape {
        objectName: root.objectName.length>0 ? "variantMark:"+root.objectName : ""
        visible: root.hasVariants
        width: 6; height: 6
        x: root.width-6; y: root.height-6
        ShapePath {
            strokeColor: "transparent"; strokeWidth: 0
            fillColor: root.enabled ? Theme.text : Theme.disabled
            PathSvg { path: "M0 6L6 6L6 0Z" }
        }
    }
}
