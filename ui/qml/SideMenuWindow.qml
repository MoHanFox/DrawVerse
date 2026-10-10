import QtQuick
import QtQuick.Window
import QtQuick.Controls
import "."

ApplicationWindow {
    id: host
    objectName: "sideMenuWindow"
    required property var menu
    required property Item anchorItem
    required property Item originalParent
    property bool belowAnchor:false
    transientParent: anchorItem.Window.window
    flags: Qt.Popup | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    color: Theme.surface
    background: null
    font: anchorItem.Window.window.font
    palette: anchorItem.Window.window.palette
    width: menu.implicitWidth
    height: menu.implicitHeight
    function showMenu() {
        const owner=anchorItem.Window.window
        const area=Workspace.availableScreenGeometry(owner)
        height=Math.min(menu.implicitHeight,area.height-12)
        const right=anchorItem.mapToGlobal(Qt.point(anchorItem.width+4,8))
        const left=anchorItem.mapToGlobal(Qt.point(-4,8))
        const rightSpace=Math.max(1,area.x+area.width-6-right.x)
        const leftSpace=Math.max(1,left.x-area.x-6)
        const useRight=rightSpace>=width || rightSpace>=leftSpace
        width=Math.min(menu.implicitWidth,useRight?rightSpace:leftSpace)
        x=useRight?right.x:left.x-width
        y=Math.max(area.y+6,Math.min(right.y,area.y+area.height-height-6))
        if(belowAnchor) {
            const below=anchorItem.mapToGlobal(Qt.point(0,anchorItem.height))
            width=Math.min(menu.implicitWidth,area.width-12)
            x=Math.max(area.x+6,Math.min(below.x,area.x+area.width-width-6))
            y=Math.max(area.y+6,Math.min(below.y,area.y+area.height-height-6))
        }
        menu.parent=contentItem
        menu.margins=0
        menu.width=width
        menu.height=height
        visible=true
        requestActivate()
        menu.popup(contentItem,Qt.point(0,0))
    }
    onVisibleChanged: {
        Workspace.watchMenuWindow(host,visible)
        if (!visible && menu.visible) menu.close()
    }
    // A menu must only close when the user really left it. The host briefly loses activation while a
    // submenu opens or while the pointer moves inside the first row, and closing on that transient
    // state dismissed the menu under the cursor. Only a window outside the menu's own family counts.
    onActiveChanged: {
        if (active || !menu.opened) return
        const owner = anchorItem ? anchorItem.Window.window : null
        const activeWindow = Window.activeWindow
        if (activeWindow === host || activeWindow === owner) return
        // A submenu or flyout belonging to this menu keeps it open.
        if (activeWindow && activeWindow.transientParent && (activeWindow.transientParent === host || activeWindow.transientParent === owner)) return
        menu.close()
    }
    onClosing: menu.close()
}
