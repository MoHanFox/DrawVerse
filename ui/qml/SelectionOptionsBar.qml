import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

RowLayout {
    id: root
    objectName: "selectionOptionsBar"
    required property var client
    readonly property bool lasso: client.selectionTool>=3
    spacing: 8
    Icon {name:root.lasso?(root.client.selectionTool===4?"polygonSelection":"lassoSelection"):(root.client.selectionTool===2?"ellipseSelection":"rectangleSelection");Layout.preferredWidth:17;Layout.preferredHeight:17}
    CompactComboBox {highPriorityMenu:true;
        objectName:"selectionVariant"
        model:root.lasso?["套索","多边形套索"]:["矩形选框","椭圆选框"]
        currentIndex:Math.max(0,root.client.selectionTool-(root.lasso?3:1))
        implicitWidth:100;implicitHeight:22
        onActivated:root.client.selectionTool=currentIndex+(root.lasso?3:1)
    }
    Rectangle {Layout.preferredWidth:1;Layout.preferredHeight:24;color:Theme.line}
    RowLayout {
        spacing:2
        Repeater {
            model:[{glyph:"selectionNew",name:"新建选区"},{glyph:"selectionAdd",name:"添加到选区"},{glyph:"selectionSubtract",name:"从选区减去"},{glyph:"selectionIntersect",name:"与选区交叉"}]
            IconButton {
                required property var modelData
                required property int index
                objectName:"selectionMode:"+index
                Layout.preferredWidth:26;Layout.preferredHeight:26
                glyph:modelData.glyph;tooltip:modelData.name
                checkable:true;checked:root.client.selectionOperation===index
                onClicked:root.client.selectionOperation=index
            }
        }
    }
    Rectangle {Layout.preferredWidth:1;Layout.preferredHeight:24;color:Theme.line}
    Label {text:"羽化：";color:Theme.muted}
    CompactSpinBox {
        tooltip:"使选区边缘逐渐过渡，数值为像素半径。";objectName:"selectionFeather"
        from:0;to:1000;editable:true
        validator:RegularExpressionValidator {regularExpression:/[0-9]{1,4}(\s*像素)?/}
        value:Math.round(root.client.selectionFeather)
        textFromValue:function(value,locale){return value+" 像素"}
        valueFromText:function(text,locale){return parseInt(text)||0}
        Layout.preferredWidth:104;Layout.preferredHeight:24
        onValueModified:root.client.selectionFeather=value
    }
    CompactCheckBox {
        objectName:"selectionAntialias"
        tooltip:"平滑选区边缘，减少锯齿。"
        text:"消除锯齿";checked:root.client.selectionAntialias
        font.pixelSize:10;padding:0;spacing:6
        onToggled:root.client.selectionAntialias=checked
    }
    Item {Layout.fillWidth:true}
}
