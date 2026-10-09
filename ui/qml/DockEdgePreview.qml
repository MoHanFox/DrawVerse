import QtQuick
import "."
Item {
    id:root
    property string mode:"left"
    readonly property bool vertical:mode==="left" || mode==="right"
    readonly property bool far:mode==="right" || mode==="bottom" || mode==="after"
    Rectangle {
        x:root.vertical && root.far?root.width-width:0
        y:!root.vertical && root.far?root.height-height:0
        width:root.vertical?10:root.width;height:root.vertical?root.height:root.mode==="merge"?24:10
        color:"#2023b5ee"
        Rectangle {
            anchors.fill:parent;anchors.margins:root.mode==="merge"?0:3
            color:"#4023b5ee"
        }
        Rectangle {
            x:root.vertical && root.far?parent.width-width:0
            y:!root.vertical && root.far?parent.height-height:0
            width:root.vertical?2:parent.width;height:root.vertical?parent.height:2
            color:Theme.accent
        }
    }
}
