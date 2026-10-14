import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import "."

ColumnLayout {
    id:root
    readonly property var library:PaintClient.brushLibrary
    property int category:0
    property int shapePage:0
    property int dualPage:0
    readonly property var dual:library.dualBrush
    function dualModule(name) {
        if(dual[name])return dual[name]
        if(name==="tipDynamics")return {angleControl:"fixed",tiltRoundness:false,minimumRoundness:0.01}
        if(name==="scatterDynamics")return {enabled:false,amount:0,bothAxes:false,count:1,countJitter:0,seed:"0000000000000000"}
        return {enabled:false,size:0,minimumSize:0,angle:0,roundness:0,minimumRoundness:0.01,seed:"0000000000000000"}
    }
    function setDualField(key,value,module) {
        let data=Object.assign({},dual)
        if(module){let nested=Object.assign({},dualModule(module));nested[key]=value;data[module]=nested}
        else data[key]=value
        return library.setDualBrush(data)
    }
    QtObject {
        id:dualPressure
        readonly property var pressureDynamics:root.dual.pressureDynamics || {size:{curve:[[0,0],[1,1]],minimum:0},flow:{curve:[[0,0],[1,1]],minimum:0}}
        signal settingsChanged()
        function setPressureControl(target,points,minimum){return root.library.setDualPressureControl(target,points,minimum)}
        function resetPressureControl(target){root.library.resetDualPressureControl(target)}
    }
    Connections {target:root.library;function onSettingsChanged(){dualPressure.settingsChanged()}}
    property string inspectedSource:""
    readonly property var sourceInfo:PaintClient.brushSourceInfo
    function inspectSource() {
        inspectedSource=root.library.sourceResource
        if(inspectedSource.length>0)PaintClient.inspectBrushSource()
    }
    function setTextureField(key,value) {let data=Object.assign({},root.library.texture);data[key]=value;root.library.setTexture(data)}
    onCategoryChanged:if(category===8)inspectSource()
    readonly property real contentHeight:360
    implicitHeight:contentHeight
    spacing:6
    Label {objectName:"selectedBrushName";textFormat:Text.PlainText;text:root.library.selectedName;color:Theme.text;font.pixelSize:10;elide:Text.ElideRight;Layout.fillWidth:true}
    RowLayout {
        Layout.fillWidth:true;Layout.fillHeight:true;spacing:6
        ColumnLayout {
            Layout.preferredWidth:76;Layout.minimumWidth:76;Layout.maximumWidth:76;Layout.alignment:Qt.AlignTop;spacing:2
            Repeater {
                model:["笔尖形状","形状动态","散布","纹理","双重笔刷","颜色动态","传递","光标与预览","来源与兼容","平滑"]
                Rectangle {
                    required property string modelData
                    required property int index
                    objectName:"brushCategory:"+index
                    Layout.fillWidth:true;implicitHeight:26
                    color:root.category===index?Theme.selected:"transparent"
                    Label {anchors.fill:parent;anchors.leftMargin:4;text:parent.modelData;color:Theme.text;font.pixelSize:10;verticalAlignment:Text.AlignVCenter;elide:Text.ElideRight}
                    MouseArea {anchors.fill:parent;onClicked:root.category=parent.index}
                }
            }
        }
        Rectangle {Layout.fillHeight:true;width:1;color:Theme.line}
        ScrollView {
            id:parameterScroll;objectName:"brushParameterScroll"
            Layout.fillWidth:true;Layout.fillHeight:true;clip:true
            contentWidth:availableWidth
            ColumnLayout {
                width:parameterScroll.availableWidth;spacing:5
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
                }
                ColumnLayout {
                    visible:root.category===1;Layout.fillWidth:true;spacing:5
                    CompactComboBox {objectName:"settingsShapePage";Layout.fillWidth:true;model:["设备与压感","随机抖动"];currentIndex:root.shapePage;onActivated:root.shapePage=currentIndex;tooltip:"选择形状动态设置；两个页面的参数可同时作用。"}
                    ColumnLayout {visible:root.shapePage===0;Layout.fillWidth:true;spacing:5
                    RowLayout {
                        Layout.fillWidth:true
                        Label {text:"角度控制";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                        CompactComboBox {
                            objectName:"settingsAngleControl";implicitWidth:90
                            readonly property var controls:["fixed","direction","pen-rotation","tilt"]
                            model:["关闭","方向","笔旋转","笔倾斜"]
                            currentIndex:controls.indexOf(root.library.angleControl)
                            onActivated:root.library.angleControl=controls[currentIndex]
                            tooltip:"相对基础角度叠加路径方向、笔旋转或倾斜方位；缺失设备能力时使用基础角度。"
                        }
                    }
                    CompactCheckBox {objectName:"settingsTiltRoundness";text:"倾斜控制圆度";checked:root.library.tiltRoundness;font.pixelSize:10;onToggled:root.library.tiltRoundness=checked;tooltip:"按笔倾斜缩短短轴；没有倾斜能力时保留基础圆度。关闭后保留最低比例。"}
                    RowLayout {
                        Layout.fillWidth:true
                        Label {text:"最低圆度比例";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                        CompactSpinBox {objectName:"settingsMinimumRoundness";from:1;to:100;editable:true;implicitWidth:66;implicitHeight:20;value:Math.round(root.library.minimumRoundness*100);onValueModified:root.library.minimumRoundness=value/100;tooltip:"倾斜圆度相对于基础圆度的最低比例；最终圆度不会低于1%。"}
                        Label {text:"%";color:Theme.muted;font.pixelSize:9}
                    }
                    CompactCheckBox {objectName:"settingsPressureSize";text:"大小压感";checked:root.library.pressureSize;font.pixelSize:10;onToggled:root.library.pressureSize=checked;tooltip:"按压力曲线和最低比例控制笔尖大小；关闭后保留曲线。"}
                    PressureCurveEditor {library:root.library;target:"size"}
                    }
                    ColumnLayout {visible:root.shapePage===1;Layout.fillWidth:true;spacing:5
                        CompactCheckBox {objectName:"settingsShapeJitterEnabled";text:"启用形状抖动";checked:root.library.shapeJitterEnabled;font.pixelSize:10;onToggled:root.library.shapeJitterEnabled=checked;tooltip:"每个实际印章分别随机改变形状；关闭时保留参数。"}
                        ColumnLayout {Layout.fillWidth:true;spacing:5
                            TipField {title:"大小抖动";field:"shapeSizeJitter";from:0;to:100;factor:100;unit:"%";help:"大小在最低比例与原始大小之间变化；不改变笔尖间距。";sliderName:"settingsShapeSizeJitter"}
                            TipField {title:"最小直径比例";field:"shapeMinimumSize";from:0;to:100;factor:100;unit:"%";help:"随机大小相对于压感大小的最低比例。";sliderName:"settingsShapeMinimumSize"}
                            TipField {title:"角度抖动";field:"shapeAngleJitter";from:0;to:100;factor:100;unit:"%";help:"最大随机偏移为此比例乘以180度，叠加到设备或路径角度。";sliderName:"settingsShapeAngleJitter"}
                            TipField {title:"圆度抖动";field:"shapeRoundnessJitter";from:0;to:100;factor:100;unit:"%";help:"随机缩短笔尖短轴，叠加到基础和设备圆度。";sliderName:"settingsShapeRoundnessJitter"}
                            TipField {title:"最低圆度比例";field:"shapeMinimumRoundness";from:1;to:100;factor:100;unit:"%";help:"随机圆度相对于当前圆度的最低比例，最终圆度至少1%。";sliderName:"settingsShapeMinimumRoundness"}
                        }
                    }
                }
                ColumnLayout {
                    visible:root.category===2;Layout.fillWidth:true;spacing:5
                    CompactCheckBox {objectName:"settingsScatterEnabled";text:"启用散布";checked:root.library.scatterEnabled;font.pixelSize:10;onToggled:root.library.scatterEnabled=checked;tooltip:"在路径周围分散笔尖印章；关闭时保留参数。"}
                    ColumnLayout {
                        Layout.fillWidth:true;spacing:5
                        TipField {title:"散布";field:"scatterAmount";from:0;to:1000;factor:100;unit:"%";help:"相对名义直径的最大偏移；单轴沿路径垂直方向，双轴沿画布两个方向。";sliderName:"settingsScatterAmount"}
                        CompactCheckBox {objectName:"settingsScatterBothAxes";text:"双轴";checked:root.library.scatterBothAxes;font.pixelSize:10;onToggled:root.library.scatterBothAxes=checked;tooltip:"在画布水平和垂直方向分别随机偏移。"}
                        TipField {title:"数量";field:"scatterCount";from:1;to:16;factor:1;help:"每个路径印章产生的最大印章数量；增加数量会增加绘画工作量。";sliderName:"settingsScatterCount"}
                        TipField {title:"数量抖动";field:"scatterCountJitter";from:0;to:100;factor:100;unit:"%";help:"在最大数量和降低后的数量之间随机变化，每次至少一个印章。";sliderName:"settingsScatterCountJitter"}
                    }
                }
                ColumnLayout {Layout.minimumWidth:0;visible:root.category===3;Layout.fillWidth:true;spacing:5
                    CompactCheckBox {objectName:"settingsTextureEnabled";text:"启用纹理";enabled:!!root.library.texture.resource;checked:root.library.texture.enabled;font.pixelSize:10;onToggled:root.setTextureField("enabled",checked);tooltip:"使用图片覆盖调制笔触；关闭保留纹理和参数。"}
                    RowLayout {Layout.fillWidth:true
                        Button {objectName:"importBrushTexture";text:"导入";Layout.fillWidth:true;Layout.minimumWidth:0;Layout.preferredWidth:0;implicitHeight:24;enabled:!PaintClient.brushFileBusy;onClicked:textureDialog.open();ToolTip.delay:1000;ToolTip.visible:hovered;ToolTip.text:"导入 PNG 纹理"}
                        Button {objectName:"removeBrushTexture";text:"移除";Layout.fillWidth:true;Layout.minimumWidth:0;Layout.preferredWidth:0;implicitHeight:24;enabled:!!root.library.texture.resource;onClicked:root.library.removeTexture();ToolTip.delay:1000;ToolTip.visible:hovered;ToolTip.text:"移除纹理图片并关闭纹理，保留其他参数"}
                    }
                    CompactComboBox {id:textureInterpretation;objectName:"textureInterpretation";Layout.fillWidth:true;model:["灰度覆盖（白色绘制）","透明度覆盖","反向亮度（黑色绘制）"];currentIndex:0;tooltip:"将图片转换为可重复的灰度覆盖，纹理与笔尖独立。"}
                    Label {text:root.library.texture.resource?"已加载纹理；关闭时仍可编辑。":"先导入 PNG 纹理，再启用。";Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                    Label {visible:PaintClient.brushImportMessage.length>0;text:PaintClient.brushImportMessage;color:Theme.muted;font.pixelSize:9;Layout.fillWidth:true;wrapMode:Text.WordWrap}
                    CompactComboBox {objectName:"settingsTextureSpace";Layout.fillWidth:true;model:["文档纸纹","笔尖局部"];currentIndex:root.library.texture.space==="tip"?1:0;onActivated:root.setTextureField("space",currentIndex===1?"tip":"document");tooltip:"纸纹固定在文档坐标；局部纹理随每个印章的位置和角度变化。"}
                    TextureField {title:"比例";field:"scale";from:1;to:10000;factor:100;unit:"%";help:"源图片一个像素对应的文档像素比例，100%为原始大小。"}
                    TextureField {title:"角度";field:"angle";from:-180;to:180;factor:1;unit:"°";help:"旋转纹理采样方向。"}
                    TextureField {title:"深度";field:"depth";from:0;to:100;factor:100;unit:"%";help:"纹理覆盖影响强度；纸纹在同一笔触反复擦拭时保持覆盖。"}
                    CompactCheckBox {objectName:"settingsTextureInvert";text:"反相";checked:root.library.texture.invert;font.pixelSize:10;onToggled:root.setTextureField("invert",checked);tooltip:"交换纹理黑白覆盖。"}
                    TextureField {title:"水平偏移";field:"offsetX";from:-1000000;to:1000000;factor:1;unit:"px";help:"纹理相位在文档水平方向的偏移。"}
                    TextureField {title:"垂直偏移";field:"offsetY";from:-1000000;to:1000000;factor:1;unit:"px";help:"纹理相位在文档垂直方向的偏移。"}
                }
                ColumnLayout {Layout.minimumWidth:0;visible:root.category===4;Layout.fillWidth:true;spacing:5
                    CompactCheckBox {objectName:"settingsDualEnabled";text:"启用双重笔刷";checked:root.dual.enabled;font.pixelSize:10;onToggled:root.setDualField("enabled",checked);tooltip:"第二笔尖调制主笔尖覆盖；关闭时保留全部设置。"}
                    CompactComboBox {objectName:"settingsDualPage";Layout.fillWidth:true;model:["笔尖与组合","设备与压力","散布","形状抖动"];currentIndex:root.dualPage;onActivated:root.dualPage=currentIndex;tooltip:"编辑第二笔尖的形状、压力与随机动态。"}
                    ColumnLayout {Layout.minimumWidth:0;visible:root.dualPage===0;Layout.fillWidth:true;spacing:5
                        CompactComboBox {objectName:"settingsDualMode";Layout.fillWidth:true;readonly property var modes:["multiply","darken","lighten","subtract","difference","screen"];model:["正片叠底","变暗","变亮","减去","差值","滤色"];currentIndex:modes.indexOf(root.dual.mode);onActivated:root.setDualField("mode",modes[currentIndex]);tooltip:"组合两个笔尖的灰度覆盖。"}
                        CompactComboBox {objectName:"settingsDualModel";Layout.fillWidth:true;model:["独立路径间距","每个主印章配对"];currentIndex:root.dual.model==="paired-dab-v1"?1:0;onActivated:{let data=Object.assign({},root.dual);data.model=currentIndex===1?"paired-dab-v1":"path-coverage-v1";data.spacing=currentIndex===1?null:0.25;root.library.setDualBrush(data)} tooltip:"独立路径使用第二笔尖自己的间距；配对在每个主印章的位置生成第二笔尖。"}
                        RowLayout {Layout.fillWidth:true
                            Button {objectName:"importDualBrushTip";text:"导入";Layout.fillWidth:true;Layout.minimumWidth:0;implicitHeight:24;enabled:!PaintClient.brushFileBusy;onClicked:dualDialog.open();ToolTip.delay:1000;ToolTip.visible:hovered;ToolTip.text:"导入第二笔尖 PNG 图片"}
                            Button {objectName:"proceduralDualBrushTip";text:"程序";Layout.fillWidth:true;Layout.minimumWidth:0;implicitHeight:24;enabled:!!root.dual.resource;onClicked:root.library.useProceduralDual();ToolTip.delay:1000;ToolTip.visible:hovered;ToolTip.text:"恢复程序第二笔尖，保留其他参数"}
                        }
                        CompactComboBox {id:dualInterpretation;objectName:"dualTipInterpretation";Layout.fillWidth:true;model:["灰度覆盖（白色绘制）","透明度覆盖","反向亮度（黑色绘制）"];currentIndex:2;tooltip:"选择导入图片的覆盖解释。"}
                        Label {visible:PaintClient.brushImportMessage.length>0;text:PaintClient.brushImportMessage;Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                        DualField {title:"大小";field:"diameter";from:1;to:512;factor:1;unit:"px"}
                        DualField {title:"间距";field:"spacing";from:1;to:100;factor:100;unit:"%";visible:root.dual.model==="path-coverage-v1"}
                        DualField {title:"硬度";field:"hardness";enabled:!root.dual.resource}
                        DualField {title:"圆度";field:"roundness";from:1}
                        DualField {title:"角度";field:"angle";from:-180;to:180;factor:1;unit:"°"}
                        DualField {title:"流量";field:"flow"}
                        Label {text:"画布光标显示主笔尖；第二笔尖改变实际覆盖。";Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                    }
                    ColumnLayout {Layout.minimumWidth:0;visible:root.dualPage===1;Layout.fillWidth:true;spacing:5
                        CompactComboBox {objectName:"settingsDualAngleControl";Layout.fillWidth:true;readonly property var controls:["fixed","direction","pen-rotation","tilt"];model:["固定角度","路径方向","笔旋转","笔倾斜"];currentIndex:controls.indexOf(root.dualModule("tipDynamics").angleControl);onActivated:root.setDualField("angleControl",controls[currentIndex],"tipDynamics");tooltip:"第二笔尖独立叠加方向、旋转或倾斜角度；缺失设备能力使用基础角度。"}
                        CompactCheckBox {objectName:"settingsDualTiltRoundness";text:"倾斜控制圆度";checked:root.dualModule("tipDynamics").tiltRoundness;onToggled:root.setDualField("tiltRoundness",checked,"tipDynamics");tooltip:"按倾斜缩短第二笔尖短轴；关闭后保留最低比例。"}
                        DualField {title:"最低圆度比例";field:"minimumRoundness";module:"tipDynamics";from:1}
                        CompactCheckBox {objectName:"settingsDualPressureSize";text:"大小压感";checked:root.dual.pressureSize;onToggled:root.setDualField("pressureSize",checked);tooltip:"压力曲线控制第二笔尖；关闭时保留曲线和最低比例。"}
                        PressureCurveEditor {library:dualPressure;target:"size";prefix:"dualPressureSize"}
                        CompactCheckBox {objectName:"settingsDualPressureFlow";text:"流量压感";checked:root.dual.pressureFlow;onToggled:root.setDualField("pressureFlow",checked);tooltip:"压力曲线控制第二笔尖；关闭时保留曲线和最低比例。"}
                        PressureCurveEditor {library:dualPressure;target:"flow";prefix:"dualPressureFlow"}
                    }
                    ColumnLayout {Layout.minimumWidth:0;visible:root.dualPage===2;Layout.fillWidth:true;spacing:5
                        CompactCheckBox {objectName:"settingsDualScatterEnabled";text:"启用散布";checked:root.dualModule("scatterDynamics").enabled;onToggled:root.setDualField("enabled",checked,"scatterDynamics");tooltip:"散布第二笔尖；关闭后保留参数。"}
                        DualField {title:"散布";field:"amount";module:"scatterDynamics";to:1000}
                        CompactCheckBox {objectName:"settingsDualScatterBothAxes";text:"双轴";checked:root.dualModule("scatterDynamics").bothAxes;onToggled:root.setDualField("bothAxes",checked,"scatterDynamics");tooltip:"沿画布两个方向分别随机偏移。"}
                        DualField {title:"数量";field:"count";module:"scatterDynamics";from:1;to:16;factor:1;unit:""}
                        DualField {title:"数量抖动";field:"countJitter";module:"scatterDynamics"}
                    }
                    ColumnLayout {Layout.minimumWidth:0;visible:root.dualPage===3;Layout.fillWidth:true;spacing:5
                        CompactCheckBox {objectName:"settingsDualShapeEnabled";text:"启用形状抖动";checked:root.dualModule("shapeJitter").enabled;onToggled:root.setDualField("enabled",checked,"shapeJitter");tooltip:"独立随机改变第二笔尖形状；关闭后保留参数。"}
                        DualField {title:"大小抖动";field:"size";module:"shapeJitter"}
                        DualField {title:"最小直径比例";field:"minimumSize";module:"shapeJitter"}
                        DualField {title:"角度抖动";field:"angle";module:"shapeJitter"}
                        DualField {title:"圆度抖动";field:"roundness";module:"shapeJitter"}
                        DualField {title:"最低圆度比例";field:"minimumRoundness";module:"shapeJitter";from:1}
                    }
                }
                ColumnLayout {Layout.minimumWidth:0;visible:root.category===5;Layout.fillWidth:true;spacing:5
                    CompactCheckBox {objectName:"settingsColorDynamicsEnabled";text:"启用颜色动态";checked:root.library.colorDynamicsEnabled;font.pixelSize:10;onToggled:root.library.colorDynamicsEnabled=checked;tooltip:"在当前前景色与背景色之间混合并随机调整颜色；关闭保留参数。"}
                    ColumnLayout {Layout.fillWidth:true;spacing:5
                        CompactCheckBox {objectName:"settingsColorPerTip";text:"应用于每个笔尖";checked:root.library.colorPerTip;font.pixelSize:10;onToggled:root.library.colorPerTip=checked;tooltip:"开启时每个实际印章取样新颜色，关闭时整条笔触使用同一随机颜色。"}
                        TipField {title:"前景/背景抖动";field:"foregroundBackgroundJitter";from:0;to:100;factor:100;unit:"%";help:"在前景与背景之间随机混合，100%允许全部范围。";sliderName:"settingsForegroundBackgroundJitter"}
                        TipField {title:"色相抖动";field:"hueJitter";from:0;to:100;factor:100;unit:"%";help:"围绕原色相随机偏移，100%最大偏移半圈。";sliderName:"settingsHueJitter"}
                        TipField {title:"饱和度抖动";field:"saturationJitter";from:0;to:100;factor:100;unit:"%";help:"随机增加或减少饱和度，结果限制在有效范围。";sliderName:"settingsSaturationJitter"}
                        TipField {title:"明度抖动";field:"valueJitter";from:0;to:100;factor:100;unit:"%";help:"随机增加或减少明度，不改变前景透明度。";sliderName:"settingsValueJitter"}
                    }
                    Label {text:"下方预览使用红色前景与蓝色背景示例。";Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                }
                ColumnLayout {
                    visible:root.category===6;Layout.fillWidth:true;spacing:5
                    TipField {title:"不透明度";field:"opacity";from:0;to:100;factor:100;unit:"%";help:"绘画与擦除的整笔不透明度上限；涂抹仍使用强度语义。";sliderName:"settingsBrushOpacity"}
                    TipField {title:"流量";field:"flow";from:0;to:100;factor:100;unit:"%";help:"每次印章的沉积强度；重叠会逐步累积。"}
                    CompactCheckBox {objectName:"settingsPressureOpacity";text:"不透明度压感";checked:root.library.pressureOpacity;font.pixelSize:10;onToggled:root.library.pressureOpacity=checked;tooltip:"压力决定局部整笔不透明度上限；较低压力不会使已画出的颜色变浅。流量控制接近上限的速度。"}
                    PressureCurveEditor {library:root.library;target:"opacity"}
                    CompactCheckBox {objectName:"settingsPressureFlow";text:"流量压感";checked:root.library.pressureFlow;font.pixelSize:10;onToggled:root.library.pressureFlow=checked;tooltip:"使用笔压力控制绘画与擦除流量；涂抹强度压感由涂抹工具单独控制。"}
                    PressureCurveEditor {library:root.library;target:"flow"}
                    Label {text:"涂抹使用此流量曲线，由涂抹工具的强度压感开关控制。";Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                }
                ColumnLayout {
                    visible:root.category===7;Layout.fillWidth:true;spacing:10
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
                    visible:root.category===8;Layout.fillWidth:true;spacing:6
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
                ColumnLayout {visible:root.category===9;Layout.fillWidth:true;Layout.minimumWidth:0;spacing:5
                    CompactCheckBox {objectName:"settingsSmoothingEnabled";text:"启用平滑";checked:root.library.smoothing.enabled;onToggled:{let data=Object.assign({},root.library.smoothing);data.enabled=checked;root.library.setSmoothing(data)} tooltip:"平滑位置和压力输入；主笔刷与第二笔刷共用同一路径。关闭时仍保留参数。"}
                    SmoothingField {title:"位置平滑";field:"positionMs";to:250;unit:"ms";help:"减轻指针抖动；数值越大，跟随延迟越明显。"}
                    SmoothingField {title:"压力平滑";field:"pressureMs";to:250;unit:"ms";help:"独立平滑压力变化；不改变位置滤波的时间设置。"}
                    SmoothingField {title:"速度响应";field:"speedResponse";factor:100;to:100;unit:"%";help:"快速移动时减少位置平滑延迟；压力平滑保持独立。"}
                    SmoothingField {title:"稳定距离";field:"stabilizerPx";to:128;unit:"px";help:"指针移动超过此距离后画笔开始跟随；抬笔时补齐终点。"}
                }
                Button {objectName:"resetBrushPreset";text:"重置预设";implicitHeight:22;Layout.minimumWidth:0;Layout.maximumWidth:parameterScroll.availableWidth;Layout.alignment:Qt.AlignLeft;Layout.topMargin:6;onClicked:root.library.resetPreset();ToolTip.delay:1000;ToolTip.visible:hovered;ToolTip.text:"恢复此预设的初始参数；不改变当前颜色、工具或画布历史。"}
            }
        }
    }
    Rectangle {
        objectName:"brushStrokePreview";Layout.fillWidth:true;Layout.preferredHeight:60;color:Theme.selected
        Image {objectName:"brushStrokePreviewImage";anchors.fill:parent;anchors.margins:4;source:root.library.selectedPreview;fillMode:Image.PreserveAspectFit;asynchronous:true}
        Text {anchors.centerIn:parent;text:root.library.previewMessage;font.pixelSize:9;color:Theme.muted;visible:!root.library.selectedPreview.length}
        ToolTip.delay:1000;ToolTip.visible:hover.containsMouse;ToolTip.text:"实际引擎压感笔触 · 预览固定正常模式 · 双击重新生成"
        MouseArea {id:hover;anchors.fill:parent;hoverEnabled:true;onDoubleClicked:root.library.retryPreview()}
    }
    component SmoothingField: ColumnLayout {
        id:smoothingControl
        property string title
        property string field
        property real factor:1
        property real to:100
        property string unit:""
        property string help:""
        Layout.fillWidth:true;Layout.minimumWidth:0;spacing:2
        function commit(value){let data=Object.assign({},root.library.smoothing);data[field]=value/factor;root.library.setSmoothing(data)}
        Label {text:smoothingControl.title+" ("+smoothingControl.unit+")";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true;Layout.minimumWidth:0;elide:Text.ElideRight}
        CompactSpinBox {objectName:"settingsSmoothing"+smoothingControl.field+"Input";Layout.fillWidth:true;Layout.minimumWidth:0;implicitHeight:20;from:0;to:smoothingControl.to;editable:true;value:Math.round(root.library.smoothing[smoothingControl.field]*smoothingControl.factor);onValueModified:smoothingControl.commit(value);tooltip:smoothingControl.help}
        CompactSlider {objectName:"settingsSmoothing"+smoothingControl.field;Layout.fillWidth:true;from:0;to:smoothingControl.to;value:root.library.smoothing[smoothingControl.field]*smoothingControl.factor;onMoved:smoothingControl.commit(value);tooltip:smoothingControl.help}
    }
    component DualField: ColumnLayout {
        id:dualControl
        property string title
        property string field
        property string module:""
        property real from:0
        property real to:100
        property real factor:100
        property string unit:"%"
        readonly property real currentValue:Number((module?root.dualModule(module):root.dual)[field] || 0)*factor
        Layout.fillWidth:true;Layout.minimumWidth:0;spacing:2
        Label {text:dualControl.title+(dualControl.unit.length?" ("+dualControl.unit+")":"");color:Theme.text;font.pixelSize:10;Layout.fillWidth:true;Layout.minimumWidth:0;elide:Text.ElideRight}
        CompactSpinBox {objectName:"settingsDual"+dualControl.module+dualControl.field+"Input";Layout.fillWidth:true;Layout.minimumWidth:0;implicitHeight:20;editable:true;from:dualControl.from;to:dualControl.to;value:Math.round(dualControl.currentValue);onValueModified:root.setDualField(dualControl.field,value/dualControl.factor,dualControl.module);tooltip:"第二笔尖的"+dualControl.title+"；关闭功能后仍保留设置。"}
        CompactSlider {objectName:"settingsDual"+dualControl.module+dualControl.field;Layout.fillWidth:true;from:dualControl.from;to:dualControl.to;value:dualControl.currentValue;onMoved:root.setDualField(dualControl.field,value/dualControl.factor,dualControl.module)}
    }
    component TextureField: ColumnLayout {
        property string title
        property string field
        property real from:0
        property real to:100
        property real factor:100
        property string unit:""
        property string help:""
        id:textureControl
        Layout.fillWidth:true;spacing:2
        Label {text:textureControl.title+(textureControl.unit.length?" ("+textureControl.unit+")":"");color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
        CompactSpinBox {objectName:"settingsTexture"+textureControl.field+"Input";Layout.fillWidth:true;Layout.minimumWidth:0;implicitWidth:110;implicitHeight:20;editable:true;from:textureControl.from;to:textureControl.to;value:Math.round(root.library.texture[textureControl.field]*textureControl.factor);onValueModified:root.setTextureField(textureControl.field,value/textureControl.factor);tooltip:textureControl.help}
        CompactSlider {objectName:"settingsTexture"+textureControl.field;Layout.fillWidth:true;from:textureControl.from;to:textureControl.to;value:root.library.texture[textureControl.field]*textureControl.factor;onMoved:root.setTextureField(textureControl.field,value/textureControl.factor);tooltip:textureControl.help}
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
            CompactSpinBox {objectName:control.sliderName+"Input";implicitWidth:66;implicitHeight:20;editable:true;from:control.from;to:control.to;value:Math.round(root.library[control.field]*control.factor);onValueModified:root.library[control.field]=value/control.factor}
            Label {text:control.unit;color:Theme.muted;font.pixelSize:9}
        }
        CompactSlider {objectName:control.sliderName;Layout.fillWidth:true;from:control.from;to:control.to;value:root.library[control.field]*control.factor;onMoved:root.library[control.field]=value/control.factor;tooltip:control.help}
    }
    Timer {id:previewDelay;interval:100;onTriggered:root.library.requestPreview(root.library.selectedId)}
    FileDialog {id:tipDialog;title:"导入图片笔尖";nameFilters:["PNG 图片 (*.png)"];onAccepted:PaintClient.importBrushTip(selectedFile,tipInterpretation.currentIndex)}
    FileDialog {id:textureDialog;title:"导入纹理";nameFilters:["PNG 图片 (*.png)"];onAccepted:PaintClient.importBrushTexture(selectedFile,textureInterpretation.currentIndex)}
    FileDialog {id:dualDialog;title:"导入第二笔尖";nameFilters:["PNG 图片 (*.png)"];onAccepted:PaintClient.importDualBrushTip(selectedFile,dualInterpretation.currentIndex)}
    ColorDialog {id:outerColor;title:"外侧轮廓颜色";selectedColor:root.library.cursorColor;onAccepted:root.library.cursorColor=selectedColor}
    ColorDialog {id:innerColor;title:"内侧轮廓颜色";selectedColor:root.library.cursorSecondaryColor;onAccepted:root.library.cursorSecondaryColor=selectedColor}
    Component.onCompleted:previewDelay.restart()
    Connections {target:root.library;function onSettingsChanged(){previewDelay.restart();if(root.category===8 && root.inspectedSource!==root.library.sourceResource)root.inspectSource()}}
}
