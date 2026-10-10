import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

ColumnLayout {
    id:root
    readonly property var library:PaintClient.brushLibrary
    spacing:4
    RowLayout {
        Layout.fillWidth:true
        Label {text:"大小";color:Theme.text;font.pixelSize:9;Layout.fillWidth:true}
        CompactSpinBox {objectName:"libraryBrushSize";from:1;to:512;editable:true;implicitWidth:76;implicitHeight:20;value:Math.round(root.library.radius*2);onValueModified:root.library.radius=value/2}
        Label {text:"px";color:Theme.muted;font.pixelSize:8}
    }
    CompactSlider {Layout.fillWidth:true;from:1;to:512;value:root.library.radius*2;onMoved:root.library.radius=value/2}
    TextField {id:search;objectName:"brushSearch";Layout.fillWidth:true;implicitHeight:22;font.pixelSize:9;placeholderText:"搜索画笔";color:Theme.text;background:Rectangle {color:Theme.input;border.color:Theme.line}}
    RowLayout {
        Layout.fillWidth:true;spacing:2
        Repeater {
            model:[6,12,24,48]
            Button {
                required property int modelData
                Layout.fillWidth:true;implicitWidth:26;implicitHeight:30;padding:1
                onClicked:root.library.radius=modelData/2
                background:Rectangle {color:parent.hovered?Theme.hover:Theme.strip;border.color:Theme.line}
                contentItem:Column {
                    spacing:2
                    Rectangle {width:Math.min(12,modelData/4+3);height:width;radius:width/2;color:Theme.text;anchors.horizontalCenter:parent.horizontalCenter}
                    Text {text:modelData;font.pixelSize:8;color:Theme.muted;anchors.horizontalCenter:parent.horizontalCenter}
                }
            }
        }
    }
    RowLayout {Icon {name:"folder";width:14;height:14} Label {text:"基础与自定义画笔";font.pixelSize:9;color:Theme.text}}
    ListView {
        objectName:"brushPresetList";Layout.fillWidth:true;Layout.fillHeight:true;clip:true;spacing:2
        model:root.library.presets.filter(p=>p.name.toLowerCase().indexOf(search.text.toLowerCase())>=0)
        delegate:Rectangle {
            required property var modelData
            objectName:"brushPreset:"+modelData.id
            width:ListView.view.width;height:58
            color:root.library.selectedId===modelData.id?Theme.selected:Theme.surface
            border.color:Theme.line
            Image {anchors.left:parent.left;anchors.right:parent.right;anchors.top:parent.top;anchors.margins:4;height:35;source:modelData.preview;fillMode:Image.PreserveAspectFit;asynchronous:true}
            Text {x:6;y:40;width:parent.width-12;text:modelData.name;color:Theme.text;font.pixelSize:9;elide:Text.ElideRight}
            MouseArea {anchors.fill:parent;onClicked:root.library.select(modelData.id)}
            Component.onCompleted:root.library.requestPreview(modelData.id)
        }
        ScrollBar.vertical:ScrollBar {width:6}
    }
    RowLayout {
        Item {Layout.fillWidth:true}
        IconButton {glyph:"plus";tooltip:"保存当前设置为新画笔";implicitHeight:22;onClicked:saveDialog.open()}
        IconButton {glyph:"trash";tooltip:"删除自定义画笔";implicitHeight:22;enabled:root.library.presets.some(p=>p.id===root.library.selectedId && !p.builtin);onClicked:root.library.remove(root.library.selectedId)}
    }
    Dialog {
        id:saveDialog;title:"保存画笔";modal:true;standardButtons:Dialog.Ok|Dialog.Cancel
        anchors.centerIn:Overlay.overlay
        TextField {id:brushName;placeholderText:"画笔名称";maximumLength:80;text:root.library.selectedName+" 副本"}
        onAccepted:root.library.saveCopy(brushName.text)
    }
    Timer {id:previewDelay;interval:100;onTriggered:root.library.requestPreview(root.library.selectedId)}
    Connections {target:root.library;function onSettingsChanged(){previewDelay.restart()}}
}
