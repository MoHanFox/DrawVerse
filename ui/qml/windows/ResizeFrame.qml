import QtQuick
import QtQuick.Controls
import QtQuick.Window

Item {
    id: root
    required property var targetWindow
    parent: Overlay.overlay
    anchors.fill: parent
    z: 1000
    visible: targetWindow.visibility!==Window.Maximized && targetWindow.visibility!==Window.FullScreen
    Repeater {
        model: [Qt.LeftEdge,Qt.RightEdge,Qt.TopEdge,Qt.BottomEdge,Qt.LeftEdge|Qt.TopEdge,Qt.RightEdge|Qt.TopEdge,Qt.LeftEdge|Qt.BottomEdge,Qt.RightEdge|Qt.BottomEdge]
        MouseArea {
            required property int index
            required property int modelData
            readonly property bool corner: index>=4
            readonly property int edge: modelData
            x: edge & Qt.LeftEdge ? 0 : edge & Qt.RightEdge ? root.width-width : 6
            y: edge & Qt.TopEdge ? 0 : edge & Qt.BottomEdge ? root.height-height : 6
            width: corner ? 6 : edge & (Qt.LeftEdge|Qt.RightEdge) ? 3 : root.width-12
            height: corner ? 6 : edge & (Qt.TopEdge|Qt.BottomEdge) ? 3 : root.height-12
            cursorShape: corner ? ((edge===(Qt.LeftEdge|Qt.TopEdge) || edge===(Qt.RightEdge|Qt.BottomEdge)) ? Qt.SizeFDiagCursor : Qt.SizeBDiagCursor) : edge & (Qt.LeftEdge|Qt.RightEdge) ? Qt.SizeHorCursor : Qt.SizeVerCursor
            onPressed: root.targetWindow.startSystemResize(edge)
        }
    }
}
