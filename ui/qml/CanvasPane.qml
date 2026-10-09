import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import DrawVerse 1.0
import "."

Item {
    id: root
    property alias canvasView: canvas
    property var parkingParent
    property var mainWindow
    property bool savingBlocked: false
    signal newDocumentRequested()
    function park() { parent=parkingParent }
    function attach(host) { parent=host; width=Qt.binding(()=>host.width);height=Qt.binding(()=>host.height) }
    ColumnLayout {
        anchors.fill: parent; spacing: 0
        Rectangle {
            Layout.fillWidth: true; Layout.preferredHeight: 22; color: Theme.strip
            Rectangle {
                objectName:"documentTab";x:6;y:2;width:Math.max(40,Math.min(parent.width-55,documentTitle.implicitWidth+24));height:parent.height-2
                color:Theme.background;radius:Theme.documentTabRadius
                Rectangle {anchors.left:parent.left;anchors.right:parent.right;anchors.bottom:parent.bottom;height:parent.radius;color:parent.color}
                Label {id:documentTitle;anchors.centerIn:parent;font.pixelSize:9;text:PaintClient.documentName+(PaintClient.modified?" *":"")+"  @ "+Math.round(canvas.zoom*100)+"% · "+PaintClient.documentWidth+" × "+PaintClient.documentHeight;color:Theme.text}
                MouseArea {
                    objectName:"canvasGrip";anchors.fill:parent;property point start
                    onPressed:mouse=>start=Qt.point(mouse.x,mouse.y)
                    onPositionChanged:mouse=>{if(pressed && !PaintClient.drawing && Math.abs(mouse.x-start.x)+Math.abs(mouse.y-start.y)>8)Qt.callLater(()=>Workspace.beginDrag("__canvas","",true))}
                    onDoubleClicked: {if(PaintClient.drawing)return;if(Workspace.groupDefinition("__canvas").host==="main")Workspace.detachGroup("__canvas");else Workspace.returnGroup("__canvas")}
                }
            }
            IconButton {anchors.right:canvasMenuButton.left;width:24;height:22;padding:4;glyph:"plus";tooltip:"新建画布";onClicked:root.newDocumentRequested()}
            IconButton {
                id:canvasMenuButton;objectName:"canvasPanelMenu";anchors.right:parent.right;width:22;height:22;padding:6;glyph:"menu";tooltip:"画布菜单"
                onClicked: {if(Workspace.groupDefinition("__canvas").host!=="main")canvasMenu.popupBeside(root);else canvasMenu.popup()}
                GlassMenu {
                    id:canvasMenu;objectName:"canvasOperationsMenu"
                    GlassMenuItem {text:"浮动画布";enabled:!PaintClient.drawing;onTriggered:Qt.callLater(()=>Workspace.detachGroup("__canvas"))}
                    GlassMenuItem {text:"画布返回工作区";enabled:!PaintClient.drawing;onTriggered:Qt.callLater(()=>Workspace.returnGroup("__canvas"))}
                }
            }
        }
        Rectangle {
            Layout.fillWidth:true;Layout.fillHeight:true;color:Theme.background;clip:true
            Rectangle {x:canvas.documentRect.x+8;y:canvas.documentRect.y+10;width:canvas.documentRect.width;height:canvas.documentRect.height;color:"#0c0d0f"}
            TransparencyGrid {objectName:"canvasTransparency";x:canvas.documentRect.x;y:canvas.documentRect.y;width:canvas.documentRect.width;height:canvas.documentRect.height}
            PaintCanvas {id:canvas;initialFitRatio:.76;objectName:"mainCanvas";anchors.fill:parent;client:PaintClient;focus:true;enabled:!root.savingBlocked}
            Loader {
                objectName:"selectionOutlineLoader";anchors.fill:parent
                active:PaintClient.selectionEnabled || canvas.selectionPreview.width>0 && canvas.selectionPreview.height>0
                sourceComponent:SelectionOutline {objectName:"selectionOutline";enabledSelection:PaintClient.selectionEnabled;steps:PaintClient.selectionSteps;documentRect:canvas.documentRect;zoom:canvas.zoom;preview:canvas.selectionPreview;previewKind:canvas.selectionPreviewKind}
            }
            BusyIndicator {objectName:"canvasBusy";anchors.centerIn:parent;running:!PaintClient.ready;visible:running}
        }
    }
    Shortcut {sequences:[StandardKey.Undo];context:Qt.ApplicationShortcut;enabled:canvas.activeFocus && canvas.Window.window!==root.mainWindow && !PaintClient.drawing;onActivated:PaintClient.undo()}
    Shortcut {sequences:[StandardKey.Redo];context:Qt.ApplicationShortcut;enabled:canvas.activeFocus && canvas.Window.window!==root.mainWindow && !PaintClient.drawing;onActivated:PaintClient.redo()}
}
