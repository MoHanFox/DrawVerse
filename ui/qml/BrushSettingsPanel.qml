import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

ColumnLayout {
    id:root
    readonly property var library:PaintClient.brushLibrary
    spacing:6
    Label {objectName:"selectedBrushName";text:root.library.selectedName;color:Theme.text;font.pixelSize:10;elide:Text.ElideRight;Layout.fillWidth:true}
    // No second preset grid: these controls always edit the selected brush.
    RowLayout {
        Layout.fillWidth:true
        Label {text:"大小";font.pixelSize:9;color:Theme.text;Layout.fillWidth:true}
        CompactSpinBox {objectName:"settingsBrushSize";implicitWidth:76;implicitHeight:20;editable:true;from:1;to:512;value:Math.round(root.library.radius*2);onValueModified:root.library.radius=value/2}
        Label {text:"px";font.pixelSize:8;color:Theme.muted}
    }
    CompactSlider {Layout.fillWidth:true;from:1;to:512;value:root.library.radius*2;onMoved:root.library.radius=value/2}
    RowLayout {
        Label {text:"间距";font.pixelSize:9;color:Theme.text;Layout.fillWidth:true}
        Label {text:(root.library.spacing*50).toFixed(1)+"%";font.pixelSize:9;color:Theme.text}
    }
    CompactSlider {objectName:"settingsBrushSpacing";Layout.fillWidth:true;from:2.5;to:50;value:root.library.spacing*50;onMoved:root.library.spacing=value/50}
    Rectangle {Layout.fillWidth:true;height:1;color:Theme.line}
    RowLayout {
        Label {text:"不透明度";font.pixelSize:9;color:Theme.text;Layout.fillWidth:true}
        Label {text:Math.round(root.library.opacity*100)+"%";font.pixelSize:9;color:Theme.text}
    }
    CompactSlider {objectName:"settingsBrushOpacity";Layout.fillWidth:true;from:0;to:1;value:root.library.opacity;onMoved:root.library.opacity=value}
    Label {text:"圆形笔尖 · 压感控制大小与流量";font.pixelSize:8;color:Theme.muted;wrapMode:Text.WordWrap;Layout.fillWidth:true}
    Item {Layout.fillHeight:true;Layout.minimumHeight:0}
    Rectangle {
        objectName:"brushStrokePreview";Layout.fillWidth:true;Layout.preferredHeight:60;color:Theme.selected
        Image {anchors.fill:parent;anchors.margins:4;source:root.library.selectedPreview;fillMode:Image.PreserveAspectFit;asynchronous:true}
        Text {anchors.centerIn:parent;text:root.library.previewMessage;font.pixelSize:9;color:Theme.muted;visible:!root.library.selectedPreview.length}
        ToolTip.visible:hover.containsMouse;ToolTip.text:"实际引擎压感笔触 · 预览大小按范围缩放"
        MouseArea {id:hover;anchors.fill:parent;hoverEnabled:true;onDoubleClicked:root.library.retryPreview()}
    }
    Timer {id:previewDelay;interval:100;onTriggered:root.library.requestPreview(root.library.selectedId)}
    Component.onCompleted:previewDelay.restart()
    Connections {target:root.library;function onSettingsChanged(){previewDelay.restart()}}
}
