import QtQuick
import QtQuick.Controls
import "."
ApplicationWindow {
    id:root
    required property var ownerTile
    property var canvasView
    property string selectedPanel:""
    property var groupData:({id:"",panels:[],active:"",dockHeight:320})
    property real slideOffset:0
    objectName:"panelFlyout:"+groupData.id
    flags:Qt.Tool|Qt.FramelessWindowHint;visible:false;color:Theme.line
    font.family:Qt.platform.os==="windows"?"Microsoft YaHei UI":"sans-serif";font.pixelSize:10
    width:Math.max(190,ownerTile.layoutData.location==="left"?Workspace.leftDockWidth:Workspace.rightDockWidth)
    height:Math.min(620,Math.max(280,ownerTile.height))
    function positionSide(wantedY){
        const at=ownerTile.mapToGlobal(0,0),bounds=Workspace.availableScreenGeometry(root)
        const area=ownerTile.workspace,top=area.mapToGlobal(0,0).y,bottom=top+area.height
        const low=Math.max(top,bounds.y),visibleBottom=Math.min(bottom,bounds.y+bounds.height)
        height=Math.min(height,Math.max(1,visibleBottom-low))
        const left=at.x-width,right=at.x+ownerTile.width
        x=ownerTile.layoutData.location==="left"?right:left
        if(x<bounds.x)x=right
        if(x+width>bounds.x+bounds.width)x=left
        x=Math.max(bounds.x,Math.min(x,bounds.x+bounds.width-width))
        const high=Math.max(low,visibleBottom-height)
        y=Math.max(low,Math.min(wantedY,high))
    }
    function showPanel(panel){
        selectedPanel=panel;groupData=Workspace.groupDefinition(ownerTile.layoutData.id)
        if(!visible){positionSide(ownerTile.mapToGlobal(0,0).y);visible=true}
        Workspace.watchPanelFlyout(root,ownerTile,true)
    }
    Connections {
        target:Workspace
        function onGroupStateChanged(id){if(id===root.groupData.id){root.groupData=Workspace.groupDefinition(id);root.selectedPanel=root.groupData.active}}
        function onGroupsChanged(){
            const data=Workspace.groupDefinition(root.groupData.id)
            if(!data.id || !data.icons || data.panels.indexOf(root.selectedPanel)<0)root.close()
            else {root.groupData=data;root.selectedPanel=data.active}
        }
        function onDragModifiersChanged(){if(Workspace.dragging)root.close()}
    }
    PanelGroup {
        anchors.fill:parent;anchors.margins:1;groupData:root.groupData;canvasView:root.canvasView;flyout:true
        onDismissRequested:root.close()
        onSlideRequested:globalY=>root.positionSide(globalY-4)
    }
    Component.onDestruction:Workspace.watchPanelFlyout(root,ownerTile,false)
}
