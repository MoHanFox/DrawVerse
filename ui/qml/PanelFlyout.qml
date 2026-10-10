import QtQuick
import QtQuick.Controls
import QtQuick.Window
import "."
PanelWindow {
    id:root
    required property var ownerTile
    required property string panelKey
    property var canvasView
    property string selectedPanel:panelKey
    property var groupData:({id:"",panels:[],groups:[],dockHeight:320})
    readonly property var sourceWindow:ownerTile.Window.window
    property real lastSourceY:0
    property bool positioning:false
    property bool restoring:false
    objectName:"panelFlyout:"+groupData.id+":"+panelKey
    flags:Qt.Tool|Qt.FramelessWindowHint;visible:false;color:"transparent"
    Rectangle {anchors.fill:parent;color:"transparent";border.color:Theme.line}
    readonly property string panelKind:Workspace.panelDefinition(panelKey).kind
    minimumWidth:Math.min(Math.max(190,groupData.minimumWidth||190),Workspace.availableScreenGeometry(root).width)
    minimumHeight:Math.min(Math.max(148,groupData.minimumHeight||148),Workspace.availableScreenGeometry(root).height)
    width:320
    height:panelKind==="brush" || panelKind==="brush-settings"?540:440
    function refreshGroup(){groupData=Workspace.categoryDefinition(ownerTile.layoutData.id)}
    function remember(){if(visible && !positioning && !restoring && ownerTile)for(const panel of groupData.panels)Workspace.updatePanelView(panel,width,height,Math.round(y-ownerTile.mapToGlobal(0,0).y))}
    function positionSide(wantedY){
        if(!ownerTile || positioning)return
        positioning=true
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
        y=Math.round(Math.max(navLow,Math.min(wantedY,navHigh)))
        positioning=false;remember()
    }
    function showPanel(panel){
        selectedPanel=panel;refreshGroup();Workspace.setActive(Workspace.groupForPanel(panel),panel)
        if(!visible){
            restoring=true
            lastSourceY=sourceWindow.y
            const view=Workspace.panelView(panel),bounds=Workspace.availableScreenGeometry(root)
            const anchor=view.width?ownerTile.mapToGlobal(0,0).y+view.offset:ownerTile.iconAnchor(panel).y
            if(view.width){width=Math.max(minimumWidth,view.width);height=Math.max(minimumHeight,view.height)}
            else if(ownerTile.layoutData.location==="floating")height=Math.min(height,Math.max(minimumHeight,bounds.y+bounds.height-anchor))
            positionSide(anchor);visible=true
            restoring=false;remember()
        }
        Workspace.watchPanelFlyout(root,ownerTile,true);raise();requestActivate()
    }
    onWidthChanged:if(visible)positionSide(y)
    onHeightChanged:if(visible)positionSide(y)
    onYChanged:remember()
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
        function onGroupStateChanged(id){if(root.groupData.groups.indexOf(id)>=0){root.selectedPanel=Workspace.groupDefinition(id).active;root.refreshGroup()}}
        function onGroupsChanged(){
            const data=Workspace.categoryDefinition(root.groupData.id)
            if(!Workspace.groupDefinition(root.groupData.id).icons || data.panels.indexOf(root.panelKey)<0 || data.groups.join("|")!==root.groupData.groups.join("|"))root.close()
            else {root.refreshGroup();if(root.visible)root.positionSide(root.y)}
        }
        function onDragModifiersChanged(){if(Workspace.dragging && Workspace.dragGroups.indexOf(root.groupData.id)>=0)root.close()}
    }
    DockWorkspace {anchors.fill:parent;anchors.margins:1;hostId:"__category:"+root.ownerTile.layoutData.id;canvasPane:root.ownerTile.workspace.canvasPane;canvasView:root.canvasView;flyoutHost:root}
    ResizeFrame {objectName:"panelFlyoutResizeFrame";targetWindow:root}
    onClosing:{remember();Workspace.watchPanelFlyout(root,ownerTile,false);ownerTile.removeFlyout(root);visible=false;Qt.callLater(()=>root.destroy())}
    Component.onDestruction:Workspace.watchPanelFlyout(root,ownerTile,false)
}
