import QtQuick
import QtQuick.Controls
import "."
ComboBox {
    id: control
    implicitHeight: 20
    leftPadding: 5; rightPadding: 16
    font.pixelSize: 9
    contentItem: Text {
        text: control.displayText; font: control.font
        color: control.enabled ? Theme.text : Theme.disabled
        elide: Text.ElideRight; verticalAlignment: Text.AlignVCenter
    }
    indicator: Icon { x: control.width-14; y: (control.height-height)/2; width: 10; height: 10; name: "down"; color: Theme.muted }
    background: Rectangle { color: Theme.input; border.color: Theme.line; radius: 2 }
}
