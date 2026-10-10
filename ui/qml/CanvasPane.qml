import QtQuick
import QtQuick.Controls
import QtQuick.Window
import DrawVerse 1.0
import "."
Rectangle {
    id:root
    property var ownClient: PaintClient
    property string documentId: ""
    property bool initialDocument: false
    property var parkingParent
    property var mainWindow
    property bool savingBlocked: false
    property alias canvasView:canvas
    color:Theme.background;clip:true
    function park(){parent=parkingParent;visible=false}
    function attach(host){parent=host;width=Qt.binding(()=>host.width);height=Qt.binding(()=>host.height);visible=true;canvas.forceActiveFocus()}
    TransparencyGrid {objectName:"canvasTransparency";x:canvas.documentRect.x;y:canvas.documentRect.y;width:canvas.documentRect.width;height:canvas.documentRect.height}
    PaintCanvas {id:canvas;initialFitRatio:.76;objectName:root.initialDocument?"mainCanvas":"documentCanvas:"+root.documentId;anchors.fill:parent;client:root.ownClient;focus:true;enabled:!root.savingBlocked}
    Rectangle {
        objectName:"brushCursorOutline";z:10
        x:canvas.brushCursorRect.x;y:canvas.brushCursorRect.y;width:canvas.brushCursorRect.width;height:canvas.brushCursorRect.height
        visible:canvas.brushCursorVisible;color:"transparent";radius:width/2;border.width:1;border.color:"black"
        Rectangle {anchors.fill:parent;anchors.margins:1;radius:width/2;color:"transparent";border.width:1;border.color:"white"}
    }
    Loader {
        objectName:"selectionOutlineLoader";anchors.fill:parent
        active:root.ownClient.selectionEnabled || canvas.selectionPreview.width>0 && canvas.selectionPreview.height>0
        sourceComponent:SelectionOutline {objectName:"selectionOutline";enabledSelection:root.ownClient.selectionEnabled;steps:root.ownClient.selectionSteps;documentRect:canvas.documentRect;zoom:canvas.zoom;viewRotation:canvas.viewRotation;preview:canvas.selectionPreview;previewKind:canvas.selectionPreviewKind}
    }
    BusyIndicator {objectName:"canvasBusy";anchors.centerIn:parent;running:!root.ownClient.ready;visible:running}
    Shortcut {sequences:[StandardKey.Undo];context:Qt.ApplicationShortcut;enabled:root.visible && canvas.activeFocus && canvas.Window.window!==root.mainWindow && !root.ownClient.drawing;onActivated:root.ownClient.undo()}
    Shortcut {sequences:[StandardKey.Redo];context:Qt.ApplicationShortcut;enabled:root.visible && canvas.activeFocus && canvas.Window.window!==root.mainWindow && !root.ownClient.drawing;onActivated:root.ownClient.redo()}
}
