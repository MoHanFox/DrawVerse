import QtQuick
import QtQuick.Controls
import "."
Slider {
    id: control
    property real sizeFactor: 1
    implicitHeight: 22*sizeFactor; leftPadding: 4*sizeFactor; rightPadding: 4*sizeFactor
    background: Rectangle {
        x: control.leftPadding; y: control.topPadding+control.availableHeight/2-height/2
        width: control.availableWidth; height: 3*control.sizeFactor; radius: control.sizeFactor; color: Theme.line
        Rectangle { width: control.visualPosition*parent.width; height: parent.height; color: Theme.muted; radius: 1 }
    }
    handle: Rectangle {
        x: control.leftPadding+control.visualPosition*(control.availableWidth-width)
        y: control.topPadding+control.availableHeight/2-height/2
        width: 9*control.sizeFactor; height: 12*control.sizeFactor; radius: 2*control.sizeFactor
        color: control.pressed ? "#eeeeee" : Theme.text
        border.color: Theme.line
    }
}
