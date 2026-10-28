import QtQuick
import QtQuick.Controls
import "../theme"
TextField {
    id: root
    property real value: 0
    property real from: 0
    property real to: 255
    property int decimals: 0
    property string unit: ""
    signal edited(real number)
    implicitWidth: 94; implicitHeight: 28
    color: Theme.text; font.pixelSize: 13
    horizontalAlignment: TextInput.AlignHCenter
    selectByMouse: true
    text: Number(value).toFixed(decimals)
    rightPadding: unit.length ? 24 : 6; leftPadding: 6
    validator: DoubleValidator {bottom: root.from; top: root.to; decimals: root.decimals; locale: "C"}
    background: Rectangle {color: "#222325"; radius: 3; border.color: root.activeFocus ? Theme.accent : "#484a4c"}
    Label {anchors.right: parent.right; anchors.rightMargin: 7; anchors.verticalCenter: parent.verticalCenter; text: root.unit; color: Theme.muted}
    onTextEdited: if(acceptableInput) edited(Math.max(from,Math.min(to,Number(text))))
    onEditingFinished: {if(acceptableInput) edited(Number(text)); text=Number(value).toFixed(decimals)}
    onValueChanged: if(!activeFocus) text=Number(value).toFixed(decimals)
}
