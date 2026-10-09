import QtQuick
import QtQuick.Controls
import "."
ApplicationWindow {
    id: root
    required property var windowData
    property var canvasPane
    property var canvasView
    property bool appClosing:false
    property bool initializing: true
    property int expandedWidth:windowData.width
    readonly property bool onlyTools:windowData.groups.length===1 && windowData.groups[0]==="__toolstrip"
    objectName:onlyTools?"floatingToolStrip":"floatingDock:"+windowData.id
    visible:true;flags:Qt.Tool|Qt.FramelessWindowHint
    font.family:Qt.platform.os==="windows"?"Microsoft YaHei UI":"sans-serif";font.pixelSize:10
    title:"DrawVerse · "+windowData.panels.map(p=>Workspace.panelDefinition(p).title).join(" / ")
    color:windowData.icons?Theme.surface:Theme.line
    palette.window:Theme.surface;palette.windowText:Theme.text;palette.base:Theme.background;palette.text:Theme.text
    palette.button:Theme.raised;palette.buttonText:Theme.text;palette.highlight:Theme.selected;palette.highlightedText:Theme.accent
    x:windowData.x;y:windowData.y;width:windowData.width;height:windowData.height
    minimumWidth:onlyTools?38:Math.min(windowData.minimumWidth,Workspace.availableScreenGeometry(root).width)
    minimumHeight:onlyTools?304:Math.min(windowData.minimumHeight,Workspace.availableScreenGeometry(root).height)
    maximumWidth:onlyTools?38:windowData.icons?windowData.minimumWidth:16777215
    maximumHeight:onlyTools?304:16777215
    DockWorkspace {id:docks;anchors.fill:parent;anchors.margins:1;hostId:root.windowData.id;canvasPane:root.canvasPane;canvasView:root.canvasView}
    ResizeFrame {targetWindow:root}
    function releaseCanvas(){docks.releaseCanvas()}
    function updateLayout(data) {
        initializing=true;windowData=data;x=data.x;y=data.y;expandedWidth=data.width;width=data.icons?data.minimumWidth:data.width
        height=data.height;initializing=false;docks.refresh();remember()
    }
    Component.onCompleted:{if(windowData.icons)width=windowData.minimumWidth;initializing=false}
    Connections {
        target:Workspace
        function onGroupStateChanged(group){
            if(root.windowData.groups.indexOf(group)<0)return
            const live=Workspace.floatingWindows
            for(let i=0;i<live.length;i++)if(live[i].id===root.windowData.id){root.updateLayout(live[i]);break}
        }
    }
    function remember(){
        if(!visible || initializing || visibility===Window.Minimized)return
        if(onlyTools){
            Workspace.updateToolStripPosition(x,y)
            initializing=true;x=Workspace.toolStripX;y=Workspace.toolStripY;initializing=false
        }
        Workspace.updateGeometry(windowData.id,x,y,windowData.icons?expandedWidth:width,height)
    }
    onXChanged:remember();onYChanged:remember();onWidthChanged:remember();onHeightChanged:remember()
    onClosing:event=>{if(appClosing){event.accepted=true;return}event.accepted=false;if(!PaintClient.drawing)Qt.callLater(()=>Workspace.returnWindow(windowData.id))}
}
