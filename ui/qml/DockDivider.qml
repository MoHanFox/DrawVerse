import QtQuick
import "."
Rectangle {
    id:root
    required property var layoutData
    required property var workspace
    objectName:"dockDivider:"+layoutData.id
    x:layoutData.rect.x;y:layoutData.rect.y;width:layoutData.rect.width;height:layoutData.rect.height
    z:2;color:mouse.containsMouse || mouse.pressed?Theme.accent:Theme.dockSeparator
    function updateLayout(data){layoutData=data}
    MouseArea {
        id:mouse;anchors.fill:parent;anchors.margins:-2;hoverEnabled:true
        cursorShape:root.layoutData.axis==="horizontal"?Qt.SplitHCursor:Qt.SplitVCursor
        onPositionChanged:m=> {
            if(!pressed || PaintClient.drawing)return
            const at=mapToItem(root.workspace,m.x,m.y),area=root.layoutData.area
            Workspace.setSplitRatio(root.workspace.hostId,root.layoutData.id,root.layoutData.axis==="horizontal"?(at.x-area.x)/area.width:(at.y-area.y)/area.height)
        }
    }
}
