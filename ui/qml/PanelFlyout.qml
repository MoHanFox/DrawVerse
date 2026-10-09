import QtQuick
import QtQuick.Controls
import QtQuick.Window
import "."
ApplicationWindow {
    id:root
    required property var ownerTile
    required property string panelKey
    property var canvasView
    property string selectedPanel:panelKey
    property var groupData:({id:"",panels:[],active:"",dockHeight:320})
    readonly property var sourceWindow:ownerTile.Window.window
    property real lastSourceY:0
    objectName:"panelFlyout:"+groupData.id+":"+panelKey
    flags:Qt.Tool|Qt.FramelessWindowHint;visible:false;color:Theme.line
    font.family:Qt.platform.os==="windows"?"Microsoft YaHei UI":"sans-serif";font.pixelSize:10
    readonly property string panelKind:Workspace.panelDefinition(panelKey).kind
    minimumWidth:Math.min(190,Workspace.availableScreenGeometry(root).width)
    minimumHeight:Math.min(panelKind==="layers"?278:panelKind==="brush-settings"?288:148,Workspace.availableScreenGeometry(root).height)
    width:320
    height:panelKind==="brush" || panelKind==="brush-settings"?540:440
    function refreshGroup(){const data=Workspace.groupDefinition(ownerTile.layoutData.id);groupData=Object.assign({},data,{panels:[panelKey],active:panelKey})}
    function positionSide(wantedY){
        if(!ownerTile)return
        const at=ownerTile.mapToGlobal(0,0),bounds=Workspace.availableScreenGeometry(root)
        const floating=ownerTile.layoutData.location==="floating"
        const area=ownerTile.workspace,areaTop=area.mapToGlobal(0,0).y
        const low=floating?bounds.y:Math.max(areaTop,bounds.y)
        const bottom=floating?bounds.y+bounds.height:Math.min(areaTop+area.height,bounds.y+bounds.height)
        height=Math.min(height,Math.max(minimumHeight,bottom-low));width=Math.min(width,bounds.width)
        const left=at.x-width,right=at.x+ownerTile.width
        x=ownerTile.layoutData.location==="left"?right:left
        if(x<bounds.x)x=right
        if(x+width>bounds.x+bounds.width)x=left
        x=Math.max(bounds.x,Math.min(x,bounds.x+bounds.width-width))
        const screenHigh=Math.max(low,bottom-height)
        const navLow=floating?Math.max(low,Math.min(at.y,screenHigh)):low
        const navHigh=floating?Math.max(navLow,Math.min(screenHigh,at.y+ownerTile.height-8)):screenHigh
        y=Math.max(navLow,Math.min(wantedY,navHigh))
    }
    function showPanel(panel){
        selectedPanel=panel;refreshGroup()
        if(!visible){
            lastSourceY=sourceWindow.y
            const anchor=ownerTile.iconAnchor(panel).y,bounds=Workspace.availableScreenGeometry(root)
            if(ownerTile.layoutData.location==="floating")height=Math.min(height,Math.max(minimumHeight,bounds.y+bounds.height-anchor))
            positionSide(anchor);visible=true
        }
        Workspace.watchPanelFlyout(root,ownerTile,true);raise();requestActivate()
    }
    onWidthChanged:if(visible)positionSide(y)
    onHeightChanged:if(visible)positionSide(y)
    Connections {
        target:root.sourceWindow
        function onXChanged(){if(root.visible)root.positionSide(root.y)}
        function onYChanged(){const delta=root.sourceWindow.y-root.lastSourceY;root.lastSourceY=root.sourceWindow.y;if(root.visible)root.positionSide(root.y+delta)}
    }
    Connections {
        target:root.ownerTile
        function onXChanged(){if(root.visible)Qt.callLater(()=>root.positionSide(root.y))}
        function onYChanged(){if(root.visible)Qt.callLater(()=>root.positionSide(root.y))}
    }
    Connections {
        target:Workspace
        function onGroupStateChanged(id){if(id===root.groupData.id)root.refreshGroup()}
        function onGroupsChanged(){
            const data=Workspace.groupDefinition(root.groupData.id)
            if(!data.id || !data.icons || data.panels.indexOf(root.panelKey)<0)root.close()
            else {root.refreshGroup();if(root.visible)root.positionSide(root.y)}
        }
        function onDragModifiersChanged(){if(Workspace.dragging && Workspace.dragGroups.indexOf(root.groupData.id)>=0)root.close()}
    }
    PanelGroup {
        anchors.fill:parent;anchors.margins:1;groupData:root.groupData;canvasView:root.canvasView;flyout:true
        onDismissRequested:root.close()
        onSlideRequested:globalY=>root.positionSide(globalY-4)
    }
    ResizeFrame {objectName:"panelFlyoutResizeFrame";targetWindow:root}
    Component.onDestruction:Workspace.watchPanelFlyout(root,ownerTile,false)
}
