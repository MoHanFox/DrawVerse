import QtQuick
import QtQuick.Controls
import "."
ComboBox {
    id: control
    implicitHeight: 20
    leftPadding: 5; rightPadding: 16
    font.pixelSize: 9
    signal optionHovered(int index)
    signal menuClosed()
    contentItem: Text {
        text: control.displayText; font: control.font
        color: control.enabled ? Theme.text : Theme.disabled
        elide: Text.ElideRight; verticalAlignment: Text.AlignVCenter
    }
    indicator: Icon { x: control.width-14; y: (control.height-height)/2; width: 10; height: 10; name: "down"; color: Theme.muted }
    background: Rectangle { color: Theme.input; border.color: Theme.line; radius: 2 }
    delegate: ItemDelegate {
        required property int index
        required property var modelData
        objectName:control.objectName+"Option:"+index
        width:control.popup.availableWidth;height:20;padding:5;hoverEnabled:true
        highlighted:control.highlightedIndex===index
        onHoveredChanged:control.optionHovered(hovered?index:-1)
        contentItem:Text {
            text:control.textRole?modelData[control.textRole]:modelData
            font:control.font;color:parent.hovered?Theme.accent:Theme.text
            verticalAlignment:Text.AlignVCenter;elide:Text.ElideRight
        }
        background:Rectangle {color:parent.hovered || parent.highlighted?Theme.hover:Theme.surface}
    }
    popup: Popup {
        y:control.height;width:Math.max(132,control.width);padding:2
        implicitHeight:Math.min(264,contentItem.implicitHeight+4)
        onClosed:control.menuClosed()
        background:Rectangle {color:Theme.surface;border.color:Theme.line;radius:2}
        contentItem:ListView {
            clip:true;implicitHeight:contentHeight
            model:control.popup.visible?control.delegateModel:null
            currentIndex:control.highlightedIndex
            ScrollIndicator.vertical:ScrollIndicator {}
        }
    }
}
