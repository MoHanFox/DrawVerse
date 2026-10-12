import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Flickable {
    id:root;objectName:"liquifyOptionsBar"
    required property var controller
    contentWidth:content.implicitWidth;contentHeight:height;clip:true
    flickableDirection:Flickable.HorizontalFlick;boundsBehavior:Flickable.StopAtBounds
    RowLayout {
        id:content;height:root.height;spacing:5
        Icon {name:"liquify";Layout.preferredWidth:17;Layout.preferredHeight:17}
        Label {text:"液化";color:Theme.text;font.pixelSize:10}
        Repeater {
            model:[{key:"Push",title:"推拉",icon:"liquifyPush",tool:0},{key:"Bloat",title:"膨大",icon:"liquifyBloat",tool:1},{key:"Twirl",title:"扭曲",icon:"liquifyTwirl",tool:2},{key:"Reconstruct",title:"重建",icon:"liquifyReconstruct",tool:3},{key:"Reset",title:"重置",icon:"undo",tool:4},{key:"Cancel",title:"取消",icon:"close",tool:5},{key:"Commit",title:"确定",icon:"check",tool:6}]
            delegate:ToolButton {
                id:toolButton;required property var modelData
                objectName:"liquify"+modelData.key;implicitWidth:52;implicitHeight:26;padding:4
                enabled:!root.controller.liquifyBusy && !root.controller.drawing;checkable:modelData.tool<4;checked:modelData.tool<4 && root.controller.liquifyTool===modelData.tool
                background:Rectangle {radius:2;color:toolButton.checked?Theme.selected:toolButton.hovered?Theme.hover:Theme.raised}
                contentItem:RowLayout {spacing:3;Icon {name:toolButton.modelData.icon;Layout.preferredWidth:14;Layout.preferredHeight:14;color:toolButton.enabled?Theme.text:Theme.disabled}Label {text:toolButton.modelData.title;color:toolButton.enabled?Theme.text:Theme.disabled;font.pixelSize:10}}
                onClicked:{const mode=modelData.tool;if(mode<4)root.controller.liquifyTool=mode;else if(mode===4)root.controller.resetLiquify();else root.controller.finishLiquify(mode===6)}
                ToolTip.delay:1000;ToolTip.visible:hovered
                ToolTip.text:modelData.tool===0?"拖动像素；强度控制推动量":modelData.tool===1?"按住使像素向外膨大；速度控制持续变化":modelData.tool===2?"按住顺时针扭曲；速度控制旋转":modelData.tool===3?"局部恢复原始形状；强度控制恢复比例":modelData.tool===4?"恢复本次全部变形，继续液化":modelData.tool===5?"丢弃本次液化并退出":"提交为一条液化历史并退出"
            }
        }
        Rectangle {Layout.preferredWidth:1;Layout.preferredHeight:22;color:Theme.line}
        Repeater {
            model:[{key:"Size",title:"尺寸",property:"liquifySize",max:2000,unit:"px",tip:"液化笔刷直径（像素）"},{key:"Strength",title:"强度",property:"liquifyStrength",max:100,unit:"%",tip:"每次推动、膨大、扭曲或重建的幅度"},{key:"Density",title:"密度",property:"liquifyDensity",max:100,unit:"%",tip:"中心影响区域与边缘衰减；越高中心影响区域越宽、边缘更硬"},{key:"Speed",title:"速度",property:"liquifySpeed",max:100,unit:"%",tip:"按住膨大、扭曲时每秒变化的速度"}]
            delegate:RowLayout {
                id:parameter;required property var modelData;spacing:3
                readonly property bool usable:!root.controller.liquifyBusy && !root.controller.drawing && (modelData.key!=="Speed" || root.controller.liquifyTool===1 || root.controller.liquifyTool===2)
                Label {text:parameter.modelData.title;Layout.preferredWidth:24;color:parameter.usable?Theme.text:Theme.disabled;font.pixelSize:10}
                CompactSlider {objectName:"liquify"+parameter.modelData.key;Layout.preferredWidth:44;enabled:parameter.usable;from:1;to:parameter.modelData.max;stepSize:1;value:root.controller[parameter.modelData.property];onMoved:root.controller[parameter.modelData.property]=Math.round(value);tooltip:parameter.modelData.tip}
                CompactSpinBox {objectName:"liquify"+parameter.modelData.key+"Input";Layout.preferredWidth:66;implicitHeight:22;editable:true;enabled:parameter.usable;from:1;to:parameter.modelData.max;value:root.controller[parameter.modelData.property];onValueModified:root.controller[parameter.modelData.property]=value;tooltip:parameter.modelData.tip}
                Label {text:parameter.modelData.unit;color:Theme.muted;font.pixelSize:9;Layout.preferredWidth:12}
            }
        }
        IconButton {objectName:"liquifyPressure";glyph:"pressure";Layout.preferredWidth:28;Layout.preferredHeight:26;enabled:!root.controller.liquifyBusy && !root.controller.drawing;checkable:true;checked:root.controller.liquifyPressure;tooltip:checked?"压感已开启：压力控制液化强度":"压感已关闭：点击开启";onClicked:root.controller.liquifyPressure=!root.controller.liquifyPressure}
        CheckBox {objectName:"liquifyFocus";text:"专注图层";enabled:!root.controller.liquifyBusy && !root.controller.drawing;checked:root.controller.liquifyFocus;font.pixelSize:10;onClicked:{const value=checked;Qt.callLater(()=>root.controller.liquifyFocus=value)}
            ToolTip.delay:1000;ToolTip.visible:hovered;ToolTip.text:"临时隐藏其他图层；退出液化后恢复原来的图层显示"}
    }
    ScrollBar.horizontal:ScrollBar {height:3;policy:ScrollBar.AsNeeded}
}
