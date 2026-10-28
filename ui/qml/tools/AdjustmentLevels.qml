import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../theme"
ColumnLayout {
    id:root
    required property var settings
    property var bins: []
    spacing:5
    AdjustmentHistogram {Layout.fillWidth:true;Layout.preferredHeight:100;bins:root.bins}
    Item {
        Layout.fillWidth:true;Layout.preferredHeight:20
        readonly property real middle:root.settings.black+(root.settings.white-root.settings.black)*Math.pow(.5,root.settings.gamma)
        Repeater {model:3
            delegate:Canvas {required property int index;property real v:index===0?root.settings.black:index===1?parent.middle:root.settings.white
                x:Math.max(0,Math.min(parent.width-14,v/255*(parent.width-14)));width:14;height:18
                onVChanged:requestPaint()
                onPaint:{const c=getContext("2d");c.reset();c.beginPath();c.moveTo(7,2);c.lineTo(13,15);c.lineTo(1,15);c.closePath();c.fillStyle=index===0?"#161616":index===1?"#818181":"#eeeeee";c.fill();c.strokeStyle="#bcbcbc";c.stroke()}
            }
        }
        MouseArea {objectName:"levelsInputDrag";anchors.fill:parent;preventStealing:true;property int handle:0
            function change(mouse){let v=Math.max(0,Math.min(255,(mouse.x-7)/(width-14)*255));if(handle===0)root.settings.black=Math.min(root.settings.white-1,Math.round(v));else if(handle===2)root.settings.white=Math.max(root.settings.black+1,Math.round(v));else {let t=Math.max(.001,Math.min(.999,(v-root.settings.black)/(root.settings.white-root.settings.black)));root.settings.gamma=Math.max(.1,Math.min(9.99,Math.log(t)/Math.log(.5)))}}
            onPressed:mouse=>{root.forceActiveFocus();let v=(mouse.x-7)/(width-14)*255,d=[Math.abs(v-root.settings.black),Math.abs(v-parent.middle),Math.abs(v-root.settings.white)];handle=d.indexOf(Math.min.apply(null,d));change(mouse)}
            onPositionChanged:mouse=>{if(pressed)change(mouse)}
        }
    }
    RowLayout {Layout.fillWidth:true
        AdjustmentValue {objectName:"levelsBlackValue";value:root.settings.black;to:root.settings.white-1;onEdited:number=>root.settings.black=number}
        Item {Layout.fillWidth:true}
        AdjustmentValue {objectName:"levelsGammaValue";value:root.settings.gamma;from:.1;to:9.99;decimals:2;onEdited:number=>root.settings.gamma=number}
        Item {Layout.fillWidth:true}
        AdjustmentValue {objectName:"levelsWhiteValue";value:root.settings.white;from:root.settings.black+1;onEdited:number=>root.settings.white=number}
    }
    Label {Layout.topMargin:10;text:"输出色阶：";color:Theme.text;font.pixelSize:10}
    Rectangle {Layout.fillWidth:true;Layout.preferredHeight:12;gradient:Gradient {orientation:Gradient.Horizontal;GradientStop {position:0;color:"black"} GradientStop {position:1;color:"white"}}}
    Item {
        Layout.fillWidth:true;Layout.preferredHeight:20
        Repeater {model:2
            delegate:Canvas {required property int index;property real v:index===0?root.settings.outputBlack:root.settings.outputWhite
                x:Math.max(0,Math.min(parent.width-14,v/255*(parent.width-14)));width:14;height:18
                onVChanged:requestPaint()
                onPaint:{const c=getContext("2d");c.reset();c.beginPath();c.moveTo(7,2);c.lineTo(13,15);c.lineTo(1,15);c.closePath();c.fillStyle=index===0?"#161616":"#eeeeee";c.fill();c.strokeStyle="#bcbcbc";c.stroke()}
            }
        }
        MouseArea {objectName:"levelsOutputDrag";anchors.fill:parent;preventStealing:true;property int handle:0
            function change(mouse){let v=Math.round(Math.max(0,Math.min(255,(mouse.x-7)/(width-14)*255)));if(handle===0)root.settings.outputBlack=v;else root.settings.outputWhite=v}
            onPressed:mouse=>{root.forceActiveFocus();let v=(mouse.x-7)/(width-14)*255;handle=Math.abs(v-root.settings.outputBlack)<=Math.abs(v-root.settings.outputWhite)?0:1;change(mouse)}
            onPositionChanged:mouse=>{if(pressed)change(mouse)}
        }
    }
    RowLayout {Layout.fillWidth:true
        AdjustmentValue {objectName:"levelsOutputBlackValue";value:root.settings.outputBlack;onEdited:number=>root.settings.outputBlack=number}
        Item {Layout.fillWidth:true}
        AdjustmentValue {objectName:"levelsOutputWhiteValue";value:root.settings.outputWhite;onEdited:number=>root.settings.outputWhite=number}
    }
}
