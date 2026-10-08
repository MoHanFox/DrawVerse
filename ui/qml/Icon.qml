import QtQuick
import QtQuick.Shapes
import "."

Item {
    id: root
    property string name: "brush"
    property color color: Theme.text
    implicitWidth: 20; implicitHeight: 20
    // Code-native vectors use a fixed view box; no font glyphs or icon-font dependency.
    readonly property var paths: ({
        move: "M12 2V22M2 12H22M8 6L12 2L16 6M8 18L12 22L16 18M6 8L2 12L6 16M18 8L22 12L18 16",
        rectangleSelection: "M3 7V3H7M10 3H14M17 3H21V7M21 10V14M21 17V21H17M14 21H10M7 21H3V17M3 14V10",
        ellipseSelection: "M4 7Q5 4 8 3M11 2H13M16 3Q19 4 20 7M21 10V14M20 17Q19 20 16 21M13 22H11M8 21Q5 20 4 17M3 14V10",
        brush: "M9 14L17 3Q19 1 21 3Q23 5 21 7L12 16ZM9 14Q5 13 5 17Q5 20 2 21Q10 23 12 16",
        eraser: "M3 15L13 3Q14 2 15 3L21 9Q22 10 21 11L12 21H8ZM8 10L17 19M12 21H22",
        undo: "M8 4L3 9L8 14M3 9H14Q21 9 21 16V20",
        redo: "M16 4L21 9L16 14M21 9H10Q3 9 3 16V20",
        folder: "M3 6H10L12 8H21V20H3ZM3 6V4H10L12 6H21V8",
        trash: "M3 6H21M9 6V3H15V6M5 6L6 21H18L19 6M10 10V17M14 10V17",
        mask: "M3 5H21V19H3ZM16 12A4 4 0 1 1 8 12A4 4 0 1 1 16 12",
        plus: "M12 4V20M4 12H20",
        close: "M6 6L18 18M18 6L6 18",
        down: "M5 9L12 16L19 9",
        collapse: "M8 6L14 12L8 18M14 6L20 12L14 18",
        expand: "M16 6L10 12L16 18M10 6L4 12L10 18",
        float: "M13 3H21V11M21 3L11 13M9 5H3V21H19V15",
        dock: "M3 4H21V20H3ZM15 4V20M6 8H11M6 12H11M6 16H11",
        menu: "M4 6H20M4 12H20M4 18H20",
        layers: "M3 8L12 3L21 8L12 13ZM3 12L12 17L21 12M3 16L12 21L21 16",
        history: "M4 8A9 9 0 1 1 4 17M3 3V9H9M12 7V12L16 14",
        navigator: "M3 3H21V21H3ZM3 9H21M9 9V21M12 12H18V18H12Z",
        color: "M21 12A9 9 0 1 1 3 12A9 9 0 1 1 21 12ZM12 3V12L18 18M12 12L4 16",
        palette: "M12 3Q3 3 3 12Q3 21 12 21Q15 21 15 18Q15 16 18 16Q22 16 21 11Q20 3 12 3ZM8 7H8.1M14 7H14.1M18 11H18.1M6 12H6.1",
        eye: "M2 12Q12 1 22 12Q12 23 2 12ZM16 12A4 4 0 1 1 8 12A4 4 0 1 1 16 12",
        link: "M10 8L13 5Q18 1 21 5Q24 9 19 13L16 15M14 16L11 19Q6 23 3 19Q0 15 5 11L8 9M8 16L16 8",
        search: "M17 10A7 7 0 1 1 3 10A7 7 0 1 1 17 10ZM15 15L22 22",
        page: "M4 3H15L20 8V21H4ZM15 3V8H20",
        fit: "M3 9V3H9M15 3H21V9M21 15V21H15M9 21H3V15",
        settings: "M4 6H20M4 12H20M4 18H20M8 3V9M16 9V15M10 15V21"
        ,minimize: "M5 12H19"
        ,maximize: "M5 5H19V19H5Z"
        ,restore: "M8 4H20V16H16M4 8H16V20H4Z"
    })
    Item {
        width: 24; height: 24
        anchors.centerIn: parent
        scale: Math.min(root.width,root.height)/24
        Shape {
            anchors.fill: parent
            ShapePath {
                strokeColor: root.color; strokeWidth: 1.6
                capStyle: ShapePath.RoundCap; joinStyle: ShapePath.RoundJoin
                fillColor: "transparent"
                PathSvg { path: root.paths[root.name] || root.paths.page }
            }
        }
    }
}
