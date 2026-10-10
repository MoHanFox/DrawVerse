import QtQuick
import QtQuick.Controls
import "."
SpinBox {
    id: control
    implicitWidth: 90; implicitHeight: 26
    leftPadding: 23; rightPadding: 23; topPadding: 3; bottomPadding: 3
    font.pixelSize: 10
    background: Rectangle { color: Theme.input; border.color: Theme.line; radius: 2 }
    contentItem: TextInput {
        objectName: control.objectName+"Text"
        text: control.displayText; font: control.font; color: Theme.text
        horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter
        readOnly: !control.editable; selectByMouse: true
        validator: control.validator; inputMethodHints: Qt.ImhFormattedNumbersOnly
    }
    up.indicator: Rectangle {
        x: control.width-width; width: 22; height: control.height
        color: control.up.pressed ? Theme.selected : control.up.hovered ? Theme.hover : Theme.raised
        Icon { anchors.centerIn: parent; width: 12; height: 12; name: "plus" }
    }
    down.indicator: Rectangle {
        width: 22; height: control.height
        color: control.down.pressed ? Theme.selected : control.down.hovered ? Theme.hover : Theme.raised
        Rectangle { anchors.centerIn: parent; width: 8; height: 1; color: Theme.text }
    }
}
