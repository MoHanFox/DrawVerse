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
    property string selected: groupData.active
    property bool collapsed: groupData.collapsed || false
    onGroupDataChanged: { selected=groupData.active; collapsed=groupData.collapsed || false }
    readonly property int minimumPanelHeight: collapsed ? 8+groupData.panels.length*22 : Workspace.panelDefinition(selected).kind === "layers" ? 250 : Workspace.panelDefinition(selected).kind === "brush-settings" ? 260 : 120
    property string dropMode: "merge"
    property string beforePanel: ""
    color: Theme.surface; border.color: Theme.panelBar; clip: true
    implicitHeight: collapsed ? minimumPanelHeight : groupData.dockHeight || 320
    onHeightChanged: if(!collapsed) Workspace.updateDockHeight(groupData.id,Math.round(height))
    Connections {
        target: Workspace
        function onGroupStateChanged(group) {
            if(group!==root.groupData.id) return
            const data=Workspace.groupDefinition(group)
            root.selected=data.active; root.collapsed=data.collapsed
        }
    }
    function locateDrop(x,y) {
        beforePanel=""
        if(y<5) dropMode="before"
        else if(y>height-10) dropMode="after"
        else {
            dropMode="merge"
            if(y<28) {
                const p=tabRow.mapFromItem(root,x,y)
                for(let i=0;i<tabRepeater.count;i++) {
                    const tab=tabRepeater.itemAt(i)
                    if(p.x<tab.x+tab.width/2) { beforePanel=tab.modelData; break }
                }
            }
        }
    }
    ColumnLayout {
        anchors.fill: parent; spacing: 0
        Rectangle {
            Layout.fillWidth: true; implicitHeight: 8; color: Theme.strip
            MouseArea {
                objectName: "groupGrip:"+root.groupData.id
                anchors.fill: parent
                property point start
                onPressed: mouse => start=Qt.point(mouse.x,mouse.y)
                onPositionChanged: mouse => {
                    if(pressed && Math.abs(mouse.x-start.x)+Math.abs(mouse.y-start.y)>8) {
                        const id=root.groupData.id
                        Qt.callLater(() => Workspace.beginDrag(id,"",true))
                    }
                }
                onDoubleClicked: Workspace.setGroupCollapsed(root.groupData.id,!root.collapsed)
            }
            Row {
                anchors.right: parent.right; height: parent.height
                IconButton {
                    objectName: "groupCollapse:"+root.groupData.id
                    width: 16; height: 8; padding: 1; glyph: root.collapsed ? "expand" : "collapse"
                    tooltip: root.collapsed ? "展开面板组" : "折叠为标签"
                    onClicked: { if(root.flyout) root.dismissRequested(); else Workspace.setGroupCollapsed(root.groupData.id,!root.collapsed) }
                }
                IconButton {
                    width: 16; height: 8; padding: 1; glyph: "close"; tooltip: "关闭面板组"
                    onClicked: Qt.callLater(() => Workspace.hidePanel(root.groupData.id))
                }
            }
        }
        Rectangle {
            Layout.fillWidth: true; implicitHeight: 20; color:root.groupData.panels.length===1?Theme.surface:Theme.strip; visible: !root.collapsed
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
                            MouseArea {
                                anchors.fill: parent
                                property point start
                                onPressed: mouse => {
                                    start=Qt.point(mouse.x,mouse.y)
                                    root.selected=modelData; Workspace.setActive(root.groupData.id,modelData)
                                    if(root.collapsed) Workspace.setGroupCollapsed(root.groupData.id,false)
                                }
                                onPositionChanged: mouse => {
                                    if(pressed && Math.abs(mouse.x-start.x)+Math.abs(mouse.y-start.y)>8) {
                                        const id=root.groupData.id, panel=modelData
                                        Qt.callLater(() => Workspace.beginDrag(id,panel,false))
                                    }
                                }
                                onDoubleClicked: Workspace.setGroupCollapsed(root.groupData.id,!root.collapsed)
                            }
                        }
                    }
                }
            }
            IconButton {
                id: panelMenuButton; objectName: "panelMenu:"+root.groupData.id
                anchors.right: parent.right; width: 22; height: 20; padding: 6
                glyph: "menu"; tooltip: "面板菜单"; onClicked: {
                    if(root.groupData.location==="floating") panelMenu.popupBeside(root)
                    else panelMenu.popup()
                }
                GlassMenu {
                    id: panelMenu
                    objectName: "panelOperationsMenu:"+root.groupData.id
                    GlassMenuItem { text: "浮动当前面板"; onTriggered: Qt.callLater(() => Workspace.detachPanel(root.groupData.id,root.selected)) }
                    GlassMenuItem { text: root.groupData.location==="floating" ? "返回工作区" : "浮动整个面板组"; onTriggered: Qt.callLater(() => { if(root.groupData.location==="floating") Workspace.returnGroup(root.groupData.id); else Workspace.detachGroup(root.groupData.id) }) }
                    MenuSeparator {}
                    GlassMenuItem { text: root.collapsed ? "展开面板组" : "折叠为标签"; onTriggered: Workspace.setGroupCollapsed(root.groupData.id,!root.collapsed) }
                    GlassMenuItem { text: "关闭当前面板"; onTriggered: Qt.callLater(() => Workspace.hidePanel(root.groupData.id,root.selected)) }
                    GlassMenuItem { text: "关闭面板组"; onTriggered: Qt.callLater(() => Workspace.hidePanel(root.groupData.id)) }
                }
            }
        }
        Column {
            visible: root.collapsed; Layout.fillWidth: true
            Repeater {
                model: root.groupData.panels
                Rectangle {
                    required property string modelData
                    objectName: "collapsedPanel:"+modelData
                    width: parent.width; height: 22
                    color: root.selected===modelData ? Theme.selected : Theme.strip
                    Row { anchors.verticalCenter: parent.verticalCenter; x: 7; spacing: 7
                        Icon { name: Workspace.panelDefinition(modelData).kind==="brush-settings" ? "brush" : Workspace.panelDefinition(modelData).kind; width: 12; height: 12 }
                        Text { text: Workspace.panelDefinition(modelData).title; color: Theme.text; font.pixelSize: 9 }
                    }
                    MouseArea {
                        anchors.fill: parent; property point start
                        onPressed: mouse => start=Qt.point(mouse.x,mouse.y)
                        onPositionChanged: mouse => { if(pressed && Math.abs(mouse.x-start.x)+Math.abs(mouse.y-start.y)>8) {const id=root.groupData.id,panel=modelData;Qt.callLater(()=>Workspace.beginDrag(id,panel,false))} }
                        onClicked: {root.selected=modelData;Workspace.setActive(root.groupData.id,modelData);Workspace.setGroupCollapsed(root.groupData.id,false)}
                    }
                }
            }
        }
        PanelContent { Layout.fillWidth: true; Layout.fillHeight: true; visible: !root.collapsed; panelId: root.selected; canvasView: root.canvasView }
    }
    DropArea {
        id: drop; anchors.fill: parent; keys: ["application/x-drawverse-panel"]
        onEntered: d => { d.accepted=!Workspace.dockingSuppressed; root.locateDrop(d.x,d.y) }
        onPositionChanged: d => { d.accepted=!Workspace.dockingSuppressed; root.locateDrop(d.x,d.y) }
        onDropped: d => {
            root.locateDrop(d.x,d.y)
            if(!Workspace.dockingSuppressed && Workspace.dockPayload(d.getDataAsString("application/x-drawverse-panel"),root.groupData.location,root.groupData.id,root.dropMode,root.beforePanel)) d.acceptProposedAction()
        }
    }
    Rectangle {
        objectName: "dockPreview:"+root.groupData.id; visible: drop.containsDrag
        x: 1; width: root.width-2
        y: root.dropMode==="after" ? root.height*0.65 : root.dropMode==="merge" ? 8 : 0
        height: root.dropMode==="merge" ? root.height-9 : root.height*.35
        color: "#2423b5ee"; border.color: Theme.accent; border.width: 2
        Text { anchors.centerIn: parent; color: "#e6f6ff"; font.pixelSize: 11; text: root.dropMode==="before" ? "停靠到上方" : root.dropMode==="after" ? "停靠到下方" : root.beforePanel ? "插入标签" : "合并为标签" }
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
