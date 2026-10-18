import QtQuick
import QtQuick.Controls
import "../theme"

TextField {
    implicitHeight: 20
    color: Theme.text
    font.pixelSize: 9
    selectByMouse: true
    horizontalAlignment: Text.AlignRight
    leftPadding: 3; rightPadding: 3; topPadding: 0; bottomPadding: 0
    background: Rectangle { color: Theme.input; border.color: Theme.line; radius: 2 }
}
