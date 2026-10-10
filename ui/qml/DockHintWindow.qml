import QtQuick
import QtQuick.Window
import "."
Window {
    id:root
    required property rect targetRect
    required property string edge
    objectName:"dockingHintWindow"
    flags:Qt.ToolTip|Qt.FramelessWindowHint|Qt.WindowTransparentForInput|Qt.WindowDoesNotAcceptFocus
    color:"transparent";visible:true
    readonly property bool lateral:edge==="left" || edge==="right"
    x:Math.round(edge==="left"?targetRect.x-10:edge==="right"?targetRect.x+targetRect.width:targetRect.x)
    y:Math.round(edge==="before"?targetRect.y-10:edge==="after"?targetRect.y+targetRect.height:targetRect.y)
    width:lateral?10:Math.max(1,Math.round(targetRect.width))
    height:lateral?Math.max(1,Math.round(targetRect.height)):10
    Rectangle {
        anchors.fill:parent;color:"#2023b5ee"
        Rectangle {anchors.fill:parent;anchors.margins:3;color:"#4023b5ee"}
        Rectangle {
            x:root.edge==="left"?parent.width-width:0
            y:root.edge==="before"?parent.height-height:0
            width:root.lateral?2:parent.width;height:root.lateral?parent.height:2;color:Theme.accent
        }
    }
}
