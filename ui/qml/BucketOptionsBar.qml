import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

RowLayout {
    id: root
    objectName:"bucketOptionsBar"
    required property var client
    spacing:8
    Icon {objectName:"bucketOptionsIcon";name:"bucket";Layout.preferredWidth:17;Layout.preferredHeight:17}
    Label {text:"不透明度：";color:Theme.muted}
    LayerPercent {
        objectName:"bucketOpacity";fraction:root.client.bucketOpacity
        Layout.preferredWidth:80;Layout.preferredHeight:24
        onCommitted:fraction=>root.client.bucketOpacity=fraction
    }
    Label {text:"容差：";color:Theme.muted}
    CompactSpinBox {
        objectName:"bucketTolerance";from:0;to:255;editable:true
        value:root.client.bucketTolerance
        Layout.preferredWidth:60;Layout.preferredHeight:24
        onValueModified:root.client.bucketTolerance=value
    }
    Repeater {
        model:[{key:"Antialias",title:"消除锯齿"},{key:"Contiguous",title:"连续的"},{key:"AllLayers",title:"所有图层"}]
        CheckBox {
            required property var modelData
            objectName:"bucket"+modelData.key
            text:modelData.title;checked:root.client["bucket"+modelData.key]
            onToggled:root.client["bucket"+modelData.key]=checked
            font.pixelSize:12;padding:0;spacing:6
            indicator:Rectangle {
                implicitWidth:13;implicitHeight:13;x:0;y:(parent.height-height)/2
                radius:2;color:parent.checked?Theme.text:Theme.input;border.color:Theme.muted
                Icon {anchors.fill:parent;anchors.margins:2;name:"check";color:Theme.input;visible:parent.parent.checked}
            }
            contentItem:Text {text:parent.text;font:parent.font;color:Theme.text;leftPadding:19;verticalAlignment:Text.AlignVCenter}
        }
    }
    Item {Layout.fillWidth:true}
}
