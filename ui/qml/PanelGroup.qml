import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Rectangle {
    id: root
    objectName: "dockGroup:"+groupData.id
    required property var groupData
    property var canvasView
    property bool flyout: false
    signal dismissRequested()
    signal slideRequested(real globalY)
    property string selected: groupData.active
    onGroupDataChanged: selected=groupData.active
    readonly property int minimumPanelHeight: Workspace.panelDefinition(selected).kind === "layers" ? 250 : Workspace.panelDefinition(selected).kind === "brush-settings" ? 260 : 120
    property string dropMode: "merge"
    property string beforePanel: ""
    property bool reorderDrop:false
    property string tabDragPanel:""
    property string tabDropTarget:""
    function finishTabDrag(commit) {
        const first=tabDragPanel,second=tabDropTarget,id=groupData.id
        tabDragPanel="";tabDropTarget=""
        if(commit && first && second && first!==second)Workspace.swapPanelTabs(id,first,second)
    }
    color: Theme.surface; clip: true
    implicitHeight: groupData.dockHeight || 320
    onHeightChanged: if(!flyout) Workspace.updateDockHeight(groupData.id,Math.round(height))
    Connections {
        target: Workspace
        function onGroupStateChanged(group) {
            if(root.flyout || group!==root.groupData.id) return
            const data=Workspace.groupDefinition(group)
            root.selected=data.active
        }
    }
    function specialDrop(d) {
        reorderDrop=false
        if(d.formats.indexOf("application/x-drawverse-tool-strip")>=0)return true
        const data=d.getDataAsString("application/x-drawverse-panel")
        if(data.length>1024)return false
        try{const parsed=JSON.parse(data);reorderDrop=parsed.group===root.groupData.id && !parsed.whole;return parsed.group==="__canvas"}catch(e){return false}
    }
    function acceptsDrop(d){
        if(Workspace.dockingSuppressed || PaintClient.drawing)return false
        if(d.y>=28 && Math.min(d.x,width-d.x,d.y,height-d.y)>24)return false
        if(d.formats.indexOf("application/x-drawverse-tool-strip")>=0)return root.groupData.id!=="__toolstrip"
        try {const data=JSON.parse(d.getDataAsString("application/x-drawverse-panel"));return data.group!=="__canvas" && (data.group!==root.groupData.id || !data.whole && d.y<28 && root.groupData.panels.length>1)}catch(e){return false}
    }
    function locateDrop(x,y,special) {
        beforePanel=""
        if(!special && y>=8 && y<28 && (reorderDrop || x>24 && x<width-24)) {
            dropMode="merge"
            if(y<28) {
                const p=tabRow.mapFromItem(root,x,y)
                for(let i=0;i<tabRepeater.count;i++) {
                    const tab=tabRepeater.itemAt(i)
                    if(p.x<tab.x+tab.width/2) { beforePanel=tab.modelData; break }
                }
            }
        } else {
            const distance=Math.min(x,width-x,y,height-y)
            if(distance===x)dropMode="left"
            else if(distance===width-x)dropMode="right"
            else if(distance===y)dropMode="before"
            else dropMode="after"
        }
    }
    ColumnLayout {
        anchors.fill: parent; spacing: 0
        Rectangle {
            objectName:"columnHeader:"+root.groupData.id
            Layout.fillWidth: true; implicitHeight:8;visible:root.flyout || root.groupData.columnFirst;color:Theme.strip
            MouseArea {
                objectName: "groupGrip:"+root.groupData.id
                enabled: !PaintClient.drawing
                anchors.fill: parent
                property point start
                onPressed: mouse => start=Qt.point(mouse.x,mouse.y)
                onPositionChanged: mouse => {
                    if(pressed && !PaintClient.drawing && Math.abs(mouse.x-start.x)+Math.abs(mouse.y-start.y)>8) {
                        if(root.flyout)root.slideRequested(mapToGlobal(mouse.x,mouse.y).y)
                        else {const id=root.groupData.id;Qt.callLater(() => Workspace.beginDrag(id,"",true))}
                    }
                }
                onDoubleClicked: {root.dismissRequested();Workspace.setColumnCollapsed(root.groupData.id,!root.flyout)}
            }
            Row {
                anchors.right: parent.right; height: parent.height
                IconButton {
                    objectName:"dockCollapse:"+root.groupData.location
                    visible:!root.flyout
                    width:visible?16:0;height:8;padding:1;glyph:"collapse";tooltip:"折叠为图标"
                    onClicked:Workspace.setColumnCollapsed(root.groupData.id,true)
                }
                IconButton {
                    objectName:"flyoutRetract:"+root.selected
                    visible:root.flyout;width:visible?16:0;height:8;padding:1;glyph:"collapse";tooltip:"收回面板"
                    onClicked:root.dismissRequested()
                }

            }
        }
        Rectangle {
            Layout.fillWidth: true; implicitHeight: 20; color:root.groupData.panels.length===1?Theme.surface:Theme.strip; visible: true
            Flickable {
                id: tabStrip
                anchors.left: parent.left;anchors.leftMargin:root.groupData.panels.length>1?4:0; anchors.right: panelMenuButton.left; y:root.groupData.panels.length>1?2:0;height: parent.height-y
                contentWidth: tabRow.width; contentHeight: height
                clip: true; boundsBehavior: Flickable.StopAtBounds
                Row {
                    id: tabRow; height: tabStrip.height;spacing:root.groupData.panels.length>1?3:0
                    Repeater {
                        id: tabRepeater; model: root.groupData.panels
                        Rectangle {
                            required property string modelData
                            objectName: "panelTab:"+modelData
                            width: Math.max(38,label.implicitWidth+16); height: tabRow.height
                            color: root.selected===modelData ? Theme.surface : Theme.tabInactive
                            radius:root.groupData.panels.length>1?Theme.panelTabRadius:0
                            // Only the top corners round; the active tab joins its content.
                            Rectangle {anchors.left:parent.left;anchors.right:parent.right;anchors.bottom:parent.bottom;height:parent.radius;color:parent.color}
                            Text { id:label; anchors.centerIn: parent; text: Workspace.panelDefinition(modelData).title; color: root.selected===modelData ? "#eeeeee" : Theme.muted; font.pixelSize: 9 }
                            Rectangle {anchors.bottom:parent.bottom;width:parent.width;height:2;color:Theme.accent;visible:root.tabDragPanel.length>0 && root.tabDropTarget===modelData && root.tabDragPanel!==modelData}
                            MouseArea {
                                objectName:"panelTabGrip:"+modelData
                                anchors.fill: parent
                                preventStealing:true
                                property point start
                                property bool moving:false
                                onPressed: mouse => {
                                    start=Qt.point(mouse.x,mouse.y);moving=false;root.finishTabDrag(false)
                                    root.selected=modelData; Workspace.setActive(root.groupData.id,modelData)
                                }
                                onPositionChanged: mouse => {
                                    if(pressed && !PaintClient.drawing && Math.abs(mouse.x-start.x)+Math.abs(mouse.y-start.y)>8) {
                                        const at=mapToItem(tabStrip,mouse.x,mouse.y)
                                        if(at.x>=-8 && at.x<=tabStrip.width+8 && at.y>=-8 && at.y<=tabStrip.height+8) {
                                            moving=true;root.tabDragPanel=modelData;root.tabDropTarget=""
                                            const rowAt=mapToItem(tabRow,mouse.x,mouse.y)
                                            for(let i=0;i<tabRepeater.count;i++) {
                                                const tab=tabRepeater.itemAt(i)
                                                if(rowAt.x>=tab.x && rowAt.x<=tab.x+tab.width){root.tabDropTarget=tab.modelData;break}
                                            }
                                        } else {
                                            moving=true;root.finishTabDrag(false)
                                            const id=root.groupData.id, panel=modelData
                                            Qt.callLater(() => Workspace.beginDrag(id,panel,false))
                                        }
                                    }
                                }
                                onReleased:root.finishTabDrag(moving)
                                onCanceled:root.finishTabDrag(false)
                                onDoubleClicked: {root.dismissRequested();Workspace.setColumnCollapsed(root.groupData.id,!root.flyout)}
                            }
                        }
                    }
                }
            }
            IconButton {
                id: panelMenuButton; objectName: "panelMenu:"+root.groupData.id
                anchors.right: parent.right; width: 22; height: 20; padding: 6
                glyph: "menu"; tooltip: "面板菜单"; onClicked: {
                    if(root.flyout || root.groupData.location==="floating") panelMenu.popupBeside(root)
                    else panelMenu.popup()
                }
                GlassMenu {
                    id: panelMenu
                    objectName: "panelOperationsMenu:"+root.groupData.id
                    GlassMenuItem { text: "浮动当前面板"; onTriggered: Qt.callLater(() => Workspace.detachPanel(root.groupData.id,root.selected)) }
                    GlassMenuItem { text: root.groupData.location==="floating" ? "返回工作区" : "浮动整个面板组"; onTriggered: Qt.callLater(() => { if(root.groupData.location==="floating") Workspace.returnGroup(root.groupData.id); else Workspace.detachGroup(root.groupData.id) }) }
                    MenuSeparator {}
                    GlassMenuItem { text: "折叠为图标"; onTriggered: {root.dismissRequested();Workspace.setColumnCollapsed(root.groupData.id,true)} }
                    GlassMenuItem { text: "关闭当前面板"; onTriggered: Qt.callLater(() => Workspace.hidePanel(root.groupData.id,root.selected)) }
                    GlassMenuItem { text: "关闭面板组"; onTriggered: Qt.callLater(() => Workspace.hidePanel(root.groupData.id)) }
                }
            }
        }
        PanelContent { Layout.fillWidth: true; Layout.fillHeight: true; visible: true; panelId: root.selected; canvasView: root.canvasView }
    }
    DropArea {
        id: drop; anchors.fill: parent; keys: ["application/x-drawverse-panel","application/x-drawverse-tool-strip"]
        onEntered: d => { d.accepted=root.acceptsDrop(d); root.locateDrop(d.x,d.y,root.specialDrop(d)) }
        onPositionChanged: d => { d.accepted=root.acceptsDrop(d); root.locateDrop(d.x,d.y,root.specialDrop(d)) }
        onDropped: d => {
            root.locateDrop(d.x,d.y,root.specialDrop(d))
            if(!root.acceptsDrop(d))return
            const tools=d.formats.indexOf("application/x-drawverse-tool-strip")>=0
            const mode=tools && root.dropMode==="merge"?(d.x<root.width/2?"left":"right"):root.dropMode
            const ok=tools?Workspace.dockToolStrip(d.getDataAsString("application/x-drawverse-tool-strip"),root.groupData.location,root.groupData.id,mode):Workspace.dockPayload(d.getDataAsString("application/x-drawverse-panel"),root.groupData.location,root.groupData.id,root.dropMode,root.beforePanel)
            if(ok)d.acceptProposedAction()
        }
    }
    DockEdgePreview {
        objectName: "dockPreview:"+root.groupData.id; visible: drop.containsDrag || Workspace.dragTarget===root.groupData.id
        anchors.fill:parent
        mode:Workspace.dragTarget===root.groupData.id?Workspace.dragPlacement:root.dropMode
    }
    Rectangle {
        visible: drop.containsDrag && root.dropMode==="merge" && root.beforePanel.length>0
        x: {
            for(let i=0;i<tabRepeater.count;i++) { const tab=tabRepeater.itemAt(i); if(tab.modelData===root.beforePanel) return tab.mapToItem(root,0,0).x }
            return 0
        }
        y: 8; width: 2; height: 20; color: Theme.accent
    }
}
