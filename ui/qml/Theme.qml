pragma Singleton
import QtQuick
QtObject {
    readonly property color background: "#17191c"
    readonly property color surface: "#1c1e21"
    readonly property color raised: "#25262b"
    readonly property color line: "#33363b"
    readonly property color dockSeparator: "#2b2d31"
    readonly property color text: "#bababa"
    readonly property color muted: "#949494"
    readonly property color accent: "#23b5ee"
    readonly property color selected: "#3b3d42"
    readonly property color panelBar: "#0b0e10"
    readonly property color input: "#111214"
    readonly property color strip: "#202226"
    readonly property color tabInactive: "#111214"
    readonly property int windowRadius: 10
    readonly property int panelTabRadius: 6
    readonly property int documentTabRadius: 5
    readonly property color hover: "#303237"
    readonly property color disabled: "#5c6066"
    readonly property color menuGlass: surface
    readonly property color menuBarGlass: Qt.rgba(strip.r,strip.g,strip.b,.5)
    readonly property int menuRadius: 10
}
