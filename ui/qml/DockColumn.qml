import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Rectangle {
    id: root
    required property string side
    required property var groups
    property var canvasView
    property var peekData: ({})
    property string peekPanel: ""
    readonly property bool collapsed: side==="left" ? Workspace.leftCollapsed : Workspace.rightCollapsed
    readonly property bool empty: groups.length===0
    color: Theme.panelBar
    implicitWidth: empty ? 6 : collapsed ? 28 : side==="left" ? Workspace.leftDockWidth : Workspace.rightDockWidth
    function setCollapsed(value) { if(side==="left") Workspace.leftCollapsed=value; else Workspace.rightCollapsed=value }
    function openPeek(group,panel) {
        if(peek.opened && peekPanel===panel) { peek.close(); return }
        peekPanel=panel
        Workspace.setActive(group,panel); Workspace.setGroupCollapsed(group,false)
        peekData=Workspace.groupDefinition(group)
        peek.open()
    }
    Connections { target:Workspace; function onGroupsChanged() { peek.close() } }
    onCollapsedChanged: if(!collapsed) peek.close()
    onWidthChanged: {
        if(empty || collapsed || width<100) return
        if(side==="left") Workspace.leftDockWidth=Math.round(width)
        else Workspace.rightDockWidth=Math.round(width)
    }
    IconButton {
        objectName: "dockCollapse:"+root.side
        visible: !root.empty
        anchors.top: parent.top; anchors.right: parent.right
        width: root.collapsed ? 28 : 18; height: 10; padding: 2
        glyph: root.collapsed ? "expand" : "collapse"
        tooltip: root.collapsed ? "展开面板列" : "折叠为图标"
        onClicked: root.setCollapsed(!root.collapsed)
    }
    SplitView {
        visible: !root.collapsed && !root.empty
        anchors.fill: parent; anchors.topMargin: 10
        orientation: Qt.Vertical
        handle: Rectangle { implicitHeight: 4; color: SplitHandle.hovered || SplitHandle.pressed ? Theme.accent : Theme.panelBar }
        Repeater {
            model: root.groups
            PanelGroup {
                required property var modelData
                groupData: modelData; canvasView: root.canvasView
                SplitView.preferredHeight: collapsed ? minimumPanelHeight : groupData.dockHeight
                SplitView.minimumHeight: minimumPanelHeight
                SplitView.maximumHeight: collapsed ? minimumPanelHeight : 2000
            }
        }
    }
    Flickable {
        visible: root.collapsed
        anchors.fill: parent; anchors.topMargin: 12
        contentHeight: rail.height; contentWidth: width
        clip: true; boundsBehavior: Flickable.StopAtBounds
        Column {
            id: rail; width: parent.width
            Repeater {
                model: root.groups
                Column {
                    id: railGroup
                    required property var modelData
                    width: rail.width
                    Rectangle { width: parent.width; height: 1; color: Theme.line }
                    Repeater {
                        model: modelData.panels
                        IconButton {
                            required property string modelData
                            objectName: "railPanel:"+modelData
                            width: 28; height: 28; padding: 7
                            glyph: Workspace.panelDefinition(modelData).kind
                            tooltip: Workspace.panelDefinition(modelData).title
                            onClicked: root.openPeek(railGroup.modelData.id,modelData)
                        }
                    }
                }
            }
        }
    }
    Popup {
        id: peek
        objectName: "dockPeek:"+root.side
        x: root.side==="left" ? root.width : -width
        y: 18; width: root.side==="left" ? Workspace.leftDockWidth : Workspace.rightDockWidth
        height: Math.max(280,Math.min(root.height-18,620))
        padding: 1
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        background: Rectangle { color: Theme.surface; border.color: Theme.line }
        contentItem: Loader {
            active: peek.visible && root.peekData.id!==undefined
            sourceComponent: PanelGroup {
                objectName: "peekGroup:"+root.side
                groupData: root.peekData; canvasView: root.canvasView; flyout: true
                onDismissRequested: peek.close()
            }
        }
    }
    DropArea {
        id: edge
        anchors.top: parent.top; anchors.bottom: parent.bottom
        x: root.side==="left" ? 0 : root.width-width
        width: root.empty || root.collapsed ? root.width : 6
        keys: ["application/x-drawverse-panel"]
        onEntered: d => d.accepted=!Workspace.dockingSuppressed
        onPositionChanged: d => d.accepted=!Workspace.dockingSuppressed
        onDropped: d => { if(!Workspace.dockingSuppressed && Workspace.dockPayload(d.getDataAsString("application/x-drawverse-panel"),root.side)) d.acceptProposedAction() }
    }
    Rectangle { visible: edge.containsDrag; anchors.fill: parent; color: "#2423b5ee"; border.color: Theme.accent; border.width: 2 }
}
