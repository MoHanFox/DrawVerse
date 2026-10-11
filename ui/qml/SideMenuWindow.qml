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
    width: menu.implicitWidth
    height: menu.implicitHeight
    function alignMenu() {
        if(!visible || !menu.visible)return
        // A menu-bar open may finish positioning after the fallback reparented it.
        // The host owns screen placement; its item popup always starts at the host origin.
        menu.x=0;menu.y=0;menu.width=host.width;menu.height=host.height
        const popupItem=menu.contentItem.parent
        if(popupItem){popupItem.x=0;popupItem.y=0}
    }
    Connections {
        target:host.menu
        function onOpened(){host.alignMenu();Qt.callLater(host.alignMenu)}
    }
    function showMenu() {
        const owner=anchorItem.Window.window
        if(!owner)return
        font=owner.font;palette=owner.palette
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
    // Deliberately no onActiveChanged close. Every close path that watched activation fired while the
    // pointer was still inside the menu: the host is a Qt.Popup window that loses activation when a
    // submenu opens or the pointer moves, and the menu then dismissed itself and the focus fell back
    // to the first menu-bar entry. The popup's own closePolicy (Escape or a press outside) is the only
    // thing that decides now.
    onClosing: menu.close()
}
