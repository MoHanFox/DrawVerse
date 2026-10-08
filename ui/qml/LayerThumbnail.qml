import QtQuick
import QtQuick.Controls
import "."

Rectangle {
    id: root
    property var node
    property bool selected: false
    signal clicked(int modifiers)
    width: 34; height: 24
    color: Theme.surface
    border.color: selected ? "#ffffff" : Theme.muted
    border.width: selected ? 2 : 1
    clip: true
    // Checkerboard is restricted to the actual fitted image. An opaque image
    // must never show transparency in its border or aspect-ratio letterbox.
    Item {
        id: imageArea
        objectName: "thumbnailTransparency:" + (root.node ? root.node.id : 0)
        anchors.centerIn: parent
        width: preview.paintedWidth; height: preview.paintedHeight
        TransparencyGrid { anchors.fill: parent }
    }
    Image {
        id: preview
        objectName: "layerThumbnail:" + (root.node ? root.node.id : 0)
        anchors.fill: parent; anchors.margins: 2
        source: root.node ? root.node.thumbnail || "" : ""
        fillMode: Image.PreserveAspectFit; asynchronous: true
    }
    Text { anchors.centerIn: parent; text: "×"; color: "#ea6b67"; font.pixelSize: 26; visible: root.node && root.node.mask && !root.node.visible }
    MouseArea {
        objectName: "layerThumbnailHit:" + (root.node ? root.node.id : 0)
        anchors.fill: parent
        property point origin
        onPressed:mouse=>{origin=Qt.point(mouse.x,mouse.y)}
        onPositionChanged:mouse=>{if(pressed && root.node && !root.node.mask && Math.abs(mouse.x-origin.x)+Math.abs(mouse.y-origin.y)>10)PaintClient.beginLayerDrag(root.node.id)}
        onClicked: mouse => root.clicked(mouse.modifiers)
        ToolTip.visible: containsMouse
        ToolTip.text: root.node && root.node.mask ? "编辑蒙版 · Shift 点击启用/禁用" : "编辑图层内容"
        hoverEnabled: true
    }
}
