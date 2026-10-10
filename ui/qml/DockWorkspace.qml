import QtQuick
import "."

Item {
    id: root
    property string hostId: "main"
    property var canvasPane
    property var canvasView
    property var flyoutHost:null
    property var views: ({})
    property bool ready: false
    function releaseCanvas() {
        for(const id in views) if(views[id].releaseCanvas) views[id].releaseCanvas()
    }
    function refresh() {
        if(!ready || !canvasPane) return
        const view=hostId==="main"?canvasView:null
        if(view)view.captureLayoutPosition()
        const items=Workspace.layoutItems(hostId,Math.round(width),Math.round(height)), live={}
        for(let i=0;i<items.length;i++) {
            const data=items[i];live[data.id]=true
            if(views[data.id] && views[data.id].layoutData.kind!==data.kind){views[data.id].visible=false;views[data.id].destroy();delete views[data.id]}
            if(views[data.id]) views[data.id].updateLayout(data)
            else views[data.id]=(data.kind==="leaf"?leafFactory:data.kind==="railBackground"?railBackgroundFactory:data.kind==="railSeparator"?separatorFactory:splitFactory).createObject(root,{layoutData:data,workspace:root})
        }
        for(const id in views) if(!live[id]) {
            if(views[id].releaseCanvas) views[id].releaseCanvas()
            views[id].visible=false;views[id].destroy();delete views[id]
        }
        if(view)Qt.callLater(()=>{if(view && typeof view.restoreLayoutPosition==="function")view.restoreLayoutPosition()})
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
    Component {
        id:separatorFactory
        Item {
            required property var layoutData
            property var workspace
            objectName:"railCategorySeparator:"+layoutData.id
            x:layoutData.rect.x;y:layoutData.rect.y;width:layoutData.rect.width;height:layoutData.rect.height
            function updateLayout(data){layoutData=data}
            Rectangle {objectName:"railSeparatorLine";anchors.centerIn:parent;width:18;height:1;color:Theme.categorySeparator}
        }
    }
    Component {
        id:railBackgroundFactory
        Rectangle {
            required property var layoutData
            property var workspace
            objectName:layoutData.id
            x:layoutData.rect.x;y:layoutData.rect.y;width:layoutData.rect.width;height:layoutData.rect.height
            z:-.5;color:Theme.surface
            function updateLayout(data){layoutData=data}
        }
    }
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
    DockEdgePreview {
        objectName:"dockColumnPreview:"+root.hostId
        readonly property point origin:root.mapFromGlobal(Workspace.dragPreviewRect.x,Workspace.dragPreviewRect.y)
        x:origin.x;y:origin.y;width:Workspace.dragPreviewRect.width;height:Workspace.dragPreviewRect.height
        visible:Workspace.dragPlacement.indexOf("column-")===0 && Workspace.groupDefinition(Workspace.dragTarget).host===root.hostId
        mode:Workspace.dragPlacement.endsWith("left")?"left":"right"
    }
}
