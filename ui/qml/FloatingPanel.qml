import QtQuick
import QtQuick.Controls
import "."
PanelWindow {
    id: root
    required property var windowData
    property var canvasPane
    property var canvasView
    property bool appClosing:false
    property bool initializing: true
    property int expandedWidth:windowData.width
    readonly property bool onlyTools:windowData.groups.length===1 && windowData.groups[0]==="__toolstrip"
    objectName:onlyTools?"floatingToolStrip":"floatingDock:"+windowData.id
    // A real top-level window, not a Qt.Tool: Windows gives tool windows the WS_EX_TOOLWINDOW style,
    // which hides them from capture sources such as OBS. It can still be activated, which the
    // window drag and the dock interactions rely on.
    visible:true;flags:Qt.Window|Qt.FramelessWindowHint
    title:"DrawVerse · "+windowData.panels.map(p=>Workspace.panelDefinition(p).title).join(" / ")
    color:windowData.icons?Theme.surface:"transparent"
    Rectangle {anchors.fill:parent;color:"transparent";border.color:Theme.line}
    Rectangle {objectName:"floatingColumnBody";x:1;y:windowData.icons?1:29;width:parent.width-2;height:parent.height-y-1;color:Theme.surface}
    x:windowData.x;y:windowData.y;width:windowData.width;height:windowData.height
    minimumWidth:onlyTools?38:Math.min(windowData.minimumWidth,Workspace.availableScreenGeometry(root).width)
    minimumHeight:onlyTools?304:Math.min(windowData.minimumHeight,Workspace.availableScreenGeometry(root).height)
    maximumWidth:onlyTools?74:windowData.icons?windowData.minimumWidth:16777215
    maximumHeight:16777215
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
