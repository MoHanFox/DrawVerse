import QtQuick

// A tiny repeated texture, not a canvas-sized bitmap or thousands of delegates.
// The document/frame alpha is untouched; this is only the presentation backdrop.
Image {
    objectName: "transparencyGrid"
    source: "qrc:/qml/assets/transparency.png"
    sourceSize: Qt.size(24, 24)
    fillMode: Image.Tile
    horizontalAlignment: Image.AlignLeft
    verticalAlignment: Image.AlignTop
    smooth: false
}
