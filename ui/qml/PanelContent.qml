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
        anchors.margins: 5
        active: root.visible
        sourceComponent: root.kind === "color" || root.kind === "palette" ? colorPanel : root.kind === "brush-settings" || (root.kind === "brush" && root.definition.custom) ? brushPanel : root.kind === "brush" ? brushLibrary : root.kind === "layers" ? layersPanel : root.kind === "history" ? historyPanel : root.kind === "navigator" ? navigatorPanel : null
    }
    Component {
        id: colorPanel
        ColumnLayout {
            spacing: 3
            RowLayout {
                Layout.fillWidth: true; spacing: 6
                Item {
                    Layout.preferredWidth: 36; Layout.preferredHeight: 48
                    Rectangle {x:9;y:14;width:25;height:25;color:"white";border.color:Theme.line}
                    Rectangle {width:25;height:25;color:PaintClient.brushColor;border.color:Theme.line
                        MouseArea {anchors.fill:parent;onDoubleClicked:hexPopup.open();hoverEnabled:true;ToolTip.visible:containsMouse;ToolTip.text:"双击输入 HEX 颜色"}
                    }
                }
                ColumnLayout {
                    Layout.fillWidth: true; spacing: 0
                    Repeater {
                        model: ["H","S","B"]
                        RowLayout {
                            required property string modelData
                            required property int index
                            Layout.fillWidth: true; spacing: 3
                            Label {text:modelData;color:Theme.muted;font.pixelSize:8;Layout.preferredWidth:7}
                            HsvSlider {
                                Layout.fillWidth: true; Layout.minimumWidth: 24
                                axis:index;hue:wheel.hue;saturation:wheel.saturation;brightness:wheel.value
                                value:index===0?wheel.hue:index===1?wheel.saturation:wheel.value
                                onMoved:wheel.pickHsv(index===0?value:wheel.hue,index===1?value:wheel.saturation,index===2?value:wheel.value)
                            }
                            CompactSpinBox {
                                objectName:"hsvValue:"+modelData
                                Layout.preferredWidth:30;implicitHeight:16;leftPadding:2;rightPadding:2;topPadding:0;bottomPadding:0;font.pixelSize:8
                                from:0;to:index===0?360:100;editable:true
                                value:Math.round((index===0?wheel.hue:index===1?wheel.saturation:wheel.value)*to)
                                up.indicator.visible:false;down.indicator.visible:false
                                onValueModified:wheel.pickHsv(index===0?value/360:wheel.hue,index===1?value/100:wheel.saturation,index===2?value/100:wheel.value)
                            }
                        }
                    }
                }
            }
            ColorWheel {
                id:wheel;objectName:"colorWheel"
                Layout.fillWidth:true;Layout.fillHeight:true;Layout.preferredHeight:145;Layout.minimumHeight:100;Layout.maximumHeight:190
                color:PaintClient.brushColor;onPicked:color=>PaintClient.brushColor=color
            }
            GridLayout {
                visible:parent.height>245
                Layout.fillWidth:true;columns:12;rowSpacing:2;columnSpacing:2
                Repeater {
                    model:["#000000","#3a3a3a","#6e6e6e","#a8a8a8","#d8d8d8","#ffffff","#8c2b2b","#d6453f","#f08a3c","#f5d14e","#7fb84b","#3e9e7a"]
                    Rectangle {required property string modelData;Layout.fillWidth:true;Layout.preferredHeight:12;color:modelData
                        MouseArea {anchors.fill:parent;onClicked:PaintClient.brushColor=modelData}
                    }
                }
            }
            Popup {
                id:hexPopup;width:160;closePolicy:Popup.CloseOnEscape|Popup.CloseOnPressOutside
                contentItem:TextField {objectName:"brushColorHex";text:String(PaintClient.brushColor).toUpperCase();selectByMouse:true;maximumLength:7
                    validator:RegularExpressionValidator {regularExpression:/#[0-9a-fA-F]{6}/}
                    onAccepted:{if(acceptableInput){PaintClient.brushColor=text;hexPopup.close()}}
                }
            }
        }
    }
    Component { id: brushLibrary; BrushLibraryPanel {} }
    Component { id: brushPanel; BrushSettingsPanel {} }
    Component { id: layersPanel; LayersPanel {} }
    Component {
        id: historyPanel
        ColumnLayout {
            id: historyContent
            property int targetDepth: -1
            property var generation: PaintClient.generation
            onGenerationChanged: targetDepth=-1
            spacing: 1
            ListView {
                objectName:"historyList"
                Layout.fillWidth:true;Layout.fillHeight:true;clip:true
                model:Math.min(256,PaintClient.undoDepth+PaintClient.redoDepth+1)
                property int offset:Math.max(0,PaintClient.undoDepth+PaintClient.redoDepth+1-256)
                currentIndex:Math.max(0,PaintClient.undoDepth-offset)
                onCurrentIndexChanged:Qt.callLater(()=>positionViewAtIndex(currentIndex,ListView.Contain))
                delegate:Rectangle {
                    required property int index
                    property int depth:index+ListView.view.offset
                    width:ListView.view.width;height:24
                    objectName:"historyEntry:"+depth
                    color:depth===PaintClient.undoDepth?Theme.selected:"transparent"
                    Row {anchors.verticalCenter:parent.verticalCenter;x:5;spacing:7
                        Icon {name:depth===0?"page":"history";width:12;height:12;color:depth>PaintClient.undoDepth?Theme.disabled:Theme.text}
                        Text {text:depth===0?"初始状态":"操作 "+depth;color:depth>PaintClient.undoDepth?Theme.disabled:Theme.text;font.pixelSize:9}
                    }
                    MouseArea {anchors.fill:parent;enabled:!PaintClient.drawing&&!PaintClient.fileBusy;onClicked:historyContent.targetDepth=parent.depth}
                }
                ScrollBar.vertical:ScrollBar {}
            }
            Timer {
                interval:20;repeat:true;running:historyContent.targetDepth>=0
                onTriggered:{if(historyContent.targetDepth===PaintClient.undoDepth){historyContent.targetDepth=-1;return}if(!PaintClient.ready||PaintClient.drawing||PaintClient.fileBusy||PaintClient.layerEditBusy)return;if(historyContent.targetDepth<PaintClient.undoDepth)PaintClient.undo();else PaintClient.redo()}
            }
            RowLayout {
                Layout.fillWidth:true
                Item {Layout.fillWidth:true}
                IconButton {glyph:"undo";implicitWidth:22;implicitHeight:20;tooltip:"撤销 Ctrl+Z";enabled:PaintClient.undoDepth>0&&!PaintClient.drawing;onClicked:{historyContent.targetDepth=-1;PaintClient.undo()}}
                IconButton {glyph:"redo";implicitWidth:22;implicitHeight:20;tooltip:"重做 Ctrl+Shift+Z";enabled:PaintClient.redoDepth>0&&!PaintClient.drawing;onClicked:{historyContent.targetDepth=-1;PaintClient.redo()}}
            }
        }
    }
    Component {
        id: navigatorPanel
        ColumnLayout {
            Rectangle {
                Layout.fillWidth: true; Layout.preferredHeight: 170; color: Theme.background; radius: 2
                TransparencyGrid { objectName: "navigatorTransparency"; x: mini.documentRect.x; y: mini.documentRect.y; width: mini.documentRect.width; height: mini.documentRect.height }
                PaintCanvas { id: mini; objectName: "navigatorCanvas"; anchors.fill: parent; client: PaintClient; interactive: false }
            }
            Label { text: PaintClient.documentWidth+" × "+PaintClient.documentHeight+" px"; color: Theme.muted }
            Label { text: root.canvasView ? Math.round(root.canvasView.zoom*100)+"%" : ""; color: Theme.text; font.pixelSize: 18 }
            Button { text: "适合窗口  F"; Layout.fillWidth: true; onClicked: { if (root.canvasView) root.canvasView.fitToView() } }
            Button { text: "实际像素  100%"; Layout.fillWidth: true; onClicked: { if (root.canvasView) root.canvasView.actualSize() } }
            Item { Layout.fillHeight: true }
        }
    }
}
