import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import DrawVerse 1.0
import "../brushes"
import "../canvas"
import "../color"
import "../controls"
import "../layers"
import "../theme"

Item {
    id: root
    property string panelId
    property var canvasView
    readonly property var definition: Workspace.panelDefinition(panelId)
    readonly property string kind: definition.kind || ""
    Loader {
        id:content
        // Loader is a focus scope, including asynchronously created panel content.
        focus: true
        objectName: "panelContentLoader:"+root.panelId
        anchors.fill: parent
        anchors.margins: 5
        // Hiding a native window does not hide its content item's visible property.
        // Cancel pending incubation for hidden windows and closing clients as well.
        readonly property bool requested:root.visible && root.Window.window !== null && root.Window.window.visible && !PaintClient.closing
        readonly property Component requestedComponent:root.kind === "color" || root.kind === "palette" ? colorPanel : root.kind === "brush-settings" || (root.kind === "brush" && root.definition.custom) ? brushPanel : root.kind === "brush" ? brushLibrary : root.kind === "layers" ? layersPanel : root.kind === "history" ? historyPanel : root.kind === "navigator" ? navigatorPanel : null
        active:false
        function synchronizeContent() {
            if(status===Loader.Loading && (!requested || sourceComponent!==requestedComponent))asynchronous=false
            if(!requested) {active=false;return}
            asynchronous=root.kind==="layers"
            sourceComponent=requestedComponent
            active=true
        }
        onRequestedChanged:synchronizeContent()
        onRequestedComponentChanged:synchronizeContent()
        Component.onCompleted:synchronizeContent()
    }
    Component {
        id: colorPanel
        ColumnLayout {
            spacing: 3
            RowLayout {
                Layout.fillWidth: true; spacing: 6
                Item {
                    Layout.preferredWidth: 36; Layout.preferredHeight: 48
                    Rectangle {objectName:"secondaryColorSwatch";x:9;y:14;width:25;height:25;color:PaintClient.secondaryBrushColor;border.color:Theme.line
                        MouseArea {anchors.fill:parent;onClicked:PaintClient.swapBrushColors()}
                    }
                    Rectangle {objectName:"foregroundColorSwatch";width:25;height:25;color:PaintClient.brushColor;border.color:Theme.line
                        MouseArea {anchors.fill:parent;onClicked:PaintClient.swapBrushColors()}
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
                objectName:"recentColorGrid"
                visible:parent.height>245
                Layout.fillWidth:true;columns:12;rowSpacing:2;columnSpacing:2
                Repeater {
                    // Recently used colours, newest first. Empty on a fresh profile.
                    model:PaintClient.recentColors
                    Rectangle {required property string modelData;Layout.fillWidth:true;Layout.preferredHeight:12;color:modelData;border.color:Theme.line
                        MouseArea {anchors.fill:parent;onClicked:PaintClient.brushColor=modelData}
                    }
                }
            }
        }
    }
    Component { id: brushLibrary; BrushLibraryPanel {canvasView:root.canvasView} }
    Component { id: brushPanel; BrushSettingsPanel {} }
    Component { id: layersPanel; LayersPanel {canvasView:root.canvasView} }
    Component {
        id: historyPanel
        ColumnLayout {
            id: historyContent
            property bool restoringHistory:false
            function restoreHistoryScroll(){restoringHistory=true;Qt.callLater(()=>{historyView.contentY=Math.min(PaintClient.documentHistoryScrollPosition(),Math.max(0,historyView.contentHeight-historyView.height));restoringHistory=false})}
            Component.onCompleted:if(!PaintClient.liquifyActive && !PaintClient.transformActive)restoreHistoryScroll()
            Connections {target:PaintClient;function onHistoryScopeChanging(liquify){if(!liquify)historyContent.restoringHistory=true}function onHistoryChanged(){if(historyContent.restoringHistory)historyContent.restoreHistoryScroll()}}
            spacing: 1
            ListView {
                id:historyView
                objectName:"historyList"
                Layout.fillWidth:true;Layout.fillHeight:true;clip:true
                model:PaintClient.historyEntries
                currentIndex:PaintClient.historyEntries.findIndex(entry=>entry.depth===PaintClient.undoDepth)
                onContentYChanged:if(!historyContent.restoringHistory)PaintClient.rememberDocumentHistoryScroll(contentY)
                onCurrentIndexChanged:if(currentIndex>=0 && !historyContent.restoringHistory)Qt.callLater(()=>{if(!historyContent.restoringHistory)positionViewAtIndex(currentIndex,ListView.Contain)})
                delegate:Rectangle {
                    required property int index
                    required property var modelData
                    property int depth:modelData.depth
                    width:ListView.view.width;height:24
                    objectName:"historyEntry:"+depth
                    color:depth===PaintClient.undoDepth?Theme.selected:"transparent"
                    Row {anchors.verticalCenter:parent.verticalCenter;x:5;spacing:7
                        Icon {objectName:"historyIcon:"+depth;name:modelData.icon;width:12;height:12;color:depth>PaintClient.undoDepth?Theme.disabled:Theme.text}
                        Text {objectName:"historyTitle:"+depth;text:modelData.title;color:depth>PaintClient.undoDepth?Theme.disabled:Theme.text;font.pixelSize:9}
                    }
                    MouseArea {anchors.fill:parent;enabled:PaintClient.ready&&!PaintClient.drawing&&!PaintClient.fileBusy&&!PaintClient.layerEditBusy&&!PaintClient.liquifyBusy&&!PaintClient.transformBusy;onClicked:PaintClient.selectHistory(parent.depth)}
                }
                ScrollBar.vertical:ScrollBar {}
            }

        }
    }
    Component {
        id: navigatorPanel
        Rectangle {
            objectName:"navigatorPreview";color:Theme.background;radius:2;clip:true
            TransparencyGrid {objectName:"navigatorTransparency";x:mini.documentRect.x;y:mini.documentRect.y;width:mini.documentRect.width;height:mini.documentRect.height}
            PaintCanvas {id:mini;objectName:"navigatorCanvas";anchors.fill:parent;client:PaintClient;interactive:false}
            Rectangle {
                objectName:"navigatorViewFrame"
                readonly property rect region:root.canvasView?root.canvasView.visibleDocumentRect:Qt.rect(0,0,0,0)
                x:mini.documentRect.x+region.x*mini.zoom;y:mini.documentRect.y+region.y*mini.zoom
                width:region.width*mini.zoom;height:region.height*mini.zoom
                visible:region.width>0 && region.height>0 && root.canvasView && root.canvasView.visible && root.canvasView.client===PaintClient
                color:"transparent";border.color:"#ff3434";border.width:1;radius:2
            }
        }
    }
}
