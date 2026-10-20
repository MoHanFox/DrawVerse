import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Shapes
import "../controls"
import "../theme"

RowLayout {
    id: root
    property real angle: 0
    property real roundness: 1
    property string namePrefix: "settingsBrush"
    signal angleEdited(real value)
    signal roundnessEdited(real value)
    Layout.fillWidth: true
    spacing: 8

    Item {
        id: dial
        objectName: root.namePrefix+"TipDial"
        Layout.preferredWidth: 96
        Layout.preferredHeight: 96
        Layout.minimumWidth: 96
        readonly property real center: width/2
        readonly property real extent: width/2-8
        readonly property real radians: root.angle*Math.PI/180
        readonly property real shortRadius: extent*Math.max(.01,Math.min(1,root.roundness))
        readonly property real handleX: -Math.sin(radians)*shortRadius
        readonly property real handleY: Math.cos(radians)*shortRadius

        Rectangle {
            anchors.centerIn: parent
            width: 2*dial.extent+12; height: width; radius: width/2
            color: Theme.raised; border.color: Theme.categorySeparator
        }
        Rectangle {anchors.centerIn: parent; width: 2*dial.extent; height: 1; color: Theme.categorySeparator}
        Rectangle {anchors.centerIn: parent; height: 2*dial.extent; width: 1; color: Theme.categorySeparator}
        Shape {
            anchors.centerIn: parent
            width: 2*dial.extent; height: width
            rotation: root.angle
            antialiasing: true
            ShapePath {
                fillColor: "transparent"; strokeColor: Theme.text; strokeWidth: 1.5
                PathAngleArc {
                    centerX: dial.extent; centerY: dial.extent
                    radiusX: dial.extent; radiusY: dial.shortRadius
                    startAngle: 0; sweepAngle: 360
                }
            }
        }
        Item {
            x: dial.center; y: dial.center; rotation: root.angle
            Rectangle {width: dial.extent+3; height: 1.5; y: -height/2; color: Theme.accent}
            Shape {
                antialiasing: true
                ShapePath {
                    strokeWidth: 0; fillColor: Theme.accent
                    startX: dial.extent+5; startY: 0
                    PathLine {x: dial.extent-3; y: -4}
                    PathLine {x: dial.extent-3; y: 4}
                    PathLine {x: dial.extent+5; y: 0}
                }
            }
        }
        Repeater {
            model: [-1,1]
            Rectangle {
                required property int modelData
                x: dial.center+modelData*dial.handleX-width/2
                y: dial.center+modelData*dial.handleY-height/2
                width: 10; height: 10; radius: 5
                color: Theme.text; border.width: 1.5; border.color: Theme.accent
                antialiasing: true
            }
        }
        MouseArea {
            anchors.fill: parent
            preventStealing: true
            cursorShape: pressed?Qt.ClosedHandCursor:Qt.OpenHandCursor
            property bool shaping: false
            property real gestureAngle: 0
            function update(mouse) {
                const dx=mouse.x-dial.center, dy=mouse.y-dial.center
                if (shaping) {
                    const projected=-Math.sin(gestureAngle)*dx+Math.cos(gestureAngle)*dy
                    root.roundnessEdited(Math.max(1,Math.min(100,Math.round(Math.abs(projected)/dial.extent*100)))/100)
                } else if (dx*dx+dy*dy>=16) {
                    root.angleEdited(Math.round(Math.atan2(dy,dx)*180/Math.PI))
                }
            }
            onPressed: mouse=>{
                const dx=mouse.x-dial.center, dy=mouse.y-dial.center
                const d1=Math.hypot(dx-dial.handleX,dy-dial.handleY)
                const d2=Math.hypot(dx+dial.handleX,dy+dial.handleY)
                shaping=Math.min(d1,d2)<=9
                gestureAngle=dial.radians
                if (!shaping) update(mouse)
            }
            onPositionChanged: mouse=>{if (pressed) update(mouse)}
            onReleased: mouse=>update(mouse)
            ToolTip.delay: 1000
            hoverEnabled: true
            ToolTip.visible: containsMouse && !pressed
            ToolTip.text: "拖动方向盘调整角度，拖动两个圆点调整圆度"
        }
    }
    ColumnLayout {
        Layout.fillWidth: true
        spacing: 10
        RowLayout {
            Layout.fillWidth: true; spacing: 3
            Label {text: "角度"; color: Theme.text; font.pixelSize: 10; Layout.fillWidth: true}
            CompactSpinBox {
                objectName: root.namePrefix+"angleInput"
                Layout.preferredWidth: 76; implicitHeight: 24
                editable: true; from: -180; to: 180
                value: Math.round(root.angle)
                onValueModified: root.angleEdited(value)
                tooltip: "笔尖方向；0°朝右，正角度顺时针旋转。"
            }
            Label {text: "°"; color: Theme.muted; font.pixelSize: 9; Layout.preferredWidth: 9}
        }
        RowLayout {
            Layout.fillWidth: true; spacing: 3
            Label {text: "圆度"; color: Theme.text; font.pixelSize: 10; Layout.fillWidth: true}
            CompactSpinBox {
                objectName: root.namePrefix+"roundnessInput"
                Layout.preferredWidth: 76; implicitHeight: 24
                editable: true; from: 1; to: 100
                value: Math.round(root.roundness*100)
                onValueModified: root.roundnessEdited(value/100)
                tooltip: "笔尖短轴与长轴的比例。"
            }
            Label {text: "%"; color: Theme.muted; font.pixelSize: 9; Layout.preferredWidth: 9}
        }
    }
}
