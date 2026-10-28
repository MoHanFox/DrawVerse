import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../theme"
ColumnLayout {
    id:root
    property string caption: ""
    property string name: ""
    property string unit: "%"
    property real value: 0
    property real from: -100
    property real to: 100
    property int spectrum: 0
    property color tint: "#ff3030"
    signal edited(real number)
    spacing:6
    RowLayout {Layout.fillWidth:true
        Label {text:root.caption;color:Theme.text;font.pixelSize:13}
        Item {Layout.fillWidth:true}
        AdjustmentValue {objectName:root.name+"Value";from:root.from;to:root.to;value:root.value;unit:root.unit;onEdited:number=>root.edited(number)}
    }
    Item {
        Layout.fillWidth:true;Layout.preferredHeight:22
        Rectangle {id:track;x:7;y:8;width:parent.width-14;height:6
            gradient:Gradient {orientation:Gradient.Horizontal
                GradientStop {position:0;color:root.spectrum===0?"#ff6666":root.spectrum===1?"#858585":"black"}
                GradientStop {position:1/6;color:root.spectrum===0?"#ffff66":root.spectrum===1?Qt.tint("#858585",Qt.rgba(root.tint.r,root.tint.g,root.tint.b,.16)):Qt.darker(root.tint,4)}
                GradientStop {position:2/6;color:root.spectrum===0?"#66ff88":root.spectrum===1?Qt.tint("#858585",Qt.rgba(root.tint.r,root.tint.g,root.tint.b,.33)):Qt.darker(root.tint,2)}
                GradientStop {position:.5;color:root.spectrum===0?"#66ffff":root.spectrum===1?Qt.tint("#858585",Qt.rgba(root.tint.r,root.tint.g,root.tint.b,.5)):root.tint}
                GradientStop {position:4/6;color:root.spectrum===0?"#7777ff":root.spectrum===1?Qt.tint("#858585",Qt.rgba(root.tint.r,root.tint.g,root.tint.b,.66)):Qt.lighter(root.tint,1.3)}
                GradientStop {position:5/6;color:root.spectrum===0?"#ff66ff":root.spectrum===1?Qt.tint("#858585",Qt.rgba(root.tint.r,root.tint.g,root.tint.b,.83)):Qt.lighter(root.tint,1.7)}
                GradientStop {position:1;color:root.spectrum===0?"#ff6688":root.spectrum===1?root.tint:"white"}
            }
        }
        Rectangle {x:track.x+(Math.max(root.from,Math.min(root.to,root.value))-root.from)/(root.to-root.from)*track.width-width/2;y:3;width:16;height:16;radius:8;color:"#e8ebeb";border.color:"#111111"}
        MouseArea {objectName:root.name+"Drag";anchors.fill:parent;preventStealing:true
            function change(mouse){root.edited(Math.round(root.from+Math.max(0,Math.min(1,(mouse.x-track.x)/track.width))*(root.to-root.from)))}
            onPressed:mouse=>{root.forceActiveFocus();change(mouse)}
            onPositionChanged:mouse=>{if(pressed)change(mouse)}
        }
    }
}
