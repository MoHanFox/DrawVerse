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
    function locateDrop(x,y){
        if(y<Math.min(30,height*.18))dropMode="top"
        else if(y>height-Math.min(30,height*.18))dropMode="bottom"
        else dropMode=x<width/2?"left":"right"
    }
    Item {id:canvasHost;anchors.fill:parent;visible:root.isCanvas}
    Component.onCompleted:{if(isCanvas)workspace.canvasPane.attach(canvasHost);else Workspace.registerTarget(layoutData.id,root)}
    Loader {
        anchors.fill:parent;active:!root.isCanvas
        sourceComponent:root.isTools?tools:root.layoutData.rail?rail:panel
    }
    Component {id:tools;ToolStrip {canvasView:root.workspace.canvasView}}
    Component {
        id:panel
        PanelGroup {groupData:root.layoutData;canvasView:root.workspace.canvasView}
    }
    Component {
        id:rail
        Rectangle {
            id:railRoot;color:Theme.panelBar
            property var peek:null
            function openPanel(panel){
                const same=peek && peek.visible && peek.selectedPanel===panel
                Workspace.setActive(root.layoutData.id,panel)
                if(same){peek.close();return}
                if(!peek)peek=flyoutFactory.createObject(railRoot,{ownerTile:root,canvasView:root.workspace.canvasView})
                peek.showPanel(panel)
            }
            Component.onDestruction:if(peek){peek.destroy();peek=null}
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
                    model:root.layoutData.panels
                    IconButton {
                        id:railButton;required property string modelData
                        objectName:"railPanel:"+modelData;width:28;height:28;padding:7
                        checked:railRoot.peek && railRoot.peek.visible && railRoot.peek.selectedPanel===modelData
                        glyph:Workspace.panelDefinition(modelData).kind==="brush-settings"?"settings":Workspace.panelDefinition(modelData).kind;tooltip:Workspace.panelDefinition(modelData).title
                        MouseArea {
                            objectName:"railGrip:"+railButton.modelData;anchors.fill:parent
                            property point start;property bool moving:false
                            onPressed:m=>{start=Qt.point(m.x,m.y);moving=false}
                            onPositionChanged:m=>{
                                if(pressed && !moving && !PaintClient.drawing && Math.abs(m.x-start.x)+Math.abs(m.y-start.y)>8){
                                    moving=true;if(railRoot.peek)railRoot.peek.close()
                                    const id=root.layoutData.id,panel=railButton.modelData;Qt.callLater(()=>Workspace.beginDrag(id,panel,false))
                                }
                            }
                            onClicked:if(!moving)railRoot.openPanel(railButton.modelData)
                        }
                    }
                }
            }
            Component {id:flyoutFactory;PanelFlyout {id:flyout;onClosing: {Workspace.watchPanelFlyout(flyout,ownerTile,false);if(railRoot.peek===flyout)railRoot.peek=null;Qt.callLater(()=>flyout.destroy())}}}
        }
    }

    DropArea {
        id:drop;anchors.fill:parent;enabled:!root.isCanvas && (root.isTools || root.layoutData.rail)
        keys:["application/x-drawverse-panel","application/x-drawverse-tool-strip"]
        onEntered:d=>{d.accepted=!Workspace.dockingSuppressed && !PaintClient.drawing;root.locateDrop(d.x,d.y)}
        onPositionChanged:d=>{d.accepted=!Workspace.dockingSuppressed && !PaintClient.drawing;root.locateDrop(d.x,d.y)}
        onDropped:d=> {
            if(Workspace.dockingSuppressed || PaintClient.drawing)return
            root.locateDrop(d.x,d.y)
            const ok=d.formats.indexOf("application/x-drawverse-tool-strip")>=0?Workspace.dockToolStrip(d.getDataAsString("application/x-drawverse-tool-strip"),root.layoutData.location,root.layoutData.id,root.dropMode):Workspace.dockPayload(d.getDataAsString("application/x-drawverse-panel"),root.layoutData.location,root.layoutData.id,root.dropMode)
            if(ok)d.acceptProposedAction()
        }
    }
    Rectangle {
        objectName:"dockPreview:"+root.layoutData.id;visible:(drop.containsDrag || Workspace.dragTarget===root.layoutData.id) && !Workspace.dockingSuppressed
        property string mode:Workspace.dragTarget===root.layoutData.id?Workspace.dragPlacement:root.dropMode
        x:mode==="right"?root.width*.65:0;y:mode==="bottom" || mode==="after"?root.height*.65:0
        width:mode==="left" || mode==="right"?root.width*.35:root.width
        height:mode==="top" || mode==="bottom" || mode==="before" || mode==="after"?root.height*.35:root.height
        color:"#2423b5ee";border.color:Theme.accent;border.width:2
    }
}
