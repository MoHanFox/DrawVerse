import QtQuick
import QtQuick.Controls
import "."
Slider {
    id: control
    implicitHeight: 22; leftPadding: 4; rightPadding: 4
    background: Rectangle {
        x: control.leftPadding; y: control.topPadding+control.availableHeight/2-height/2
        width: control.availableWidth; height: 3; radius: 1; color: Theme.line
        Rectangle { width: control.visualPosition*parent.width; height: parent.height; color: Theme.muted; radius: 1 }
    }
    handle: Rectangle {
        x: control.leftPadding+control.visualPosition*(control.availableWidth-width)
        y: control.topPadding+control.availableHeight/2-height/2
        width: 9; height: 12; radius: 2
        color: control.pressed ? "#eeeeee" : Theme.text
        border.color: Theme.line
    }
}
