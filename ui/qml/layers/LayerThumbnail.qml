import QtQuick
import QtQuick.Controls
import "../canvas"
import "../theme"

Rectangle {
    id: root
    property var node
    property bool selected: false
    signal clicked(int modifiers)
    signal contextRequested(real x, real y)
    width: 34; height: 24
    color: Theme.surface
    border.color: Theme.line
    border.width: 1
    Rectangle {
        objectName: "thumbnailFocus:"+(root.node?root.node.id:0)
        anchors.fill: parent; anchors.margins: -2
        color: "transparent"; border.color: "#ffffff"; border.width: 1; visible: root.selected
    }
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
        source: root.node ? PaintClient.layerThumbnails[String(root.node.id)] || root.node.thumbnail || "" : ""
        fillMode: Image.PreserveAspectFit; asynchronous: true
    }
    Text { anchors.centerIn: parent; text: "×"; color: "#ea6b67"; font.pixelSize: 26; visible: root.node && root.node.mask && !root.node.visible }
    MouseArea {
        objectName: "layerThumbnailHit:" + (root.node ? root.node.id : 0)
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        property point origin
        property bool dragging:false
        onPressed:mouse=>{origin=Qt.point(mouse.x,mouse.y);dragging=false}
        onPositionChanged:mouse=>{if(pressed && (pressedButtons & Qt.LeftButton) && !dragging && root.node && !root.node.mask && Math.abs(mouse.x-origin.x)+Math.abs(mouse.y-origin.y)>10){dragging=true;PaintClient.beginLayerDrag(root.node.id)}}
        onClicked: mouse => {if(mouse.button===Qt.RightButton)root.contextRequested(mouse.x,mouse.y);else if(!dragging)root.clicked(mouse.modifiers)}
        ToolTip.delay:1000;ToolTip.visible: containsMouse
        ToolTip.text: root.node && root.node.mask ? "编辑蒙版 · Shift 点击启用/禁用" : "编辑图层内容"
        hoverEnabled: true
    }
}
