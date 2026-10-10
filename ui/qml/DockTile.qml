import QtQuick
import QtQuick.Controls
import "."

Item {
    id:root
    required property var layoutData
    required property var workspace
    objectName:"dockTile:"+layoutData.id
    x:layoutData.rect.x;y:layoutData.rect.y;width:layoutData.rect.width;height:layoutData.rect.height
    property string dropMode:"right"
    readonly property bool isCanvas:layoutData.id==="__canvas"
    readonly property bool isTools:layoutData.id==="__toolstrip"
    function updateLayout(data){layoutData=data;if(isCanvas && workspace.canvasPane.parent!==canvasHost)workspace.canvasPane.attach(canvasHost)}
    function releaseCanvas(){if(isCanvas && workspace.canvasPane.parent===canvasHost)workspace.canvasPane.park()}
    function iconAnchor(panel){return contentLoader.item && contentLoader.item.iconAnchor?contentLoader.item.iconAnchor(panel):mapToGlobal(0,0)}
    function removeFlyout(window){if(contentLoader.item && contentLoader.item.removePeek)contentLoader.item.removePeek(window)}
    function locateDrop(x,y){
        const distance=Math.min(x,width-x,y,height-y)
        if(distance===x)dropMode="left"
        else if(distance===width-x)dropMode="right"
        else if(distance===y)dropMode="before"
        else dropMode="after"
        dropMode=Workspace.sidePlacement(root.layoutData.id,dropMode)
    }
    Item {id:canvasHost;anchors.fill:parent;visible:root.isCanvas}
    Component.onCompleted:{if(isCanvas)workspace.canvasPane.attach(canvasHost);else if(!workspace.flyoutHost)Workspace.registerTarget(layoutData.id,root)}
    Loader {
        id:contentLoader
        anchors.fill:parent;active:!root.isCanvas
        sourceComponent:root.isTools?tools:root.layoutData.rail?rail:panel
    }
    Component {id:tools;ToolStrip {canvasView:root.workspace.canvasView}}
    Component {
        id:panel
        PanelGroup {
            groupData:root.layoutData;canvasView:root.workspace.canvasView;flyout:!!root.workspace.flyoutHost
            onDismissRequested:if(root.workspace.flyoutHost)root.workspace.flyoutHost.close()
            onSlideRequested:globalY=>{if(root.workspace.flyoutHost)root.workspace.flyoutHost.positionSide(globalY-4)}
        }
    }
    Component {
        id:rail
        Rectangle {
            id:railRoot;objectName:"iconRail:"+root.layoutData.id;color:Theme.surface
            property var peeks:({})
            property var flyoutFactory:Qt.createComponent("PanelFlyout.qml")
            function iconAnchor(panel){for(let i=0;i<icons.count;i++){const icon=icons.itemAt(i);if(icon.modelData===panel)return icon.mapToGlobal(0,0)}return root.mapToGlobal(0,0)}
            function removePeek(window){const next=Object.assign({},peeks);for(const panel in next)if(next[panel]===window)delete next[panel];peeks=next}
            function openPanel(panel){
                let peek=peeks[panel]
                if(peek && peek.visible && peek.selectedPanel===panel){peek.close();return}
                Workspace.setActive(Workspace.groupForPanel(panel),panel)
                if(!peek){peek=flyoutFactory.createObject(railRoot,{ownerTile:root,panelKey:panel,canvasView:Qt.binding(()=>root.workspace.canvasView)});const next=Object.assign({},peeks);for(const key of Workspace.categoryDefinition(root.layoutData.id).panels)next[key]=peek;peeks=next}
                peek.showPanel(panel)
            }
            Component.onDestruction:{const windows=[];for(const panel in peeks)if(peeks[panel] && windows.indexOf(peeks[panel])<0)windows.push(peeks[panel]);for(const window of windows)window.destroy();peeks=({})}
            IconButton {objectName:"dockCollapse:"+root.layoutData.location;visible:root.layoutData.columnFirst;width:28;height:10;padding:2;glyph:"expand";tooltip:"展开面板列";onClicked:Workspace.setColumnCollapsed(root.layoutData.id,false)}
            MouseArea {
                anchors.top:parent.top;anchors.left:parent.left;width:12;height:12;visible:root.layoutData.columnFirst
                property point start
                onPressed:m=>start=Qt.point(m.x,m.y)
                onPositionChanged:m=>{if(pressed && Math.abs(m.x-start.x)+Math.abs(m.y-start.y)>8)Qt.callLater(()=>Workspace.beginDrag(root.layoutData.id,"",true))}
            }
            Column {
                y:root.layoutData.columnFirst?12:0;width:parent.width
                Repeater {
                    id:icons
                    model:root.layoutData.panels
                    IconButton {
                        id:railButton;required property string modelData
                        objectName:"railPanel:"+modelData;width:28;height:28;padding:7
                        checked:!!railRoot.peeks[modelData] && railRoot.peeks[modelData].visible
                        glyph:Workspace.panelDefinition(modelData).kind==="brush-settings"?"settings":Workspace.panelDefinition(modelData).kind;tooltip:Workspace.panelDefinition(modelData).title
                        MouseArea {
                            objectName:"railGrip:"+railButton.modelData;anchors.fill:parent
                            property point start;property bool moving:false
                            onPressed:m=>{start=Qt.point(m.x,m.y);moving=false}
                            onPositionChanged:m=>{
                                if(pressed && !moving && !PaintClient.drawing && Math.abs(m.x-start.x)+Math.abs(m.y-start.y)>8){
                                    moving=true;if(railRoot.peeks[railButton.modelData])railRoot.peeks[railButton.modelData].close()
                                    const id=Workspace.groupForPanel(railButton.modelData),panel=railButton.modelData;Qt.callLater(()=>Workspace.beginDrag(id,panel,false))
                                }
                            }
                            onClicked:if(!moving)railRoot.openPanel(railButton.modelData)
                        }
                    }
                }
            }
        }
    }

    DropArea {
        id:drop;anchors.fill:parent;enabled:!root.isCanvas && (root.isTools || root.layoutData.rail)
        keys:["application/x-drawverse-panel","application/x-drawverse-tool-strip"]
        onEntered:d=>{d.accepted=!Workspace.dockingSuppressed && !PaintClient.drawing && Math.min(d.x,width-d.x,d.y,height-d.y)<=24;root.locateDrop(d.x,d.y)}
        onPositionChanged:d=>{d.accepted=!Workspace.dockingSuppressed && !PaintClient.drawing && Math.min(d.x,width-d.x,d.y,height-d.y)<=24;root.locateDrop(d.x,d.y)}
        onDropped:d=> {
            if(Workspace.dockingSuppressed || PaintClient.drawing || Math.min(d.x,width-d.x,d.y,height-d.y)>24)return
            root.locateDrop(d.x,d.y)
            const ok=d.formats.indexOf("application/x-drawverse-tool-strip")>=0?Workspace.dockToolStrip(d.getDataAsString("application/x-drawverse-tool-strip"),root.layoutData.location,root.layoutData.id,root.dropMode):Workspace.dockPayload(d.getDataAsString("application/x-drawverse-panel"),root.layoutData.location,root.layoutData.id,root.dropMode)
            if(ok)d.acceptProposedAction()
        }
    }
    DockEdgePreview {
        objectName:"dockPreview:"+root.layoutData.id;anchors.fill:parent
        visible:(root.isTools || root.layoutData.rail) && (drop.containsDrag || Workspace.dragTarget===root.layoutData.id) && !Workspace.dockingSuppressed && !Workspace.dragPlacement.startsWith("column-")
        mode:Workspace.dragTarget===root.layoutData.id?Workspace.dragPlacement:root.dropMode
    }
}
