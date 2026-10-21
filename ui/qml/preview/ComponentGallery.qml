import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../app"
import "../brushes"
import "../controls"
import "../layers"
import "../theme"

ApplicationWindow {
    id: root
    objectName: "qmlComponentGallery"
    visible: true
    width: 1100; height: 800
    title: "DrawVerse · QML 控件预览"
    color: Theme.background
    readonly property var samples: [welcome,tip,parameters,brushes,settings,layers]
    header: ToolBar {
        RowLayout {
            anchors.fill: parent; anchors.margins: 6
            ComboBox {
                id: sample
                objectName: "previewComponentChoice"
                model: ["欢迎页","笔尖方向盘","参数滑条","画笔列表","画笔设置","图层"]
                implicitWidth: 150
            }
            Label {text: "宽"}
            SpinBox {id: previewWidth; from: 180; to: 1800; value: 520; editable: true}
            Label {text: "高"}
            SpinBox {id: previewHeight; from: 120; to: 1800; value: 600; editable: true}
            ComboBox {id: zoom; model: ["50%","75%","100%","125%","150%","200%"]; currentIndex: 2; implicitWidth: 90}
            ComboBox {id: background; model: ["默认背景","黑色","灰色","白色"]; implicitWidth: 115}
            Item {Layout.fillWidth: true}
        }
        implicitHeight: 48
    }
    footer: Label {
        padding: 8; color: Theme.muted
        text: "Live 运行项在保存 QML 后自动刷新；普通运行项需要重新运行。预览设置独立，刷新会清空临时编辑会话。"
        wrapMode: Text.Wrap
    }
    Flickable {
        anchors.fill: parent
        contentWidth: Math.max(width,stage.width+40)
        contentHeight: Math.max(height,stage.height+40)
        clip: true
        ScrollBar.vertical: ScrollBar {}
        ScrollBar.horizontal: ScrollBar {}
        Rectangle {
            id: stage
            x: 20; y: 20
            readonly property real previewScale: [0.5,0.75,1,1.25,1.5,2][zoom.currentIndex]
            width: previewWidth.value*previewScale
            height: previewHeight.value*previewScale
            color: [Theme.surface,"#000000","#808080","#ffffff"][background.currentIndex]
            clip: true
            Loader {
                objectName: "previewComponentLoader"
                width: previewWidth.value; height: previewHeight.value
                scale: stage.previewScale; transformOrigin: Item.TopLeft
                sourceComponent: root.samples[sample.currentIndex]
            }
        }
    }
    Component {id: welcome; WelcomeScreen {ready: true}}
    Component {
        id: tip
        Item {
            BrushTipShapeControl {
                anchors.left: parent.left; anchors.right: parent.right; anchors.top: parent.top; anchors.margins: 16
                onAngleEdited: value => angle=value
                onRoundnessEdited: value => roundness=value
            }
        }
    }
    Component {
        id: parameters
        Item {
            ColumnLayout {
                anchors.left: parent.left; anchors.right: parent.right; anchors.top: parent.top; anchors.margins: 16
                ParameterSlider {id: sizeSlider; title: "大小"; unit: "px"; from: 1; to: 15000; value: 2000; onCommitted: n => sizeSlider.value=n}
                ParameterSlider {id: opacitySlider; title: "不透明度"; unit: "%"; value: 75; onCommitted: n => opacitySlider.value=n}
                CompactCheckBox {text: "预览选项"; checked: true}
            }
        }
    }
    Component {id: brushes; BrushLibraryPanel {}}
    Component {id: settings; BrushSettingsPanel {}}
    Component {id: layers; LayersPanel {}}
}
