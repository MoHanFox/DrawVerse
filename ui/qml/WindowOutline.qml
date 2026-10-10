import QtQuick
import QtQuick.Window
import "."
Window {
    id:root
    required property var hostWindow
    objectName:"applicationOutlineWindow"
    flags:Qt.ToolTip|Qt.FramelessWindowHint|Qt.WindowTransparentForInput|Qt.WindowDoesNotAcceptFocus
    color:"transparent"
    visible:hostWindow.visible && hostWindow.visibility===Window.Windowed
    x:hostWindow.x-1;y:hostWindow.y-1
    width:hostWindow.width+2;height:hostWindow.height+2
    Rectangle {
        objectName:"applicationWindowOutline"
        anchors.fill:parent;color:"transparent";radius:root.hostWindow.cornerRadius+1
        border.color:Qt.rgba(Theme.windowOutline.r,Theme.windowOutline.g,Theme.windowOutline.b,.5)
        border.width:1
    }
}
