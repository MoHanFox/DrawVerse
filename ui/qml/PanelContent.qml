import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import DrawVerse 1.0
import "."

Item {
    id: root
    property string panelId
    property var canvasView
    readonly property var definition: Workspace.panelDefinition(panelId)
    readonly property string kind: definition.kind || ""
    Loader {
        anchors.fill: parent
        anchors.margins: 14
        sourceComponent: root.kind === "color" || root.kind === "palette" ? colorPanel : root.kind === "brush" ? brushPanel : root.kind === "layers" ? layersPanel : root.kind === "history" ? historyPanel : root.kind === "navigator" ? navigatorPanel : null
    }
    Component {
        id: colorPanel
        ColumnLayout {
            spacing: 12
            Rectangle {
                Layout.fillWidth: true; Layout.preferredHeight: 70; radius: 6
                color: PaintClient.brushColor
                Text { anchors.left: parent.left; anchors.bottom: parent.bottom; anchors.margins: 10; text: String(PaintClient.brushColor).toUpperCase(); color: PaintClient.brushColor.hslLightness > .6 ? "#222" : "white"; font.family: "Consolas"; font.pixelSize: 14 }
            }
            Label { text: "绘画色板"; color: Theme.muted; font.pixelSize: 11 }
            GridLayout {
                columns: 6; rowSpacing: 7; columnSpacing: 7; Layout.fillWidth: true
                Repeater {
                    model: ["#203b42","#2ea99d","#88d2be","#d7ece3","#f0e6d3","#ffffff","#a45c54","#ce836c","#edb17d","#ebca97","#dfcca9","#252936","#485a89","#7a84b0","#b29bc0","#d8b4bd","#76756a","#14191e"]
                    Rectangle {
                        required property string modelData
                        Layout.fillWidth: true; Layout.preferredHeight: 26; radius: 4; color: modelData
                        border.width: String(PaintClient.brushColor) === modelData ? 2 : 0
                        border.color: Theme.text
                        MouseArea { anchors.fill: parent; onClicked: PaintClient.brushColor=modelData }
                    }
                }
            }
            RowLayout {
                Label { text: "HEX"; color: Theme.muted; font.pixelSize: 11 }
                TextField {
                    Layout.fillWidth: true; text: String(PaintClient.brushColor).toUpperCase(); selectByMouse: true
                    maximumLength: 9
                    validator: RegularExpressionValidator { regularExpression: /#[0-9a-fA-F]{6}/ }
                    onEditingFinished: { if (acceptableInput) PaintClient.brushColor=text }
                }
            }
            Label { text: "工作空间  ·  线性 sRGB / RGBA 32F"; color: Theme.muted; font.pixelSize: 10; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            Item { Layout.fillHeight: true }
        }
    }
    Component {
        id: brushPanel
        ColumnLayout {
            spacing: 10
            Label { text: PaintClient.eraser ? "圆形橡皮擦" : "压感圆笔"; font.pixelSize: 17; color: Theme.text }
            Label { text: "半径  ·  "+Math.round(PaintClient.brushRadius)+" px"; color: Theme.muted }
            Slider { Layout.fillWidth: true; from: 1; to: 128; value: PaintClient.brushRadius; onMoved: PaintClient.brushRadius=value }
            RowLayout {
                Repeater {
                    model: [4,12,24,48]
                    Button { required property int modelData; text: modelData+" px"; Layout.fillWidth: true; onClicked: PaintClient.brushRadius=modelData }
                }
            }
            Label { text: "不透明度  ·  "+Math.round(PaintClient.brushOpacity*100)+"%"; color: Theme.muted }
            Slider { Layout.fillWidth: true; from: .01; to: 1; value: PaintClient.brushOpacity; onMoved: PaintClient.brushOpacity=value }
            Rectangle {
                Layout.fillWidth: true; Layout.preferredHeight: 70; color: Theme.background; radius: 6
                Row {
                    anchors.centerIn: parent; spacing: 8
                    Repeater {
                        model: [4,8,14,22,30]
                        Rectangle { required property int modelData; width: modelData; height: modelData; radius: modelData/2; color: PaintClient.eraser ? Theme.muted : PaintClient.brushColor; anchors.verticalCenter: parent.verticalCenter }
                    }
                }
            }
            Label { text: "压力控制半径与流量\n倾斜 / 旋转通道已传入核心\n当前圆笔不改变倾斜形状"; color: Theme.muted; font.pixelSize: 11; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            Item { Layout.fillHeight: true }
        }
    }
    Component { id: layersPanel; LayersPanel {} }
    Component {
        id: historyPanel
        ColumnLayout {
            Label { text: "操作历史"; font.pixelSize: 17; color: Theme.text }
            Label { text: "已提交 "+PaintClient.undoDepth+" 个操作\n可重做 "+PaintClient.redoDepth+" 个操作"; color: Theme.muted; lineHeight: 1.5 }
            Button { text: "↶  撤销  Ctrl+Z"; Layout.fillWidth: true; enabled: PaintClient.undoDepth>0 && !PaintClient.drawing; onClicked: PaintClient.undo() }
            Button { text: "↷  重做  Ctrl+Shift+Z"; Layout.fillWidth: true; enabled: PaintClient.redoDepth>0 && !PaintClient.drawing; onClicked: PaintClient.redo() }
            Label { text: "每笔独立撤销。图层增删、可见性与不透明度也进入历史。"; color: Theme.muted; wrapMode: Text.WordWrap; Layout.fillWidth: true; font.pixelSize: 11 }
            Item { Layout.fillHeight: true }
        }
    }
    Component {
        id: navigatorPanel
        ColumnLayout {
            Rectangle {
                Layout.fillWidth: true; Layout.preferredHeight: 170; color: Theme.background; radius: 5
                TransparencyGrid { objectName: "navigatorTransparency"; x: mini.documentRect.x; y: mini.documentRect.y; width: mini.documentRect.width; height: mini.documentRect.height }
                PaintCanvas { id: mini; objectName: "navigatorCanvas"; anchors.fill: parent; client: PaintClient; interactive: false }
            }
            Label { text: PaintClient.documentWidth+" × "+PaintClient.documentHeight+" px"; color: Theme.muted }
            Label { text: root.canvasView ? Math.round(root.canvasView.zoom*100)+"%" : ""; color: Theme.text; font.pixelSize: 24 }
            Button { text: "适合窗口  F"; Layout.fillWidth: true; onClicked: { if (root.canvasView) root.canvasView.fitToView() } }
            Button { text: "实际像素  100%"; Layout.fillWidth: true; onClicked: { if (root.canvasView) root.canvasView.actualSize() } }
            Item { Layout.fillHeight: true }
        }
    }
}
