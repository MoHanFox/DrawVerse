import QtQuick
import QtQuick.Controls
import "../theme"

CheckBox {
    id:root
    property string tooltip:""
    hoverEnabled:true
    font.pixelSize:12;padding:0;spacing:6
    ToolTip.delay:1000;ToolTip.visible:hovered && tooltip.length>0;ToolTip.text:tooltip
    indicator:Rectangle {
        implicitWidth:13;implicitHeight:13;x:0;y:(root.height-height)/2
        radius:2;color:root.checked?Theme.text:Theme.input;border.color:Theme.muted
        opacity:root.enabled?1:0.5
        Icon {anchors.fill:parent;anchors.margins:2;name:"check";color:Theme.input;visible:root.checked}
    }
    contentItem:Text {text:root.text;font:root.font;color:root.enabled?Theme.text:Theme.disabled;leftPadding:19;verticalAlignment:Text.AlignVCenter}
}
