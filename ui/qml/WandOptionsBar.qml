import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

RowLayout {
    id:root
    objectName:"wandOptionsBar"
    required property var client
    readonly property var sizes:[1,3,5,11,31,51,101]
    spacing:8
    Icon {name:"wand";Layout.preferredWidth:20;Layout.preferredHeight:20}
    Rectangle {Layout.preferredWidth:1;Layout.preferredHeight:24;color:Theme.line}
    Repeater {
        model:[{glyph:"selectionNew",title:"新建选区"},{glyph:"selectionAdd",title:"添加到选区"},{glyph:"selectionSubtract",title:"从选区减去"},{glyph:"selectionIntersect",title:"与选区交叉"}]
        IconButton {
            required property var modelData
            required property int index
            objectName:"wandMode:"+index
            Layout.preferredWidth:28;Layout.preferredHeight:28
            glyph:modelData.glyph;tooltip:modelData.title
            checkable:true;checked:root.client.wandOperation===index
            onClicked:root.client.wandOperation=index
        }
    }
    Rectangle {Layout.preferredWidth:1;Layout.preferredHeight:24;color:Theme.line}
    Label {text:"取样大小：";color:Theme.muted}
    CompactComboBox {highPriorityMenu:true;
        tooltip:"对点击位置周围的像素求平均颜色作为取样颜色。";objectName:"wandSampleSize"
        model:["取样点","3 x 3 平均","5 x 5 平均","11 x 11 平均","31 x 31 平均","51 x 51 平均","101 x 101 平均"]
        currentIndex:root.sizes.indexOf(root.client.wandSampleSize)
        Layout.preferredWidth:150;Layout.preferredHeight:24
        onActivated:index=>root.client.wandSampleSize=root.sizes[index]
    }
    Label {text:"容差：";color:Theme.muted}
    CompactSpinBox {tooltip:"容差越大，可匹配的颜色差异越大（0–255）。";objectName:"wandTolerance";from:0;to:255;editable:true;value:root.client.wandTolerance;Layout.preferredWidth:60;Layout.preferredHeight:24;onValueModified:root.client.wandTolerance=value}
    Repeater {
        model:[{key:"Antialias",note:"平滑选区边缘，减少锯齿。",title:"消除锯齿"},{key:"Contiguous",note:"仅选择与点击位置相连的相似颜色；关闭后选择整张画布中的相似颜色。",title:"连续"},{key:"AllLayers",note:"根据所有可见图层的合成颜色取样；选区不会改变图层像素。",title:"对所有图层取样"}]
        CompactCheckBox {
            required property var modelData
            tooltip:modelData.note
            objectName:"wand"+modelData.key
            text:modelData.title;checked:root.client["wand"+modelData.key]
            onToggled:root.client["wand"+modelData.key]=checked
        }
    }
    Item {Layout.fillWidth:true}
}
