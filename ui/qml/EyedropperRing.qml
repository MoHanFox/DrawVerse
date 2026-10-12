import QtQuick
import QtQuick.Shapes
Item {
    id: root
    required property color sampledColor
    required property color previousColor
    width:120;height:120
    Shape {
        anchors.fill:parent
        ShapePath {
            strokeColor:"#444444";strokeWidth:1;fillColor:"transparent"
            PathAngleArc {centerX:60;centerY:60;radiusX:58;radiusY:58;startAngle:0;sweepAngle:360}
        }
        ShapePath {
            strokeColor:"#808080";strokeWidth:12;fillColor:"transparent"
            PathAngleArc {centerX:60;centerY:60;radiusX:51;radiusY:51;startAngle:0;sweepAngle:360}
        }
        ShapePath {
            strokeColor:root.sampledColor;strokeWidth:18;capStyle:ShapePath.FlatCap;fillColor:"transparent"
            PathAngleArc {centerX:60;centerY:60;radiusX:36;radiusY:36;startAngle:180;sweepAngle:180}
        }
        ShapePath {
            strokeColor:root.previousColor;strokeWidth:18;capStyle:ShapePath.FlatCap;fillColor:"transparent"
            PathAngleArc {centerX:60;centerY:60;radiusX:36;radiusY:36;startAngle:0;sweepAngle:180}
        }
        ShapePath {
            strokeColor:"#444444";strokeWidth:1;fillColor:"transparent"
            PathAngleArc {centerX:60;centerY:60;radiusX:27;radiusY:27;startAngle:0;sweepAngle:360}
        }
    }
}
