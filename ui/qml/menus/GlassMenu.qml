import QtQuick
import QtQuick.Controls
import QtQuick.Window
import DrawVerse 1.0
import "../theme"

Menu {
    id: control
    property var sideWindow: null
    property bool topLevel:false
    property var cascadeParent:null
    property bool windowPopupsAvailable: false
    property var inputWindow:null
    padding: 6
    margins: 6
    // A menu closes when the user clicks elsewhere or presses Escape, never because the pointer moved
    // or the host briefly lost activation. The defaults (CloseOnReleaseOutside plus QQuickPopup's own
    // close-on-deactivate) dismissed these menus while the pointer was still inside them.
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    // Qt 6.5 already uses item popups; Qt 6.8+ can also choose native/windows.
    Component.onCompleted:windowPopupsAvailable=("popupType" in control)
    onWindowPopupsAvailableChanged:if("popupType" in control)control.popupType=windowPopupsAvailable && (topLevel || cascadeParent)?Popup.Window:Popup.Item
    onAboutToShow: {
        if(cascadeParent && !windowPopupsAvailable && !sideWindow) {
            for(let i=0;i<cascadeParent.count;i++) {
                const row=cascadeParent.itemAt(i)
                if(row && row.subMenu===control) {
                    sideWindow=sideFactory.createObject(control,{menu:control,anchorItem:row,originalParent:control.parent})
                    sideWindow.showMenu();break
                }
            }
        } else if(topLevel && !windowPopupsAvailable && !sideWindow) {
            sideWindow=sideFactory.createObject(control,{menu:control,anchorItem:control.parent,originalParent:control.parent,belowAnchor:true})
            sideWindow.showMenu()
        }
    }
    function trackInputWindow() {
        if(!topLevel && !cascadeParent)return
        const window=control.contentItem.Window.window
        if(inputWindow && inputWindow!==window)Workspace.watchMenuWindow(inputWindow,false)
        inputWindow=window
        Workspace.watchMenuWindow(window,true)
    }
    onVisibleChanged: {
        if(visible)trackInputWindow()
        else if(inputWindow){Workspace.watchMenuWindow(inputWindow,false);inputWindow=null}
    }
    onOpened:if(topLevel || cascadeParent) {
        trackInputWindow()
        Qt.callLater(()=>Workspace.promoteMenuWindow(control.contentItem))
    }
    delegate: GlassMenuItem {}
    function openChild(child) {
        for(let i=0;i<count;i++) {
            const row=itemAt(i)
            if(row && row.subMenu && row.subMenu!==child)row.subMenu.close()
        }
        child.open()
    }
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
        if(inputWindow){Workspace.watchMenuWindow(inputWindow,false);inputWindow=null}
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
