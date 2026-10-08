import QtQuick
import QtQuick.Controls
import "."
Slider {
    id: control
    property int axis: 0
    property real hue: 0
    property real saturation: 1
    property real brightness: 1
    function stopColor(t) { return Qt.hsva(axis===0 ? t : hue,axis===1 ? t : axis===0 ? 1 : saturation,axis===2 ? t : axis===0 ? 1 : brightness,1) }
    implicitHeight: 15; leftPadding: 0; rightPadding: 0
    background: Rectangle {
        x: control.leftPadding; y: control.topPadding+control.availableHeight/2-height/2
        width: control.availableWidth; height: 6
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop {position:0;color:control.stopColor(0)}
            GradientStop {position:1/6;color:control.stopColor(1/6)}
            GradientStop {position:2/6;color:control.stopColor(2/6)}
            GradientStop {position:.5;color:control.stopColor(.5)}
            GradientStop {position:4/6;color:control.stopColor(4/6)}
            GradientStop {position:5/6;color:control.stopColor(5/6)}
            GradientStop {position:1;color:control.stopColor(1)}
        }
    }
    handle: Item {
        x: control.leftPadding+control.visualPosition*(control.availableWidth-width)
        y: control.topPadding+control.availableHeight/2-height/2
        width: 6; height: 10
        Icon {anchors.fill:parent;name:"down";color:"white"}
    }
}
