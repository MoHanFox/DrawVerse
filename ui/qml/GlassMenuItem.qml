import QtQuick
import QtQuick.Controls
import "."

MenuItem {
    id: control
    hoverEnabled: true
    objectName: "glassMenuItem:"+text
    topPadding: 3
    bottomPadding: 3
    implicitHeight: implicitContentHeight+topPadding+bottomPadding
    Timer {
        // Continue honoring hover over sibling categories when a separate child window
        // owns focus, including Qt's window popups at scaled display coordinates.
        interval:180
        running:control.enabled && control.hovered && control.subMenu!==null && !!control.subMenu.cascadeParent
                && !control.subMenu.visible
        onTriggered:control.subMenu.cascadeParent.openChild(control.subMenu)
    }
    // The Basic style's image arrow scales with the source DPI and can dwarf a compact row.
    arrow: Canvas {
        objectName: "submenuArrow:"+control.text
        width: 6; height: 8
        visible: control.subMenu !== null
        x: control.width-control.rightPadding-width
        y: (control.height-height)/2
        onPaint: {
            const c=getContext("2d");c.reset();c.strokeStyle=control.enabled?Theme.text:Theme.disabled
            c.lineWidth=1.3;c.beginPath();c.moveTo(1,1);c.lineTo(5,4);c.lineTo(1,7);c.stroke()
        }
    }
    palette.windowText: enabled ? Theme.text : Theme.disabled
    background: Rectangle {
        implicitWidth: 200
        implicitHeight: 0
        radius: 0
        color: control.down ? Theme.selected : control.highlighted ? Theme.hover : "transparent"
    }
}
