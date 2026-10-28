import QtQuick
Canvas {
    id: root
    property var bins: []
    onBinsChanged: requestPaint()
    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()
    onPaint: {
        const c=getContext("2d"); c.reset(); c.fillStyle="#232323"; c.fillRect(0,0,width,height)
        let peak=0;for(let i=0;i<bins.length;i++)peak=Math.max(peak,bins[i])
        if(peak>0){c.fillStyle="#575757";c.beginPath();c.moveTo(0,height)
            for(let i=0;i<bins.length;i++)c.lineTo(i*width/(bins.length-1),height-bins[i]/peak*(height-8))
            c.lineTo(width,height);c.closePath();c.fill()}
        c.strokeStyle="#4a4a4a";c.lineWidth=1;c.strokeRect(.5,.5,width-1,height-1)
    }
}
