import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../controls"
import "../theme"

RowLayout {
    id: root
    objectName:"bucketOptionsBar"
    required property var client
    spacing:8
    Icon {objectName:"bucketOptionsIcon";name:"bucket";Layout.preferredWidth:17;Layout.preferredHeight:17}
    Label {text:"不透明度：";color:Theme.muted}
    LayerPercent {
        tooltip:"控制填充颜色的不透明度。";objectName:"bucketOpacity";fraction:root.client.bucketOpacity
        Layout.preferredWidth:80;Layout.preferredHeight:24
        onCommitted:fraction=>root.client.bucketOpacity=fraction
    }
    Label {text:"容差：";color:Theme.muted}
    CompactSpinBox {
        tooltip:"容差越大，可匹配的颜色差异越大（0–255）。";objectName:"bucketTolerance";from:0;to:255;editable:true
        value:root.client.bucketTolerance
        Layout.preferredWidth:60;Layout.preferredHeight:24
        onValueModified:root.client.bucketTolerance=value
    }
    Repeater {
        model:[{key:"Antialias",note:"平滑填充边缘，减少锯齿。",title:"消除锯齿"},{key:"Contiguous",note:"仅填充与点击位置相连的相似颜色；关闭后填充所有相似颜色。",title:"连续的"},{key:"AllLayers",note:"根据所有可见图层的合成颜色判断填充区域；只修改当前图层。",title:"所有图层"}]
        CompactCheckBox {
            required property var modelData
            tooltip:modelData.note
            objectName:"bucket"+modelData.key
            text:modelData.title;checked:root.client["bucket"+modelData.key]
            onToggled:root.client["bucket"+modelData.key]=checked
        }
    }
    Item {Layout.fillWidth:true}
}
