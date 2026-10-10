import QtQuick
import QtQuick.Controls
import QtQuick.Window
import DrawVerse 1.0
import "."

Menu {
    id: control
    property var sideWindow: null
    property bool topLevel:false
    padding: 6
    margins: 6
    // A menu closes when the user clicks elsewhere or presses Escape, never because the pointer moved
    // or the host briefly lost activation. The defaults (CloseOnReleaseOutside plus QQuickPopup's own
    // close-on-deactivate) dismissed these menus while the pointer was still inside them.
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    // Qt 6.5 already uses item popups; Qt 6.8+ can also choose native/windows.
    Component.onCompleted: if ("popupType" in control) control.popupType=topLevel?Popup.Window:Popup.Item
    onAboutToShow: {
        if(topLevel && !("popupType" in control) && !sideWindow) {
            sideWindow=sideFactory.createObject(control,{menu:control,anchorItem:control.parent,originalParent:control.parent,belowAnchor:true})
            sideWindow.showMenu()
        }
    }
    onOpened:if(topLevel)Qt.callLater(()=>Workspace.promoteMenuWindow(control.contentItem))
    delegate: GlassMenuItem {}
    background: MenuSurface {
        objectName: "menuGlassBackground"
        radius: 0
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
