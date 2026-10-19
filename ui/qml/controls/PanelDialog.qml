import QtQuick
import QtQuick.Controls
import QtQuick.Window

Dialog {
    id: root
    required property Item anchorItem
    parent: anchorItem
    popupType: Popup.Window
    modal: true
    standardButtons: Dialog.Ok | Dialog.Cancel
    function alignToPanel() {
        const owner=anchorItem.Window.window
        if(!owner)return
        const area=Workspace.availableScreenGeometry(owner)
        const right=anchorItem.mapToGlobal(Qt.point(anchorItem.width+6,0))
        const left=anchorItem.mapToGlobal(Qt.point(-6,0))
        const targetX=right.x+width<=area.x+area.width?right.x:left.x-width
        const globalX=Math.max(area.x,Math.min(targetX,area.x+area.width-width))
        const globalY=Math.max(area.y,Math.min(right.y,area.y+area.height-height))
        const local=anchorItem.mapFromGlobal(Qt.point(globalX,globalY))
        x=local.x;y=local.y
    }
    onAboutToShow: alignToPanel()
}
