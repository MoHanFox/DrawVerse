import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Control {
    id: root
    property real fraction: 1
    signal committed(real fraction)
    function openSlider() { popup.open() }
    implicitWidth: 52
    implicitHeight: 20
    // Draft changes stay in this control; commit exactly once on release/edit.
    contentItem: RowLayout {
        spacing: 0
        TextField {
            id: edit
            objectName: root.objectName+"Input"
            Layout.fillWidth: true; implicitWidth: 34
            text: Math.round(root.fraction*100)+"%"
            horizontalAlignment: Text.AlignRight
            selectByMouse: true
            validator: RegularExpressionValidator { regularExpression: /(?:100|[0-9]{1,2})%?/ }
            background: Rectangle { color: Theme.input; border.color: edit.activeFocus ? Theme.accent : Theme.line; radius: 2 }
            color: Theme.text; font.pixelSize: 9
            function commit() {
                if(acceptableInput) root.committed(parseInt(text)/100)
                text=Qt.binding(function() { return Math.round(root.fraction*100)+"%" })
            }
            onEditingFinished: commit()
        }
        IconButton { glyph: "down"; padding: 3; implicitWidth: 12; implicitHeight: 20; onClicked: popup.open() }
    }
    Popup {
        id: popup
        y: root.height; x: root.width-width; width: 170; padding: 10
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        onOpened: slider.value=root.fraction*100
        contentItem: CompactSlider {
            id: slider
            objectName: root.objectName+"Slider"
            from: 0; to: 100; stepSize: 1
            onPressedChanged: if(!pressed) root.committed(value/100)
            Keys.onReleased: event => { if([Qt.Key_Left,Qt.Key_Right,Qt.Key_Home,Qt.Key_End].indexOf(event.key)>=0) root.committed(value/100) }
        }
    }
}
