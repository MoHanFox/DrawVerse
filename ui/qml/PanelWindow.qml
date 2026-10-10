import QtQuick
import QtQuick.Controls
import "."
ApplicationWindow {
    background:null
    font.family:Qt.platform.os==="windows"?"Microsoft YaHei UI":"sans-serif"
    font.pixelSize:10
    palette.window:Theme.surface;palette.windowText:Theme.text;palette.base:Theme.background;palette.text:Theme.text
    palette.button:Theme.raised;palette.buttonText:Theme.text;palette.highlight:Theme.selected;palette.highlightedText:Theme.accent
}
