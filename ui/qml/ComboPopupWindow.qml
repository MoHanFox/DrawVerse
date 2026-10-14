import QtQuick
import QtQuick.Window
import QtQuick.Controls
import "."
ApplicationWindow {
    id:host
    objectName:"priorityComboWindow"
    required property var menu
    required property Item anchorItem
    required property Item originalParent
    transientParent:anchorItem.Window.window
    flags:Qt.Popup | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    color:Theme.surface;background:null
    function alignPopup(){
        const owner=anchorItem.Window.window;if(!owner)return
        const area=Workspace.availableScreenGeometry(owner);const below=anchorItem.mapToGlobal(Qt.point(0,anchorItem.height))
        width=Math.min(menu.width,area.width-12);height=Math.min(menu.implicitHeight,area.height-12)
        x=Math.max(area.x+6,Math.min(below.x,area.x+area.width-width-6));y=Math.max(area.y+6,Math.min(below.y,area.y+area.height-height-6))
    }
    function showPopup(){
        const owner=anchorItem.Window.window
        if(!owner)return
        font=owner.font;palette=owner.palette
        const area=Workspace.availableScreenGeometry(owner)
        const below=anchorItem.mapToGlobal(Qt.point(0,anchorItem.height))
        width=Math.min(menu.width,area.width-12)
        height=Math.min(menu.implicitHeight,area.height-12)
        x=Math.max(area.x+6,Math.min(below.x,area.x+area.width-width-6))
        y=Math.max(area.y+6,Math.min(below.y,area.y+area.height-height-6))
        menu.parent=contentItem;menu.x=0;menu.y=0
        visible=true;requestActivate()
    }
    Shortcut {sequence:"Escape";context:Qt.WindowShortcut;enabled:host.visible;onActivated:host.menu.close()}
    Connections {
        target:host.menu
        function onOpened(){Qt.callLater(host.alignPopup)}
        function onImplicitHeightChanged(){if(host.visible)Qt.callLater(host.alignPopup)}
        function onClosed(){host.visible=false;host.menu.parent=host.originalParent;host.menu.x=0;host.menu.y=host.anchorItem.height}
    }
    onVisibleChanged:{Workspace.watchMenuWindow(host,visible);if(!visible && menu.visible)menu.close()}
    onClosing:menu.close()
}
