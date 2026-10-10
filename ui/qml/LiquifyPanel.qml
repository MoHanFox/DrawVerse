import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

// Non-movable liquefy panel pinned to the bottom of the canvas window. It is the single place the
// parameters live; the canvas forwards gestures into the controller instead of keeping its own copy.
Rectangle {
    id: root
    objectName: "liquifyPanel"
    property var controller
    readonly property bool busy: controller ? controller.busy : false
    readonly property bool active: controller ? controller.active : false
    readonly property int tool: controller ? controller.tool : 0
    implicitWidth: content.implicitWidth + 20
    implicitHeight: content.implicitHeight + 14
    visible: active
    color: Theme.surface
    radius: 4
    border.color: Theme.line

    function setTool(value) { if (controller) controller.tool = value }

    ColumnLayout {
        id: content
        anchors.centerIn: parent
        spacing: 6
        RowLayout {
            spacing: 4
            IconButton { objectName:"liquifyPush"; glyph:"move"; checkable:true; checked:root.tool===0; tooltip:"推拉"; onClicked:root.setTool(0) }
            IconButton { objectName:"liquifyBloat"; glyph:"brush"; checkable:true; checked:root.tool===1; tooltip:"膨大"; onClicked:root.setTool(1) }
            IconButton { objectName:"liquifyTwirl"; glyph:"redo"; checkable:true; checked:root.tool===2; tooltip:"扭曲"; onClicked:root.setTool(2) }
            IconButton { objectName:"liquifyReconstruct"; glyph:"history"; checkable:true; checked:root.tool===3; tooltip:"重建"; onClicked:root.setTool(3) }
            Rectangle { width:1; height:20; color:Theme.line }
            Button { objectName:"liquifyReset"; text:"重置"; font.pixelSize:9; implicitHeight:22; enabled:!root.busy; onClicked:if(root.controller)root.controller.reset() }
            Button { objectName:"liquifyCancel"; text:"取消"; font.pixelSize:9; implicitHeight:22; enabled:!root.busy; onClicked:if(root.controller)root.controller.cancel() }
            Button { objectName:"liquifyCommit"; text:"确定"; font.pixelSize:9; implicitHeight:22; enabled:!root.busy; onClicked:if(root.controller)root.controller.commit() }
        }
        RowLayout {
            spacing: 8
            Label { text:"尺寸"; color:Theme.text; font.pixelSize:9 }
            CompactSlider { objectName:"liquifySize"; Layout.preferredWidth:120; from:1; to:2000; value:root.controller?root.controller.size:200; onMoved:if(root.controller)root.controller.size=value }
            CompactSpinBox { objectName:"liquifySizeInput"; implicitWidth:64; implicitHeight:20; editable:true; from:1; to:2000; value:Math.round(root.controller?root.controller.size:200); onValueModified:if(root.controller)root.controller.size=value }
            Label { text:"px"; color:Theme.muted; font.pixelSize:8 }
        }
        RowLayout {
            spacing: 8
            Label { text:"强度"; color:Theme.text; font.pixelSize:9 }
            CompactSlider { objectName:"liquifyStrength"; Layout.preferredWidth:120; from:1; to:100; value:root.controller?root.controller.strength:50; onMoved:if(root.controller)root.controller.strength=value }
            CompactSpinBox { objectName:"liquifyStrengthInput"; implicitWidth:56; implicitHeight:20; editable:true; from:1; to:100; value:Math.round(root.controller?root.controller.strength:50); onValueModified:if(root.controller)root.controller.strength=value }
            Label { text:"%"; color:Theme.muted; font.pixelSize:8 }
        }
        RowLayout {
            spacing: 8
            Label { text:"密度"; color:Theme.text; font.pixelSize:9 }
            CompactSlider { objectName:"liquifyDensity"; Layout.preferredWidth:120; from:1; to:100; value:root.controller?root.controller.density:50; onMoved:if(root.controller)root.controller.density=value }
            CompactSpinBox { objectName:"liquifyDensityInput"; implicitWidth:56; implicitHeight:20; editable:true; from:1; to:100; value:Math.round(root.controller?root.controller.density:50); onValueModified:if(root.controller)root.controller.density=value }
            Label { text:"%"; color:Theme.muted; font.pixelSize:8 }
        }
        RowLayout {
            spacing: 8
            // Speed only affects bloat and twirl; push and reconstruct grey it out instead of
            // offering a control that would do nothing.
            readonly property bool speedUsable: root.tool===1 || root.tool===2
            Label { text:"速度"; color:parent.speedUsable?Theme.text:Theme.disabled; font.pixelSize:9 }
            CompactSlider {
                objectName:"liquifySpeed"; Layout.preferredWidth:120; enabled:parent.speedUsable
                from:1; to:100; value:root.controller?root.controller.speed:50
                onMoved:if(root.controller)root.controller.speed=value
            }
            CompactSpinBox {
                objectName:"liquifySpeedInput"; implicitWidth:56; implicitHeight:20; editable:true
                enabled:parent.speedUsable; from:1; to:100
                value:Math.round(root.controller?root.controller.speed:50)
                onValueModified:if(root.controller)root.controller.speed=value
            }
            Label { text:"%"; color:Theme.muted; font.pixelSize:8 }
        }
    }
}
