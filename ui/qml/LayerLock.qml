import QtQuick
import QtQuick.Controls
import "."

ToolButton {
    id: root
    property int kind: 1
    property bool selected: false
    property bool passiveIcon: false
    padding: passiveIcon ? 1 : 3
    implicitWidth: 25; implicitHeight: 26
    ToolTip.visible: hovered
    ToolTip.text: kind===1 ? "锁定透明像素" : kind===2 ? "锁定位置" : "全部锁定"
    background: Rectangle { radius: 3; color: root.passiveIcon ? "transparent" : root.selected ? Theme.selected : root.hovered ? Theme.raised : "transparent"; border.color: !root.passiveIcon && root.selected ? Theme.accent : "transparent" }
    contentItem: Canvas {
        id: icon
        implicitWidth: 18; implicitHeight: 18
        Connections { target: root; function onSelectedChanged() { icon.requestPaint() } function onEnabledChanged() { icon.requestPaint() } function onKindChanged() { icon.requestPaint() } }
        onPaint: {
            const c=getContext("2d"); c.reset()
            const scale=Math.min(width,height)/18
            c.translate((width-18*scale)/2,(height-18*scale)/2); c.scale(scale,scale)
            c.strokeStyle=root.selected ? Theme.accent : Theme.muted; c.fillStyle=c.strokeStyle; c.lineWidth=1.6
            if(root.kind===1) {
                c.strokeRect(2,2,14,14)
                for(let y=0;y<3;y++) for(let x=0;x<3;x++) if((x+y)%2===0) c.fillRect(3+x*4,3+y*4,4,4)
            } else if(root.kind===2) {
                c.beginPath(); c.moveTo(9,1); c.lineTo(9,17); c.moveTo(1,9); c.lineTo(17,9)
                c.moveTo(5,5); c.lineTo(9,1); c.lineTo(13,5); c.moveTo(5,13); c.lineTo(9,17); c.lineTo(13,13)
                c.moveTo(5,5); c.lineTo(1,9); c.lineTo(5,13); c.moveTo(13,5); c.lineTo(17,9); c.lineTo(13,13); c.stroke()
            } else {
                c.beginPath(); c.arc(9,6,4,Math.PI,0); c.lineTo(13,9); c.moveTo(5,6); c.lineTo(5,9); c.stroke()
                c.fillRect(3,8,12,9); c.fillStyle=Theme.background; c.fillRect(8,11,2,3)
            }
        }
    }
}
