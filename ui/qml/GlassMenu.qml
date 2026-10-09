import QtQuick
import QtQuick.Controls
import QtQuick.Window
import DrawVerse 1.0
import "."

Menu {
    id: control
    padding: 6
    margins: 6
    // Qt 6.5 already uses item popups; Qt 6.8+ can also choose native/windows.
    Component.onCompleted: if ("popupType" in control) control.popupType=Popup.Item
    delegate: GlassMenuItem {}
    background: FrostedSurface {
        id: glass
        objectName: "menuGlassBackground"
        radius: Theme.menuRadius
        tint: Theme.menuGlass
    }
    onAboutToShow: glass.capture(control.parent ? control.parent.Window.window : null)
    onOpened: glass.relocate()
    onClosed: glass.clear()
}
