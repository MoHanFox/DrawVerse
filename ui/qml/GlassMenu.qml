import QtQuick
import QtQuick.Controls
import QtQuick.Window
import DrawVerse 1.0
import "."

Menu {
    id: control
    property var sideWindow: null
    padding: 6
    margins: 6
    // Qt 6.5 already uses item popups; Qt 6.8+ can also choose native/windows.
    Component.onCompleted: if ("popupType" in control) control.popupType=Popup.Item
    delegate: GlassMenuItem {}
    background: MenuSurface {
        objectName: "menuGlassBackground"
        radius: Theme.menuRadius
        tint: Theme.menuGlass
    }
    function popupBeside(panel) {
        if (sideWindow) { close(); return }
        sideWindow=sideFactory.createObject(control,{menu:control,anchorItem:panel,originalParent:control.parent})
        sideWindow.showMenu()
    }
    Component { id:sideFactory; SideMenuWindow {} }
    onClosed: {
        if (!sideWindow) return
        const host=sideWindow
        sideWindow=null
        parent=host.originalParent
        margins=6
        width=undefined
        height=undefined
        host.visible=false
        host.destroy()
    }
}
