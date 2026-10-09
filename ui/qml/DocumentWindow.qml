import QtQuick
import QtQuick.Controls
import "."
ApplicationWindow {
    id:root
    required property var windowData
    required property var documents
    required property var presentation
    objectName:"documentWindow:"+windowData.id
    visible:true;flags:Qt.Window|Qt.FramelessWindowHint
    x:windowData.x;y:windowData.y;width:windowData.width;height:windowData.height
    minimumWidth:320;minimumHeight:200;color:Theme.strip
    font.family:Qt.platform.os==="windows"?"Microsoft YaHei UI":"sans-serif";font.pixelSize:10
    property var activeDocument:documents.client(windowData.active)
    readonly property bool multipleDocuments:windowData.documents.length>1
    title:activeDocument?activeDocument.documentName+(activeDocument.modified?" *":""):"DrawVerse"
    property bool initializing:true
    function remember(){if(!initializing && visible && visibility!==Window.Minimized)documents.updateGeometry(windowData.id,x,y,width,height)}
    function updateLayout(data){initializing=true;windowData=data;x=data.x;y=data.y;width=data.width;height=data.height;initializing=false;area.refresh()}
    function releaseCanvas(){area.releaseCanvas()}
    Component.onCompleted:initializing=false
    onXChanged:remember();onYChanged:remember();onWidthChanged:remember();onHeightChanged:remember()
    onActiveChanged:if(active && windowData.active && documents.activeId!==windowData.active)documents.activate(windowData.active)
    onClosing:event=>{event.accepted=presentation.quitting;if(!event.accepted)documents.returnWindow(windowData.id)}
    Rectangle {
        id:bar;objectName:"documentWindowTitle:"+root.windowData.id;height:root.multipleDocuments?22:0;visible:root.multipleDocuments;width:parent.width;color:Theme.strip
        MouseArea {objectName:root.multipleDocuments?"documentWindowGrip":"hiddenDocumentWindowGrip";anchors.fill:parent;anchors.rightMargin:64;property point start;onPressed:m=>start=Qt.point(m.x,m.y);onPositionChanged:m=>{if(pressed && Math.abs(m.x-start.x)+Math.abs(m.y-start.y)>4)Qt.callLater(()=>root.documents.beginDrag(root.windowData.active,true))};onDoubleClicked:root.documents.returnWindow(root.windowData.id)}
        Text {objectName:"documentWindowName:"+root.windowData.id;x:10;width:parent.width-80;anchors.verticalCenter:parent.verticalCenter;text:root.title;color:Theme.text;font.pixelSize:9;elide:Text.ElideRight}
    }
    Row {
        anchors.right:parent.right;height:22;z:2
        IconButton {width:32;height:22;padding:6;glyph:"minimize";onClicked:root.showMinimized()}
        IconButton {
            id:closeButton;width:32;height:22;padding:6;glyph:"close"
            background:Rectangle {color:closeButton.down?"#b5222c":closeButton.hovered?"#f04450":"transparent"}
            onClicked:root.documents.returnWindow(root.windowData.id)
        }
    }
    DocumentArea {id:area;anchors.top:bar.bottom;anchors.bottom:parent.bottom;anchors.left:parent.left;anchors.right:parent.right;anchors.margins:2;hostId:root.windowData.id;titleControlsWidth:root.multipleDocuments?0:64;documents:root.documents;presentation:root.presentation}
    ResizeFrame {targetWindow:root}
}
