import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import "."

ColumnLayout {
    id:root
    readonly property var library:PaintClient.brushLibrary
    property int category:0
    property string inspectedSource:""
    readonly property var sourceInfo:PaintClient.brushSourceInfo
    function inspectSource() {
        inspectedSource=root.library.sourceResource
        if(inspectedSource.length>0)PaintClient.inspectBrushSource()
    }
    onCategoryChanged:if(category===3)inspectSource()
    readonly property real contentHeight:360
    implicitHeight:contentHeight
    spacing:6
    Label {objectName:"selectedBrushName";textFormat:Text.PlainText;text:root.library.selectedName;color:Theme.text;font.pixelSize:10;elide:Text.ElideRight;Layout.fillWidth:true}
    RowLayout {
        Layout.fillWidth:true;Layout.fillHeight:true;spacing:6
        ColumnLayout {
            Layout.preferredWidth:76;Layout.alignment:Qt.AlignTop;spacing:2
            Repeater {
                model:["笔尖形状","传递","光标与预览","来源与兼容"]
                Rectangle {
                    required property string modelData
                    required property int index
                    objectName:"brushCategory:"+index
                    Layout.fillWidth:true;implicitHeight:26
                    color:root.category===index?Theme.selected:"transparent"
                    Label {anchors.fill:parent;anchors.leftMargin:4;text:parent.modelData;color:Theme.text;font.pixelSize:10;verticalAlignment:Text.AlignVCenter}
                    MouseArea {anchors.fill:parent;onClicked:root.category=parent.index}
                }
            }
        }
        Rectangle {Layout.fillHeight:true;width:1;color:Theme.line}
        ScrollView {
            Layout.fillWidth:true;Layout.fillHeight:true;clip:true
            contentWidth:availableWidth
            ColumnLayout {
                width:parent.width;spacing:5
                ColumnLayout {
                    visible:root.category===0;Layout.fillWidth:true;spacing:5
                    RowLayout {
                        Layout.fillWidth:true
                        Button {objectName:"importImageBrushTip";text:"导入图片";implicitHeight:24;enabled:!PaintClient.brushFileBusy;onClicked:tipDialog.open()}
                        Button {objectName:"proceduralBrushTip";text:"程序笔尖";implicitHeight:24;enabled:root.library.maskResource.length>0;onClicked:root.library.useProceduralTip()}
                    }
                    CompactComboBox {id:tipInterpretation;objectName:"imageTipInterpretation";Layout.fillWidth:true;model:["灰度覆盖（白色绘制）","透明度覆盖","反向亮度（黑色绘制）"];currentIndex:2;tooltip:"灰度覆盖以白色绘制；透明度使用图片 alpha；反向亮度以黑色绘制。"}
                    Label {visible:root.library.maskResource.length>0;text:"图片覆盖 · 硬度来自图片";color:Theme.muted;font.pixelSize:9;Layout.fillWidth:true;wrapMode:Text.WordWrap}
                    Label {visible:PaintClient.brushImportMessage.length>0;text:PaintClient.brushImportMessage;color:Theme.muted;font.pixelSize:9;Layout.fillWidth:true;wrapMode:Text.WordWrap}
                    RowLayout {
                        Layout.fillWidth:true
                        Label {text:"大小";font.pixelSize:10;color:Theme.text;Layout.fillWidth:true}
                        CompactSpinBox {objectName:"settingsBrushSize";implicitWidth:68;implicitHeight:20;editable:true;from:1;to:512;value:Math.round(root.library.radius*2);onValueModified:root.library.radius=value/2}
                        Label {text:"px";font.pixelSize:9;color:Theme.muted}
                    }
                    CompactSlider {Layout.fillWidth:true;from:1;to:512;value:root.library.radius*2;onMoved:root.library.radius=value/2}
                    TipField {enabled:!root.library.maskResource.length;title:"硬度";field:"hardness";from:0;to:100;factor:100;unit:"%";help:"实心中心占笔尖半径的比例；降低后形成柔软边缘。"}
                    TipField {title:"圆度";field:"roundness";from:1;to:100;factor:100;unit:"%";help:"笔尖短轴与长轴的比例。"}
                    TipField {title:"角度";field:"angle";from:-180;to:180;factor:1;unit:"°";help:"椭圆笔尖在画布上的方向。"}
                    TipField {title:"间距";field:"spacing";from:1;to:50;factor:50;unit:"%";help:"相邻印章的距离，以直径百分比表示，最低 1%。";sliderName:"settingsBrushSpacing"}
                    CompactCheckBox {objectName:"settingsPressureSize";text:"大小压感";checked:root.library.pressureSize;font.pixelSize:10;onToggled:root.library.pressureSize=checked;tooltip:"使用笔压力控制笔尖大小。"}
                }
                ColumnLayout {
                    visible:root.category===1;Layout.fillWidth:true;spacing:5
                    TipField {title:"不透明度";field:"opacity";from:0;to:100;factor:100;unit:"%";help:"当前印章的颜色沉积比例。";sliderName:"settingsBrushOpacity"}
                    TipField {title:"流量";field:"flow";from:0;to:100;factor:100;unit:"%";help:"每次印章的沉积强度；重叠会逐步累积。"}
                    CompactCheckBox {objectName:"settingsPressureFlow";text:"流量压感";checked:root.library.pressureFlow;font.pixelSize:10;onToggled:root.library.pressureFlow=checked;tooltip:"使用笔压力控制绘画与擦除流量；涂抹强度压感由涂抹工具单独控制。"}
                }
                ColumnLayout {
                    visible:root.category===2;Layout.fillWidth:true;spacing:10
                    CompactCheckBox {objectName:"brushCursorEnabled";text:"笔尖轮廓";checked:root.library.cursorOutline;font.pixelSize:10;onToggled:root.library.cursorOutline=checked;tooltip:"显示实际圆度与角度的双线笔尖边界。"}
                    CompactCheckBox {objectName:"brushCursorSimple";text:"简化圆形";checked:root.library.cursorSimple;font.pixelSize:10;onToggled:root.library.cursorSimple=checked;tooltip:"使用标称大小的圆形轮廓，不改变实际笔尖。"}
                    CompactCheckBox {objectName:"brushCursorCenter";text:"中心十字";checked:root.library.cursorCenter;font.pixelSize:10;onToggled:root.library.cursorCenter=checked;tooltip:"在笔尖中心显示精确定位十字。"}
                    CompactCheckBox {objectName:"brushCursorHoverOnly";text:"仅悬停时显示";checked:root.library.cursorHoverOnly;font.pixelSize:10;onToggled:root.library.cursorHoverOnly=checked;tooltip:"开始绘画时隐藏笔尖预览，结束后恢复。"}
                    RowLayout {
                        Label {text:"线宽";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                        CompactSpinBox {objectName:"brushCursorWidth";from:1;to:3;editable:true;implicitWidth:66;implicitHeight:20;value:root.library.cursorWidth;onValueModified:root.library.cursorWidth=value;tooltip:"黑白轮廓线宽，单位为界面像素。"}
                    }
                    RowLayout {
                        Label {text:"轮廓颜色";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                        Rectangle {objectName:"brushCursorColor";implicitWidth:24;implicitHeight:20;color:root.library.cursorColor;border.color:Theme.muted;MouseArea {anchors.fill:parent;hoverEnabled:true;onClicked:outerColor.open();ToolTip.delay:1000;ToolTip.visible:containsMouse;ToolTip.text:"外侧轮廓颜色"}}
                        Rectangle {objectName:"brushCursorSecondaryColor";implicitWidth:24;implicitHeight:20;color:root.library.cursorSecondaryColor;border.color:Theme.muted;MouseArea {anchors.fill:parent;hoverEnabled:true;onClicked:innerColor.open();ToolTip.delay:1000;ToolTip.visible:containsMouse;ToolTip.text:"内侧轮廓颜色"}}
                    }
                    Label {text:"全局绘画偏好；不随笔刷包导出。CapsLock 切换精确光标。";Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                    Label {text:PaintClient.brushOutlineMessage;visible:text.length>0;Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                }
                ColumnLayout {
                    visible:root.category===3;Layout.fillWidth:true;spacing:6
                    Label {objectName:"brushSourceDescription";textFormat:Text.PlainText;text:root.library.sourceResource.length?"导入时快照 · 当前编辑不会改变来源报告":"此预设没有 ABR 来源记录";Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:10}
                    Button {objectName:"refreshBrushSource";text:PaintClient.brushSourceBusy?"读取中…":"刷新来源";implicitHeight:24;enabled:root.library.sourceResource.length>0 && !PaintClient.brushSourceBusy;onClicked:root.inspectSource()}
                    Label {objectName:"brushSourceError";textFormat:Text.PlainText;text:PaintClient.brushSourceMessage;visible:text.length>0;Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:10}
                    Label {objectName:"brushSourceName";textFormat:Text.PlainText;visible:root.sourceInfo.originalName!==undefined;text:root.sourceInfo.originalName || "";Layout.fillWidth:true;wrapMode:Text.WrapAnywhere;color:Theme.text;font.pixelSize:10}
                    Label {textFormat:Text.PlainText;visible:root.sourceInfo.format!==undefined;text:"ABR "+root.sourceInfo.fileVersion+(root.sourceInfo.fileSubversion?"."+root.sourceInfo.fileSubversion:"")+" · "+root.sourceInfo.converter+" v"+root.sourceInfo.converterVersion;Layout.fillWidth:true;wrapMode:Text.WrapAnywhere;color:Theme.muted;font.pixelSize:9}
                    Label {objectName:"brushSourceBaseline";textFormat:Text.PlainText;visible:root.sourceInfo.baseline!==undefined;text:root.sourceInfo.baseline?"导入大小 "+root.sourceInfo.baseline.diameter+" px · 间距 "+Math.round(root.sourceInfo.baseline.spacing*100)+"%\n圆度 "+Math.round(root.sourceInfo.baseline.roundness*100)+"% · 角度 "+root.sourceInfo.baseline.angle+"° · 流量 "+Math.round(root.sourceInfo.baseline.flow*100)+"%":"";Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                    Label {text:"兼容报告（"+(root.sourceInfo.report?root.sourceInfo.report.length:0)+" 项）";visible:root.sourceInfo.report!==undefined;color:Theme.text;font.pixelSize:10}
                    ListView {
                        objectName:"brushSourceReport";Layout.fillWidth:true;Layout.preferredHeight:160;visible:root.sourceInfo.report!==undefined;clip:true
                        model:root.sourceInfo.report || [];spacing:6
                        ScrollBar.vertical:ScrollBar {}
                        delegate:Text {
                            required property var modelData
                            width:ListView.view.width-12;textFormat:Text.PlainText;wrapMode:Text.WrapAnywhere;font.pixelSize:9;color:Theme.muted
                            text:(modelData.status==="mapped"?"已映射":modelData.status==="approximate"?"近似":"未执行")+" · "+modelData.path+"\n"+modelData.reason
                        }
                    }
                }
            }
        }
    }
    Rectangle {
        objectName:"brushStrokePreview";Layout.fillWidth:true;Layout.preferredHeight:60;color:Theme.selected
        Image {anchors.fill:parent;anchors.margins:4;source:root.library.selectedPreview;fillMode:Image.PreserveAspectFit;asynchronous:true}
        Text {anchors.centerIn:parent;text:root.library.previewMessage;font.pixelSize:9;color:Theme.muted;visible:!root.library.selectedPreview.length}
        ToolTip.delay:1000;ToolTip.visible:hover.containsMouse;ToolTip.text:"实际引擎压感笔触 · 双击重新生成"
        MouseArea {id:hover;anchors.fill:parent;hoverEnabled:true;onDoubleClicked:root.library.retryPreview()}
    }
    component TipField: ColumnLayout {
        property string title
        property string field
        property real from:0
        property real to:100
        property real factor:100
        property string unit:""
        property string help:""
        property string sliderName:"settingsBrush"+field
        id:control
        Layout.fillWidth:true;spacing:2
        RowLayout {
            Label {text:control.title;color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
            CompactSpinBox {implicitWidth:66;implicitHeight:20;editable:true;from:control.from;to:control.to;value:Math.round(root.library[control.field]*control.factor);onValueModified:root.library[control.field]=value/control.factor}
            Label {text:control.unit;color:Theme.muted;font.pixelSize:9}
        }
        CompactSlider {objectName:control.sliderName;Layout.fillWidth:true;from:control.from;to:control.to;value:root.library[control.field]*control.factor;onMoved:root.library[control.field]=value/control.factor;tooltip:control.help}
    }
    Timer {id:previewDelay;interval:100;onTriggered:root.library.requestPreview(root.library.selectedId)}
    FileDialog {id:tipDialog;title:"导入图片笔尖";nameFilters:["PNG 图片 (*.png)"];onAccepted:PaintClient.importBrushTip(selectedFile,tipInterpretation.currentIndex)}
    ColorDialog {id:outerColor;title:"外侧轮廓颜色";selectedColor:root.library.cursorColor;onAccepted:root.library.cursorColor=selectedColor}
    ColorDialog {id:innerColor;title:"内侧轮廓颜色";selectedColor:root.library.cursorSecondaryColor;onAccepted:root.library.cursorSecondaryColor=selectedColor}
    Component.onCompleted:previewDelay.restart()
    Connections {target:root.library;function onSettingsChanged(){previewDelay.restart();if(root.category===3 && root.inspectedSource!==root.library.sourceResource)root.inspectSource()}}
}
