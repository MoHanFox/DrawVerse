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
    Component.onCompleted: {ready=true;refresh()}
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
        onEntered:d=>d.accepted=!Workspace.dockingSuppressed && !PaintClient.drawing
        onPositionChanged:d=>d.accepted=!Workspace.dockingSuppressed && !PaintClient.drawing
        onDropped:d=> {
            if(Workspace.dockingSuppressed || PaintClient.drawing) return
            const ok=d.formats.indexOf("application/x-drawverse-tool-strip")>=0?Workspace.dockToolStrip(d.getDataAsString("application/x-drawverse-tool-strip")):Workspace.dockPayload(d.getDataAsString("application/x-drawverse-panel"),"main")
            if(ok)d.acceptProposedAction()
        }
    }
}
