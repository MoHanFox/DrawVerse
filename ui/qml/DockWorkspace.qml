import QtQuick
import "."

Item {
    id: root
    property string hostId: "main"
    property var canvasPane
    property var canvasView
    property var views: ({})
    property bool ready: false
    function releaseCanvas() {
        for(const id in views) if(views[id].releaseCanvas) views[id].releaseCanvas()
    }
    function refresh() {
        if(!ready || !canvasPane) return
        const items=Workspace.layoutItems(hostId,Math.round(width),Math.round(height)), live={}
        for(let i=0;i<items.length;i++) {
            const data=items[i];live[data.id]=true
            if(views[data.id]) views[data.id].updateLayout(data)
            else views[data.id]=(data.kind==="leaf"?leafFactory:splitFactory).createObject(root,{layoutData:data,workspace:root})
        }
        for(const id in views) if(!live[id]) {
            if(views[id].releaseCanvas) views[id].releaseCanvas()
            views[id].visible=false;views[id].destroy();delete views[id]
        }
    }
    onWidthChanged: Qt.callLater(refresh)
    onHeightChanged: Qt.callLater(refresh)
    Component.onCompleted: {ready=true;refresh();if(hostId==="main")Workspace.registerWorkspace(root)}
    Component.onDestruction: releaseCanvas()
    Connections {
        target:Workspace
        function onGroupsChanged(){root.refresh()}
        function onDockMetricsChanged(){root.refresh()}
        function onLayoutChanged(){root.refresh()}
    }
    Component {id:leafFactory;DockTile {}}
    Component {id:splitFactory;DockDivider {}}
    DropArea {
        objectName:root.hostId==="main"?"toolStripDockTarget":"emptyDockTarget:"+root.hostId
        anchors.fill:parent;z:-1
        keys:["application/x-drawverse-panel","application/x-drawverse-tool-strip"]
        onEntered:d=>d.accepted=root.hostId==="main" && !Workspace.dockingSuppressed && !PaintClient.drawing && (d.x<=24 || d.x>=root.width-24)
        onPositionChanged:d=>d.accepted=root.hostId==="main" && !Workspace.dockingSuppressed && !PaintClient.drawing && (d.x<=24 || d.x>=root.width-24)
        onDropped:d=> {
            if(Workspace.dockingSuppressed || PaintClient.drawing) return
            if(root.hostId!=="main" || d.x>24 && d.x<root.width-24)return
            const side=d.x<=24?"left":"right"
            const ok=d.formats.indexOf("application/x-drawverse-tool-strip")>=0?Workspace.dockToolStrip(d.getDataAsString("application/x-drawverse-tool-strip"),side):Workspace.dockPayload(d.getDataAsString("application/x-drawverse-panel"),side)
            if(ok)d.acceptProposedAction()
        }
    }
    DockEdgePreview {objectName:"workspaceDockPreview";anchors.fill:parent;visible:root.hostId==="main" && Workspace.dragTarget.indexOf("__workspace_")===0;mode:Workspace.dragTarget.endsWith("right")?"right":"left"}
}
