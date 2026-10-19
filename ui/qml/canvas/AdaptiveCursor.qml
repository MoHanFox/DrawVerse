import QtQuick
import QtQuick.Window

// Both sources share canvas-local coordinates. Only the clipped cursor bounds
// enter these GPU textures; the shader output is never part of its own source.
Item {
    id: root
    required property Item backgroundItem
    required property Item outlineItem
    required property rect captureRect
    property bool active: false
    readonly property real pixelRatio: Screen.devicePixelRatio
    readonly property real textureScale: Math.min(pixelRatio, 2048 / Math.max(1, captureRect.width), 2048 / Math.max(1, captureRect.height))
    readonly property size captureSize: Qt.size(Math.max(1, Math.min(2048, Math.ceil(captureRect.width * textureScale))), Math.max(1, Math.min(2048, Math.ceil(captureRect.height * textureScale))))
    x: captureRect.x; y: captureRect.y
    width: captureRect.width; height: captureRect.height
    visible: active && width > 0 && height > 0
    ShaderEffectSource {
        id: backdrop
        sourceItem: root.visible ? root.backgroundItem : null
        sourceRect: root.captureRect
        textureSize: root.captureSize
        live: root.visible
        visible: false
        smooth: false
    }
    ShaderEffectSource {
        id: outline
        sourceItem: root.active ? root.outlineItem : null
        sourceRect: root.captureRect
        textureSize: root.captureSize
        hideSource: root.active
        live: root.visible
        visible: false
        smooth: false
    }
    ShaderEffect {
        objectName: "adaptiveCursorShader"
        anchors.fill: parent
        property var background: backdrop
        property var mask: outline
        property vector2d texel: Qt.vector2d(1 / root.captureSize.width, 1 / root.captureSize.height)
        fragmentShader: "qrc:/shaders/adaptive-cursor.frag.qsb"
    }
}
