import QtQuick
import QtQuick.Controls
import "."

MenuItem {
    id: control
    objectName: "glassMenuItem:"+text
    topPadding: 3
    bottomPadding: 3
    implicitHeight: implicitContentHeight+topPadding+bottomPadding
    palette.windowText: enabled ? Theme.text : Theme.disabled
    background: Rectangle {
        implicitWidth: 200
        implicitHeight: 0
        radius: 6
        color: control.down ? Theme.selected : control.highlighted ? Theme.hover : "transparent"
    }
}
