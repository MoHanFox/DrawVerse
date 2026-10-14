import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."
RowLayout {
    id:root
    objectName:"smudgeOptionsBar"
    required property var client
    readonly property var library:client.brushLibrary
    spacing:8
    Icon {name:"smudge";Layout.preferredWidth:20;Layout.preferredHeight:20}
    CompactComboBox {highPriorityMenu:true;
        objectName:"smudgeBrushPreset";model:root.library.presets;textRole:"name"
        currentIndex:root.library.presets.findIndex(p=>p.id===root.library.selectedId)
        Layout.preferredWidth:130;Layout.preferredHeight:24
        onActivated:index=>root.library.select(root.library.presets[index].id)
    }
    Label {text:"大小";color:Theme.muted}
    CompactSpinBox {objectName:"smudgeBrushSize";from:1;to:512;editable:true;value:Math.round(root.client.brushRadius*2);Layout.preferredWidth:76;Layout.preferredHeight:24;onValueModified:root.client.brushRadius=value/2}
    Label {text:"px";color:Theme.muted}
    Label {text:"强度：";color:Theme.muted}
    LayerPercent {tooltip:"控制每次涂抹搬运颜色的程度，并与笔刷不透明度共同生效。";objectName:"smudgeStrength";fraction:root.client.smudgeStrength;Layout.preferredWidth:70;Layout.preferredHeight:24;onCommitted:fraction=>root.client.smudgeStrength=fraction}
    Repeater {
        model:[{key:"Pressure",note:"使用数位笔压力控制涂抹强度；笔刷大小仍随压力变化。",title:"压感"},{key:"AllLayers",note:"从所有可见图层的合成颜色取样；只涂抹到当前图层。蒙版不支持此选项。",title:"对所有图层取样"},{key:"FingerPainting",note:"每条笔触起点带入前景色，随后继续搬运画布上的颜色；不使用背景色。",title:"手指绘画"}]
        CompactCheckBox {
            required property var modelData
            tooltip:modelData.note
            objectName:"smudge"+modelData.key
            text:modelData.title;checked:root.client["smudge"+modelData.key]
            onToggled:root.client["smudge"+modelData.key]=checked
        }
    }
    Item {Layout.fillWidth:true}
}
