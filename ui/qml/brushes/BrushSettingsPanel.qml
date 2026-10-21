import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import "../controls"
import "../theme"

ColumnLayout {
    id:root
    objectName:"brushSettingsPanel"
    property int tipInterpretationMode:2
    property int textureInterpretationMode:0
    property int dualInterpretationMode:2
    property string wetError:""
    function setWetField(key,value){let data=Object.assign({},root.library.wetMix);data[key]=value;const ok=root.library.setWetMix(data);root.wetError=ok?"":"湿混参数无效，未应用更改。";return ok}
    function scatterControl(field){return root.library.scatterDynamics[field]||{source:"off",minimum:0}}
    function setScatterControl(field,source){let data=Object.assign({},root.library.scatterDynamics);if(source==="off")delete data[field];else{let old=data[field]||{};data[field]={source:source,minimum:old.minimum||0};if(source==="fade")data[field].fadeSteps=old.fadeSteps||25}return root.library.setScatterDynamics(data)}
    function setScatterControlField(field,key,value){let data=Object.assign({},root.library.scatterDynamics);let control=Object.assign({},data[field]||{source:"off",minimum:0});control[key]=value;data[field]=control;return root.library.setScatterDynamics(data)}
    readonly property var library:PaintClient.brushLibrary
    property int category:0
    property int shapePage:0
    property int dualPage:0
    property string taperEnd:"start"
    readonly property bool taperRelative:library.strokeTaper[taperEnd].lengthMode==="relative"
    function setTaperLengthMode(relative){let data=Object.assign({},library.strokeTaper);let end=Object.assign({},data[taperEnd]);if(end.lengthPercent===undefined)end.lengthPercent=Math.min(10000,end.lengthPx/(library.radius*2)*100);end.lengthMode=relative?"relative":"independent";data[taperEnd]=end;return library.setStrokeTaper(data)}
    function setTaperTransparent(transparent){let opacity=Object.assign({},library.strokeTaper[taperEnd].opacity);opacity.minimum=transparent?0:1;return setTaperEndField("opacity",opacity)}
    function setTaperEndField(key,value){let data=Object.assign({},library.strokeTaper);let end=Object.assign({},data[taperEnd]);end[key]=value;data[taperEnd]=end;return library.setStrokeTaper(data)}
    readonly property string outlinePresetId:library.selectedId
    onOutlinePresetIdChanged:choosingOutlineTexture=false
    property bool choosingOutlineTexture:false
    readonly property var outline:library.strokeOutline
    function setOutlineField(key,value){let data=Object.assign({},outline);data[key]=value;return library.setStrokeOutline(data)}
    function setOutlineWidth(key,value){let width=Object.assign({},outline.width);width[key]=value;return setOutlineField("width",width)}
    function setOutlineBrush(key,value){let source=Object.assign({},outline.source);let brush=Object.assign({},source.brush);brush[key]=value;source.brush=brush;return setOutlineField("source",source)}
    function setOutlineScatter(key,value){let data=Object.assign({},outline.source.brush.scatterDynamics||{enabled:false,amount:0,bothAxes:false,count:1,countJitter:0,seed:"0000000000000000"});data[key]=value;return setOutlineBrush("scatterDynamics",data)}
    function setOutlineTexture(key,value){let source=Object.assign({},outline.source);let texture=Object.assign({},source.texture);texture[key]=value;source.texture=texture;return setOutlineField("source",source)}
    QtObject {
        id:taperControls
        readonly property var pressureDynamics: {
            let end=root.library.strokeTaper[root.taperEnd]
            let result={}
            for(let key of ["size","opacity"]){result[key]=Object.assign({},end[key]);result[key].interpolation=end[key].interpolation==="pchip"?"smooth-pchip-v1":"linear"}
            return result
        }
        signal settingsChanged()
        onPressureDynamicsChanged:settingsChanged()
        function setPressureControl(target,points,minimum,modes){let c={curve:points,minimum:minimum,interpolation:"pchip"};if(modes && modes.length)c.nodeModes=modes;return root.setTaperEndField(target,c)}
        function resetPressureControl(target){return root.setTaperEndField(target,{curve:[[0,0],[1,1]],minimum:0,interpolation:"linear"})}
    }
    readonly property var dual:library.dualBrush
    function dualModule(name) {
        if(dual[name])return dual[name]
        if(name==="sizing")return {model:"relative-size-v1",mode:"independent",diameterOffset:0,widthRule:"follow-pressure"}
        if(name==="tipDynamics")return {angleControl:"fixed",tiltRoundness:false,minimumRoundness:0.01}
        if(name==="scatterDynamics")return {enabled:false,amount:0,bothAxes:false,count:1,countJitter:0,seed:"0000000000000000"}
        return {enabled:false,size:0,minimumSize:0,angle:0,roundness:0,minimumRoundness:0.01,seed:"0000000000000000"}
    }
    function dualNominalDiameter() {
        const size=dualModule("sizing")
        return size.mode!=="relative"?dual.diameter:Math.max(1,Math.min(2000,size.model==="relative-ratio-v1"?library.radius*2*size.diameterRatio:library.radius*2+size.diameterOffset))
    }
    function setDualSizeMode(index) {
        let data=Object.assign({},dual),old=dualModule("sizing"),size=Object.assign({},old)
        if(index===1)size={model:"relative-ratio-v1",mode:"relative",diameterRatio:old.model==="relative-ratio-v1"?old.diameterRatio:dualNominalDiameter()/(library.radius*2),widthRule:old.widthRule}
        else if(index===2)size={model:"relative-size-v1",mode:"relative",diameterOffset:old.model==="relative-size-v1"?old.diameterOffset:Math.max(-511,Math.min(511,dualNominalDiameter()-library.radius*2)),widthRule:old.widthRule}
        else size.mode="independent"
        data.sizing=size;return library.setDualBrush(data)
    }
    function setDualField(key,value,module) {
        let data=Object.assign({},dual)
        if(key==="enabled" && value && !data.sizing)data.sizing={model:"relative-ratio-v1",mode:"relative",diameterRatio:data.diameter/(library.radius*2),widthRule:"follow-pressure"}
        if(module){let nested=Object.assign({},dualModule(module));nested[key]=value;data[module]=nested}
        else data[key]=value
        const ok=library.setDualBrush(data);root.wetError=ok?"":"第二笔刷参数无效，未应用更改。";return ok
    }
    function setTransferJitterField(key,value){let data=Object.assign({},library.transferJitter);data[key]=value/100;return library.setTransferJitter(data)}
    function dualScatterControl(field){return root.dualModule("scatterDynamics")[field]||{source:"off",minimum:0}}
    function setDualScatterControl(field,source){let data=Object.assign({},root.dual);let scatter=Object.assign({},root.dualModule("scatterDynamics"));if(source==="off")delete scatter[field];else{let old=scatter[field]||{};scatter[field]={source:source,minimum:old.minimum||0};if(source==="fade")scatter[field].fadeSteps=old.fadeSteps||25}data["scatterDynamics"]=scatter;return root.library.setDualBrush(data)}
    function setDualScatterControlField(field,key,value){let data=Object.assign({},root.dual);let scatter=Object.assign({},root.dualModule("scatterDynamics"));let control=Object.assign({},scatter[field]||{source:"off",minimum:0});control[key]=value;scatter[field]=control;data["scatterDynamics"]=scatter;return root.library.setDualBrush(data)}
    QtObject {
        id:dualPressure
        readonly property var pressureDynamics:root.dual.pressureDynamics || {size:{curve:[[0,0],[1,1]],minimum:0},flow:{curve:[[0,0],[1,1]],minimum:0}}
        signal settingsChanged()
        function setPressureControl(target,points,minimum,modes){return root.library.setDualPressureControl(target,points,minimum,modes || [])}
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
    function setAirbrushField(key,value) {let data=Object.assign({},root.library.airbrush);data[key]=value;root.library.setAirbrush(data)}
    function setSpeedDynamicsEnabled(enabled) {return root.library.setSpeedDynamicsEnabled(enabled)}
    function setSpeedDynamicsMax(value) {return root.library.setSpeedDynamicsMaxSpeed(value)}
    function setSpeedControl(target,points,minimum,modes){return root.library.setSpeedControl(target,points,minimum,modes || [])}
    function resetSpeedControl(target){return root.library.resetSpeedControl(target)}
    function setTangentDynamicsEnabled(enabled){return root.library.setTangentDynamicsEnabled(enabled)}
    function setTangentControl(target,points,minimum,modes){return root.library.setTangentControl(target,points,minimum,modes || [])}
    function resetTangentControl(target){return root.library.resetTangentControl(target)}
    function setDistanceDynamicsEnabled(enabled){return root.library.setDistanceDynamicsEnabled(enabled)}
    function setDistanceDynamicsMax(value){return root.library.setDistanceDynamicsMaxDistance(value)}
    function setDistanceControl(target,points,minimum,modes){return root.library.setDistanceControl(target,points,minimum,modes || [])}
    function resetDistanceControl(target){return root.library.resetDistanceControl(target)}
    function setTiltDynamicsEnabled(enabled){return root.library.setTiltDynamicsEnabled(enabled)}
    function setTiltControl(target,points,minimum,modes){return root.library.setTiltControl(target,points,minimum,modes || [])}
    function resetTiltControl(target){return root.library.resetTiltControl(target)}
    function setRotationDynamicsEnabled(enabled){return root.library.setRotationDynamicsEnabled(enabled)}
    function setRotationControl(target,points,minimum,modes){return root.library.setRotationControl(target,points,minimum,modes || [])}
    function resetRotationControl(target){return root.library.resetRotationControl(target)}
    function setDirectionDynamicsEnabled(enabled){return root.library.setDirectionDynamicsEnabled(enabled)}
    function setDirectionControl(target,points,minimum,modes){return root.library.setDirectionControl(target,points,minimum,modes || [])}
    function resetDirectionControl(target){return root.library.resetDirectionControl(target)}
    function setFadeDynamicsEnabled(enabled){return root.library.setFadeDynamicsEnabled(enabled)}
    function setFadeSteps(value){return root.library.setFadeSteps(value)}
    function setFadeControl(target,points,minimum,modes){return root.library.setFadeControl(target,points,minimum,modes || [])}
    function resetFadeControl(target){return root.library.resetFadeControl(target)}
    readonly property var dynamicsSources:["all","speed","tangent","distance","tilt","rotation","direction"]
    function dynamicsSourceIndex(value){const index=root.dynamicsSources.indexOf(value);return index<0?0:index}
    function setDynamicsSource(target,index){return root.library.setDynamicsSource(target,root.dynamicsSources[index])}
    onCategoryChanged:if(category===8)inspectSource()
    readonly property real contentHeight:360
    implicitHeight:contentHeight
    spacing:6
    Label {objectName:"selectedBrushName";textFormat:Text.PlainText;text:root.library.selectedName;color:Theme.text;font.pixelSize:10;elide:Text.ElideRight;Layout.fillWidth:true}
    Label {visible:root.wetError.length>0;text:root.wetError;Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
    RowLayout {
        Layout.fillWidth:true;Layout.fillHeight:true;spacing:6
        ColumnLayout {
            Layout.preferredWidth:76;Layout.minimumWidth:76;Layout.maximumWidth:76;Layout.alignment:Qt.AlignTop;spacing:2
            Repeater {
                model:["笔尖形状","形状动态","散布","纹理","双重笔刷","颜色动态","传递","光标与预览","来源与兼容","平滑","湿混与载色","起收笔","整笔描边"]
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
            implicitWidth:0
            Layout.fillWidth:true;Layout.fillHeight:true;Layout.minimumWidth:0;clip:true
            contentWidth:root.category===11?Math.max(200,availableWidth):availableWidth
            ColumnLayout {
                width:parameterScroll.contentWidth;spacing:5
                Loader {objectName:"brushSettingsCategory:0";Layout.fillWidth:true;Layout.minimumWidth:0;active:root.category===0;visible:active
                    sourceComponent:ColumnLayout {
                    visible:root.category===0;Layout.fillWidth:true;spacing:5
                    BrushResourcePicker {objectName:"settingsTipResourcePicker";Layout.fillWidth:true;resource:root.library.maskResource;resourceRole:0;importName:"importImageBrushTip";resetName:"proceduralBrushTip";onImportRequested:tipDialog.open();onResetRequested:root.library.useProceduralTip()}
                    Label {objectName:"imageTipInterpretationLabel";text:"图片覆盖方式";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                    CompactComboBox {id:tipInterpretation;objectName:"imageTipInterpretation";Layout.fillWidth:true;model:["灰度覆盖（白色绘制）","透明度覆盖","反向亮度（黑色绘制）"];currentIndex:root.tipInterpretationMode;onActivated:root.tipInterpretationMode=currentIndex;tooltip:"灰度覆盖以白色绘制；透明度使用图片 alpha；反向亮度以黑色绘制。"}
                    Label {visible:root.library.maskResource.length>0;text:"图片覆盖 · 硬度来自图片";color:Theme.muted;font.pixelSize:9;Layout.fillWidth:true;wrapMode:Text.WordWrap}
                    Label {visible:PaintClient.brushImportMessage.length>0;text:PaintClient.brushImportMessage;color:Theme.muted;font.pixelSize:9;Layout.fillWidth:true;wrapMode:Text.WordWrap}
                    RowLayout {
                        Layout.fillWidth:true
                        Label {text:"大小";font.pixelSize:10;color:Theme.text;Layout.fillWidth:true}
                        CompactSpinBox {objectName:"settingsBrushSize";implicitWidth:68;implicitHeight:20;editable:true;from:1;to:2000;value:Math.round(root.library.radius*2);onValueModified:root.library.radius=value/2}
                        Label {text:"px";font.pixelSize:9;color:Theme.muted}
                    }
                    CompactSlider {Layout.fillWidth:true;from:1;to:2000;value:root.library.radius*2;stepSize:1;snapMode:Slider.SnapAlways;onMoved:root.library.radius=Math.round(value)/2}
                    TipField {enabled:!root.library.maskResource.length;title:"硬度";field:"hardness";from:0;to:100;factor:100;unit:"%";help:"实心中心占笔尖半径的比例；降低后形成柔软边缘。"}
                    BrushTipShapeControl {
                        objectName:"settingsBrushTipShape"
                        angle:root.library.angle; roundness:root.library.roundness
                        onAngleEdited:value=>root.library.angle=value
                        onRoundnessEdited:value=>root.library.roundness=value
                    }
                    TipField {title:"间距";field:"spacing";from:1;to:50;factor:50;unit:"%";help:"相邻印章的距离，以直径百分比表示，最低 1%。";sliderName:"settingsBrushSpacing"}
                }
                }
                Loader {objectName:"brushSettingsCategory:1";Layout.fillWidth:true;Layout.minimumWidth:0;active:root.category===1;visible:active
                    sourceComponent:ColumnLayout {
                    visible:root.category===1;Layout.fillWidth:true;spacing:5
                    Label {objectName:"settingsShapePageLabel";text:"形状动态页面";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                    CompactComboBox {objectName:"settingsShapePage";Layout.fillWidth:true;model:["设备与压感","随机抖动"];currentIndex:root.shapePage;onActivated:root.shapePage=currentIndex;tooltip:"选择形状动态设置；两个页面的参数可同时作用。"}
                    ColumnLayout {visible:root.shapePage===0;Layout.fillWidth:true;spacing:5
                    ColumnLayout {
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
                }
                Loader {objectName:"brushSettingsCategory:2";Layout.fillWidth:true;Layout.minimumWidth:0;active:root.category===2;visible:active
                    sourceComponent:ColumnLayout {
                    visible:root.category===2;Layout.fillWidth:true;spacing:5
                    CompactCheckBox {objectName:"settingsScatterEnabled";text:"启用散布";checked:root.library.scatterEnabled;font.pixelSize:10;onToggled:root.library.scatterEnabled=checked;tooltip:"在路径周围分散笔尖印章；关闭时保留参数。"}
                    ColumnLayout {
                        Layout.fillWidth:true;spacing:5
                        TipField {title:"散布";field:"scatterAmount";from:0;to:1000;factor:100;unit:"%";help:"相对名义直径的最大偏移；单轴沿路径垂直方向，双轴沿画布两个方向。";sliderName:"settingsScatterAmount"}
                        CompactCheckBox {objectName:"settingsScatterBothAxes";text:"双轴";checked:root.library.scatterBothAxes;font.pixelSize:10;onToggled:root.library.scatterBothAxes=checked;tooltip:"在画布水平和垂直方向分别随机偏移。"}
                        TipField {title:"数量";field:"scatterCount";from:1;to:16;factor:1;help:"每个路径印章产生的最大印章数量；增加数量会增加绘画工作量。";sliderName:"settingsScatterCount"}
                        TipField {title:"数量抖动";field:"scatterCountJitter";from:0;to:100;factor:100;unit:"%";help:"在最大数量和降低后的数量之间随机变化，每次至少一个印章。";sliderName:"settingsScatterCountJitter"}
                        Label {text:"幅度控制";color:Theme.muted;font.pixelSize:9}
                        CompactComboBox {objectName:"settingsScatterAmountControl";Layout.fillWidth:true;readonly property var modes:["off","fade","pressure","tilt","direction","rotation"];model:["无","渐隐","笔压","倾斜","方向","笔旋转"];currentIndex:Math.max(0,modes.indexOf(root.scatterControl("amountControl").source));onActivated:root.setScatterControl("amountControl",modes[currentIndex]);tooltip:"控制散布幅度；渐隐按印章步数下降，其他来源使用设备输入。"}
                        CompactSpinBox {objectName:"settingsScatterAmountMinimum";visible:root.scatterControl("amountControl").source!=="off";Layout.fillWidth:true;from:0;to:100;value:Math.round(root.scatterControl("amountControl").minimum*100);onValueModified:root.setScatterControlField("amountControl","minimum",value/100);tooltip:"控制输入为零时的幅度最低比例"}
                        CompactSpinBox {objectName:"settingsScatterAmountFade";visible:root.scatterControl("amountControl").source==="fade";Layout.fillWidth:true;from:1;to:1000;value:root.scatterControl("amountControl").fadeSteps||25;onValueModified:root.setScatterControlField("amountControl","fadeSteps",value);tooltip:"渐隐达到最低比例所用的印章步数"}
                        Label {text:"数量控制";color:Theme.muted;font.pixelSize:9}
                        CompactComboBox {objectName:"settingsScatterCountControl";Layout.fillWidth:true;readonly property var modes:["off","fade","pressure","tilt","direction","rotation"];model:["无","渐隐","笔压","倾斜","方向","笔旋转"];currentIndex:Math.max(0,modes.indexOf(root.scatterControl("countControl").source));onActivated:root.setScatterControl("countControl",modes[currentIndex]);tooltip:"控制每个路径印章的子印章数量；最低比例不会低于一个印章。"}
                        CompactSpinBox {objectName:"settingsScatterCountMinimum";visible:root.scatterControl("countControl").source!=="off";Layout.fillWidth:true;from:0;to:100;value:Math.round(root.scatterControl("countControl").minimum*100);onValueModified:root.setScatterControlField("countControl","minimum",value/100);tooltip:"控制输入为零时的数量最低比例"}
                        CompactSpinBox {objectName:"settingsScatterCountFade";visible:root.scatterControl("countControl").source==="fade";Layout.fillWidth:true;from:1;to:1000;value:root.scatterControl("countControl").fadeSteps||25;onValueModified:root.setScatterControlField("countControl","fadeSteps",value);tooltip:"渐隐达到最低数量比例所用的印章步数"}
                    }
                }
                }
                Loader {objectName:"brushSettingsCategory:3";Layout.fillWidth:true;Layout.minimumWidth:0;active:root.category===3;visible:active
                    sourceComponent:ColumnLayout {Layout.minimumWidth:0;visible:root.category===3;Layout.fillWidth:true;spacing:5
                    BrushResourcePicker {objectName:"settingsTextureResourcePicker";Layout.fillWidth:true;resource:root.library.texture.resource||"";resourceRole:1;importName:"importBrushTexture";resetName:"removeBrushTexture";onImportRequested:textureDialog.open();onResetRequested:root.library.removeTexture()}
                    CompactCheckBox {objectName:"settingsTextureEnabled";text:"启用纹理";enabled:!!root.library.texture.resource;checked:root.library.texture.enabled;font.pixelSize:10;onToggled:root.setTextureField("enabled",checked);tooltip:"使用图片覆盖调制笔触；关闭保留纹理和参数。"}
                    Label {objectName:"textureInterpretationLabel";text:"图片覆盖方式";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                    CompactComboBox {id:textureInterpretation;objectName:"textureInterpretation";Layout.fillWidth:true;model:["灰度覆盖（白色绘制）","透明度覆盖","反向亮度（黑色绘制）"];currentIndex:root.textureInterpretationMode;onActivated:root.textureInterpretationMode=currentIndex;tooltip:"将图片转换为可重复的灰度覆盖，纹理与笔尖独立。"}
                    Label {objectName:"settingsTextureModeLabel";text:"纹理混合模式";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                    CompactComboBox {objectName:"settingsTextureMode";Layout.fillWidth:true;model:["正片叠底","减去","变暗","叠加","颜色减淡","颜色加深","线性加深","实色混合","线性高度","高度"];property var modes:["multiply","subtract","darken","overlay","colorDodge","colorBurn","linearBurn","hardMix","linearHeight","height"];currentIndex:Math.max(0,modes.indexOf(root.library.texture.mode));onActivated:root.setTextureField("mode",modes[currentIndex]);tooltip:"纹理灰度与笔尖覆盖的组合方式。"}
                    Label {text:root.library.texture.resource?"已加载纹理；关闭时仍可编辑。":"先导入 PNG 纹理，再启用。";Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                    Label {visible:PaintClient.brushImportMessage.length>0;text:PaintClient.brushImportMessage;color:Theme.muted;font.pixelSize:9;Layout.fillWidth:true;wrapMode:Text.WordWrap}
                    Label {objectName:"settingsTextureSpaceLabel";text:"纹理坐标";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                    CompactComboBox {objectName:"settingsTextureSpace";Layout.fillWidth:true;model:["文档纸纹","笔尖局部"];currentIndex:root.library.texture.space==="tip"?1:0;onActivated:root.setTextureField("space",currentIndex===1?"tip":"document");tooltip:"纸纹固定在文档坐标；局部纹理随每个印章的位置和角度变化。"}
                    TextureField {title:"比例";field:"scale";from:1;to:10000;factor:100;unit:"%";help:"源图片一个像素对应的文档像素比例，100%为原始大小。"}
                    TextureField {title:"角度";field:"angle";from:-180;to:180;factor:1;unit:"°";help:"旋转纹理采样方向。"}
                    TextureField {title:"深度";field:"depth";from:0;to:100;factor:100;unit:"%";help:"纹理覆盖影响强度；纸纹在同一笔触反复擦拭时保持覆盖。"}
                    TextureField {title:"亮度";field:"brightness";from:-100;to:100;factor:100;unit:"%";help:"调整纹理采样的亮度；需要 state21/VBR1.26。"}
                    TextureField {title:"对比度";field:"contrast";from:-100;to:100;factor:100;unit:"%";help:"围绕中间灰调整纹理对比度；需要 state21/VBR1.26。"}
                    TextureField {title:"最低深度";field:"minimumDepth";from:0;to:100;factor:100;unit:"%";help:"纹理深度动态允许下降到的最低值；需要 state21/VBR1.27。"}
                    TextureField {title:"深度抖动";field:"depthJitter";from:0;to:100;factor:100;unit:"%";help:"按印章随机降低纹理深度；需要 state21/VBR1.27。"}
                    CompactCheckBox {objectName:"settingsTextureDepthPressure";text:"深度压感";checked:root.library.texture.depthPressure===true;font.pixelSize:10;onToggled:root.setTextureField("depthPressure",checked);tooltip:"用笔压驱动纹理深度，并以最低深度为下限。"}
                    CompactCheckBox {objectName:"settingsTextureInvert";text:"反相";checked:root.library.texture.invert;font.pixelSize:10;onToggled:root.setTextureField("invert",checked);tooltip:"交换纹理黑白覆盖。"}
                    TextureField {title:"水平偏移";field:"offsetX";from:-1000000;to:1000000;factor:1;unit:"px";help:"纹理相位在文档水平方向的偏移。"}
                    TextureField {title:"垂直偏移";field:"offsetY";from:-1000000;to:1000000;factor:1;unit:"px";help:"纹理相位在文档垂直方向的偏移。"}
                }
                }
                Loader {objectName:"brushSettingsCategory:4";Layout.fillWidth:true;Layout.minimumWidth:0;active:root.category===4;visible:active
                    sourceComponent:ColumnLayout {Layout.minimumWidth:0;visible:root.category===4;Layout.fillWidth:true;spacing:5
                    Label {text:"第二笔刷来源";color:Theme.text;font.pixelSize:10}
                    CompactComboBox {objectName:"settingsDualSourceMode";Layout.fillWidth:true;model:["自定义","现有"];currentIndex:root.library.dualExisting?1:0;onActivated:root.library.dualExisting=currentIndex===1}
Label {objectName:"settingsDualModeLabel";text:"覆盖组合模式";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
CompactComboBox {objectName:"settingsDualMode";Layout.fillWidth:true;readonly property var modes:["multiply","darken","overlay","colorDodge","colorBurn","linearBurn","hardMix","linearHeight"];model:["正片叠底","变暗","叠加","颜色减淡","颜色加深","线性加深","实色混合","线性高度"];currentIndex:Math.max(0,modes.indexOf(root.dual.mode));onActivated:root.setDualField("mode",modes[currentIndex]);tooltip:"组合两个笔尖的灰度覆盖。"}
                    CompactCheckBox {objectName:"settingsDualEnabled";text:"启用双重笔刷";checked:root.dual.enabled;font.pixelSize:10;onToggled:if(!root.setDualField("enabled",checked))checked=Qt.binding(function(){return root.dual.enabled});tooltip:"第二笔尖调制主笔尖覆盖；关闭时保留全部设置。"}
                    BrushResourcePicker {objectName:"settingsDualResourcePicker";visible:!root.library.dualExisting;Layout.fillWidth:true;resource:root.dual.resource||"";resourceRole:2;importName:"importDualBrushTip";resetName:"proceduralDualBrushTip";onImportRequested:dualDialog.open();onResetRequested:root.library.useProceduralDual()}
                    ColumnLayout {visible:root.library.dualExisting;Layout.fillWidth:true;spacing:5
                        CompactComboBox {
                            id:existingDualPreset;objectName:"settingsExistingDualPreset";Layout.fillWidth:true
                            model:root.library.presetRows();textRole:"name";currentIndex:-1;displayText:root.library.dualSourceName||"选择现有笔刷"
                            onActivated:{if(currentIndex>=0)root.library.usePresetAsSecondary(model[currentIndex].id);currentIndex=-1}
                            tooltip:"复制主笔尖形状、压力、散布和纹理；来源后续修改不影响当前快照。"
                            Connections {target:root.library;function onPresetsChanged(){existingDualPreset.model=root.library.presetRows()}}
                        }
                        Label {text:root.library.dualSourceName?"已复制笔尖和纹理，可调整下列设置。":"选择一个笔刷作为第二笔尖。";color:Theme.muted;font.pixelSize:9;wrapMode:Text.WordWrap;Layout.fillWidth:true}
                        ColumnLayout {visible:root.library.dualSourceName.length>0;Layout.fillWidth:true;spacing:5
                            DualField {prefix:"settingsExistingDual";decimals:0;title:"大小";field:"diameter";from:1;to:2000;factor:1;unit:"px"}
                            DualField {prefix:"settingsExistingDual";decimals:0;title:"间距";field:"spacing";from:1;to:1000}
                            CompactCheckBox {objectName:"settingsExistingDualScatterEnabled";text:"启用散布";checked:root.dualModule("scatterDynamics").enabled;onToggled:root.setDualField("enabled",checked,"scatterDynamics")}
                            DualField {prefix:"settingsExistingDual";decimals:0;title:"散布";field:"amount";module:"scatterDynamics";to:1000}
                            CompactCheckBox {objectName:"settingsExistingDualBothAxes";text:"双轴";checked:root.dualModule("scatterDynamics").bothAxes;onToggled:root.setDualField("bothAxes",checked,"scatterDynamics")}
                            DualField {prefix:"settingsExistingDual";title:"数量";field:"count";module:"scatterDynamics";from:1;to:16;factor:1;unit:""}
                        }
                    }
                    Label {objectName:"settingsDualPageLabel";visible:!root.library.dualExisting;text:"第二笔刷设置页面";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                    CompactComboBox {objectName:"settingsDualPage";visible:!root.library.dualExisting;Layout.fillWidth:true;model:["笔尖与组合","设备与压力","散布","形状抖动"];currentIndex:Math.min(root.dualPage,3);onActivated:root.dualPage=currentIndex;tooltip:"编辑第二笔尖的形状、压力与随机动态。"}
                    ColumnLayout {Layout.minimumWidth:0;visible:!root.library.dualExisting && root.dualPage===0;Layout.fillWidth:true;spacing:5
                        CompactCheckBox {objectName:"settingsDualFlip";text:"随机翻转第二笔刷";checked:root.dual.flip===true;onToggled:root.setDualField("flip",checked);tooltip:"每个第二笔刷印章随机翻转形状；用于图片或非对称笔尖。"}
                        CompactCheckBox {objectName:"settingsDualFlipX";text:"固定水平镜像";checked:root.dual.flipX===true;onToggled:root.setDualField("flipX",checked);tooltip:"固定镜像第二笔刷形状的 X 轴；可与随机翻转同时使用。"}
                        CompactCheckBox {objectName:"settingsDualFlipY";text:"固定垂直镜像";checked:root.dual.flipY===true;onToggled:root.setDualField("flipY",checked);tooltip:"固定镜像第二笔刷形状的 Y 轴；可与随机翻转同时使用。"}
                        Label {objectName:"settingsDualModelLabel";text:"第二笔尖间距方式";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                        CompactComboBox {objectName:"settingsDualModel";Layout.fillWidth:true;model:["独立路径间距","每个主印章配对"];currentIndex:root.dual.model==="paired-dab-v1"?1:0;onActivated:{let data=Object.assign({},root.dual);data.model=currentIndex===1?"paired-dab-v1":"path-coverage-v1";data.spacing=currentIndex===1?null:0.25;root.library.setDualBrush(data)} tooltip:"独立路径使用第二笔尖自己的间距；配对在每个主印章的位置生成第二笔尖。"}
                        Label {objectName:"dualTipInterpretationLabel";text:"图片覆盖方式";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                        CompactComboBox {id:dualInterpretation;objectName:"dualTipInterpretation";Layout.fillWidth:true;model:["灰度覆盖（白色绘制）","透明度覆盖","反向亮度（黑色绘制）"];currentIndex:root.dualInterpretationMode;onActivated:root.dualInterpretationMode=currentIndex;tooltip:"选择导入图片的覆盖解释。"}
                        Label {visible:PaintClient.brushImportMessage.length>0;text:PaintClient.brushImportMessage;Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                        Label {objectName:"settingsDualSizeModeLabel";text:"尺寸方式";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                        CompactComboBox {objectName:"settingsDualSizeMode";Layout.fillWidth:true;model:["独立尺寸","相对主笔百分比","旧版像素偏移"];currentIndex:root.dualModule("sizing").mode!=="relative"?0:root.dualModule("sizing").model==="relative-ratio-v1"?1:2;onActivated:root.setDualSizeMode(currentIndex);tooltip:"百分比随主笔大小缩放；旧版像素偏移保持原文件语义。"}
                        DualField {title:"大小";field:"diameter";from:1;to:2000;factor:1;unit:"px";visible:root.dualModule("sizing").mode!=="relative"}
                        DualField {title:"直径偏移";field:"diameterOffset";module:"sizing";from:-511;to:511;factor:1;unit:"px";visible:root.dualModule("sizing").mode==="relative" && root.dualModule("sizing").model==="relative-size-v1"}
                        ParameterSlider {title:"相对大小";sliderName:"settingsDualRatio";visible:root.dualModule("sizing").model==="relative-ratio-v1" && root.dualModule("sizing").mode==="relative";from:1;to:200000;sliderTo:400;decimals:0;unit:"%";value:root.dualModule("sizing").diameterRatio*100;onCommitted:value=>root.setDualField("diameterRatio",value/100,"sizing");tooltip:"主笔大小的百分比；输入框可输入超出滑条范围的比例。"}
                        Label {objectName:"settingsDualWidthRuleLabel";text:"相对尺寸响应";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true;visible:root.dualModule("sizing").mode==="relative"}
                        CompactComboBox {objectName:"settingsDualWidthRule";Layout.fillWidth:true;visible:root.dualModule("sizing").mode==="relative";model:["跟随尺寸压感","固定描边宽度"];currentIndex:root.dualModule("sizing").widthRule==="fixed-width"?1:0;onActivated:root.setDualField("widthRule",currentIndex===1?"fixed-width":"follow-pressure","sizing");tooltip:"固定宽度先计算主笔实际尺寸再加偏移，不重复执行第二大小压感和大小抖动；旧配置仍保留。"}
                        Label {objectName:"settingsDualEffectiveSize";text:"标称直径："+root.dualNominalDiameter().toFixed(1)+" px";Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}

                        DualField {title:"间距";field:"spacing";from:1;to:100;factor:100;unit:"%";visible:root.dual.model==="path-coverage-v1"}
                        DualField {title:"硬度";field:"hardness";enabled:!root.dual.resource}
                        BrushTipShapeControl {
                            objectName:"settingsDualTipShape"; namePrefix:"settingsDual"
                            angle:root.dual.angle; roundness:root.dual.roundness
                            onAngleEdited:value=>root.setDualField("angle",value)
                            onRoundnessEdited:value=>root.setDualField("roundness",value)
                        }
                        DualField {title:"流量";field:"flow"}
                        Label {text:"第二笔尖改变实际覆盖；可在光标与预览中开启复合光标。";Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                    }
                    ColumnLayout {Layout.minimumWidth:0;visible:!root.library.dualExisting && root.dualPage===1;Layout.fillWidth:true;spacing:5
                        Label {objectName:"settingsDualAngleControlLabel";text:"角度控制";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                        CompactComboBox {objectName:"settingsDualAngleControl";Layout.fillWidth:true;readonly property var controls:["fixed","direction","pen-rotation","tilt"];model:["固定角度","路径方向","笔旋转","笔倾斜"];currentIndex:controls.indexOf(root.dualModule("tipDynamics").angleControl);onActivated:root.setDualField("angleControl",controls[currentIndex],"tipDynamics");tooltip:"第二笔尖独立叠加方向、旋转或倾斜角度；缺失设备能力使用基础角度。"}
                        CompactCheckBox {objectName:"settingsDualTiltRoundness";text:"倾斜控制圆度";checked:root.dualModule("tipDynamics").tiltRoundness;onToggled:root.setDualField("tiltRoundness",checked,"tipDynamics");tooltip:"按倾斜缩短第二笔尖短轴；关闭后保留最低比例。"}
                        DualField {title:"最低圆度比例";field:"minimumRoundness";module:"tipDynamics";from:1}
                        CompactCheckBox {objectName:"settingsDualPressureSize";text:"大小压感";checked:root.dual.pressureSize;onToggled:root.setDualField("pressureSize",checked);tooltip:"压力曲线控制第二笔尖；关闭时保留曲线和最低比例。"}
                        PressureCurveEditor {library:dualPressure;target:"size";prefix:"dualPressureSize"}
                        CompactCheckBox {objectName:"settingsDualPressureFlow";text:"流量压感";checked:root.dual.pressureFlow;onToggled:root.setDualField("pressureFlow",checked);tooltip:"压力曲线控制第二笔尖；关闭时保留曲线和最低比例。"}
                        PressureCurveEditor {library:dualPressure;target:"flow";prefix:"dualPressureFlow"}
                    }
                    ColumnLayout {Layout.minimumWidth:0;visible:!root.library.dualExisting && root.dualPage===2;Layout.fillWidth:true;spacing:5
                        CompactCheckBox {objectName:"settingsDualScatterEnabled";text:"启用散布";checked:root.dualModule("scatterDynamics").enabled;onToggled:root.setDualField("enabled",checked,"scatterDynamics");tooltip:"散布第二笔尖；关闭后保留参数。"}
                        DualField {title:"散布";field:"amount";module:"scatterDynamics";to:1000}
                        CompactCheckBox {objectName:"settingsDualScatterBothAxes";text:"双轴";checked:root.dualModule("scatterDynamics").bothAxes;onToggled:root.setDualField("bothAxes",checked,"scatterDynamics");tooltip:"沿画布两个方向分别随机偏移。"}
                        DualField {title:"数量";field:"count";module:"scatterDynamics";from:1;to:16;factor:1;unit:""}
                        DualField {title:"数量抖动";field:"countJitter";module:"scatterDynamics"}
                        Label {text:"幅度控制";color:Theme.muted;font.pixelSize:9}
                        CompactComboBox {objectName:"settingsDualScatterAmountControl";Layout.fillWidth:true;readonly property var modes:["off","fade","pressure","tilt","direction","rotation"];model:["无","渐隐","笔压","倾斜","方向","笔旋转"];currentIndex:Math.max(0,modes.indexOf(root.dualScatterControl("amountControl").source));onActivated:root.setDualScatterControl("amountControl",modes[currentIndex]);tooltip:"控制第二笔尖的散布幅度；渐隐按主逻辑印章步数下降。"}
                        CompactSpinBox {objectName:"settingsDualScatterAmountMinimum";visible:root.dualScatterControl("amountControl").source!=="off";Layout.fillWidth:true;from:0;to:100;value:Math.round(root.dualScatterControl("amountControl").minimum*100);onValueModified:root.setDualScatterControlField("amountControl","minimum",value/100);tooltip:"第二笔尖幅度控制的最低比例"}
                        CompactSpinBox {objectName:"settingsDualScatterAmountFade";visible:root.dualScatterControl("amountControl").source==="fade";Layout.fillWidth:true;from:1;to:1000;value:root.dualScatterControl("amountControl").fadeSteps||25;onValueModified:root.setDualScatterControlField("amountControl","fadeSteps",value);tooltip:"第二笔尖幅度渐隐步数"}
                        Label {text:"数量控制";color:Theme.muted;font.pixelSize:9}
                        CompactComboBox {objectName:"settingsDualScatterCountControl";Layout.fillWidth:true;readonly property var modes:["off","fade","pressure","tilt","direction","rotation"];model:["无","渐隐","笔压","倾斜","方向","笔旋转"];currentIndex:Math.max(0,modes.indexOf(root.dualScatterControl("countControl").source));onActivated:root.setDualScatterControl("countControl",modes[currentIndex]);tooltip:"控制第二笔尖每个路径印章的子印章数量。"}
                        CompactSpinBox {objectName:"settingsDualScatterCountMinimum";visible:root.dualScatterControl("countControl").source!=="off";Layout.fillWidth:true;from:0;to:100;value:Math.round(root.dualScatterControl("countControl").minimum*100);onValueModified:root.setDualScatterControlField("countControl","minimum",value/100);tooltip:"第二笔尖数量控制的最低比例"}
                        CompactSpinBox {objectName:"settingsDualScatterCountFade";visible:root.dualScatterControl("countControl").source==="fade";Layout.fillWidth:true;from:1;to:1000;value:root.dualScatterControl("countControl").fadeSteps||25;onValueModified:root.setDualScatterControlField("countControl","fadeSteps",value);tooltip:"第二笔尖数量渐隐步数"}
                    }
                    ColumnLayout {Layout.minimumWidth:0;visible:!root.library.dualExisting && root.dualPage===3;Layout.fillWidth:true;spacing:5
                        CompactCheckBox {objectName:"settingsDualShapeEnabled";text:"启用形状抖动";checked:root.dualModule("shapeJitter").enabled;onToggled:root.setDualField("enabled",checked,"shapeJitter");tooltip:"独立随机改变第二笔尖形状；关闭后保留参数。"}
                        DualField {title:"大小抖动";field:"size";module:"shapeJitter"}
                        DualField {title:"最小直径比例";field:"minimumSize";module:"shapeJitter"}
                        DualField {title:"角度抖动";field:"angle";module:"shapeJitter"}
                        DualField {title:"圆度抖动";field:"roundness";module:"shapeJitter"}
                        DualField {title:"最低圆度比例";field:"minimumRoundness";module:"shapeJitter";from:1}
                    }
                }
                }
                Loader {objectName:"brushSettingsCategory:5";Layout.fillWidth:true;Layout.minimumWidth:0;active:root.category===5;visible:active
                    sourceComponent:ColumnLayout {Layout.minimumWidth:0;visible:root.category===5;Layout.fillWidth:true;spacing:5
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
                }
                Loader {objectName:"brushSettingsCategory:6";Layout.fillWidth:true;Layout.minimumWidth:0;active:root.category===6;visible:active
                    sourceComponent:ColumnLayout {
                    visible:root.category===6;Layout.fillWidth:true;spacing:5
                    TipField {title:"不透明度";field:"opacity";from:0;to:100;factor:100;unit:"%";help:"绘画与擦除的整笔不透明度上限；涂抹仍使用强度语义。";sliderName:"settingsBrushOpacity"}
                    TipField {title:"流量";field:"flow";from:0;to:100;factor:100;unit:"%";help:"每次印章的沉积强度；重叠会逐步累积。"}
                    Label {text:"随机传递（DrawVerse 每笔稳定序列）";color:Theme.text;font.pixelSize:10}
                    TransferField {title:"大小抖动";field:"size";help:"每个实际印章随机缩小的最大比例；0% 关闭。"}
                    TransferField {title:"流量抖动";field:"flow";help:"每个实际印章随机降低流量的最大比例；0% 关闭。"}
                    TransferField {title:"不透明度抖动";field:"opacity";help:"每个实际印章随机降低整笔不透明度上限的最大比例；0% 关闭。"}
                    Label {text:"动态来源";color:Theme.text;font.pixelSize:10}
                    ColumnLayout {Layout.fillWidth:true
                        Label {objectName:"settingsDynamicsSizeSourceLabel";text:"尺寸动态来源";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                        CompactComboBox {objectName:"settingsDynamicsSizeSource";Layout.fillWidth:true;model:["全部来源相乘","速度","切向压力","笔触距离","倾斜幅度","笔旋转","笔触方向"];currentIndex:root.dynamicsSourceIndex(root.library.dynamicsSizeSource);onActivated:{if(!root.setDynamicsSource("size",currentIndex))currentIndex=root.dynamicsSourceIndex(root.library.dynamicsSizeSource)}tooltip:"选择尺寸响应的动态来源；全部来源表示六个来源的曲线相乘，保持旧笔刷行为。"}
                    }
                    ColumnLayout {Layout.fillWidth:true
                        Label {objectName:"settingsDynamicsFlowSourceLabel";text:"流量动态来源";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                        CompactComboBox {objectName:"settingsDynamicsFlowSource";Layout.fillWidth:true;model:["全部来源相乘","速度","切向压力","笔触距离","倾斜幅度","笔旋转","笔触方向"];currentIndex:root.dynamicsSourceIndex(root.library.dynamicsFlowSource);onActivated:{if(!root.setDynamicsSource("flow",currentIndex))currentIndex=root.dynamicsSourceIndex(root.library.dynamicsFlowSource)}tooltip:"选择流量响应的动态来源；只使用所选来源的流量曲线。"}
                    }
                    ColumnLayout {Layout.fillWidth:true
                        Label {objectName:"settingsDynamicsOpacitySourceLabel";text:"不透明度动态来源";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                        CompactComboBox {objectName:"settingsDynamicsOpacitySource";Layout.fillWidth:true;model:["全部来源相乘","速度","切向压力","笔触距离","倾斜幅度","笔旋转","笔触方向"];currentIndex:root.dynamicsSourceIndex(root.library.dynamicsOpacitySource);onActivated:{if(!root.setDynamicsSource("opacity",currentIndex))currentIndex=root.dynamicsSourceIndex(root.library.dynamicsOpacitySource)}tooltip:"选择整笔不透明度上限响应的动态来源；只使用所选来源的不透明度曲线。"}
                    }
                    CompactCheckBox {objectName:"settingsPressureOpacity";text:"不透明度压感";checked:root.library.pressureOpacity;font.pixelSize:10;onToggled:root.library.pressureOpacity=checked;tooltip:"压力决定局部整笔不透明度上限；较低压力不会使已画出的颜色变浅。流量控制接近上限的速度。"}
                    PressureCurveEditor {library:root.library;target:"opacity"}
                    CompactCheckBox {objectName:"settingsPressureFlow";text:"流量压感";checked:root.library.pressureFlow;font.pixelSize:10;onToggled:root.library.pressureFlow=checked;tooltip:"使用笔压力控制绘画与擦除流量；涂抹强度压感由涂抹工具单独控制。"}
                    PressureCurveEditor {library:root.library;target:"flow"}
                    Label {text:"涂抹使用此流量曲线，由涂抹工具的强度压感开关控制。";Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                    CompactCheckBox {objectName:"settingsSpeedEnabled";text:"启用速度动态";checked:root.library.speedDynamicsEnabled;font.pixelSize:10;onToggled:root.setSpeedDynamicsEnabled(checked);tooltip:"根据文档像素/秒控制尺寸、流量和整笔不透明度；下方曲线的横轴为速度比例。"}
                    RowLayout {Layout.fillWidth:true;visible:root.library.speedDynamicsEnabled
                        Label {text:"速度上限";color:Theme.text;Layout.fillWidth:true}
                        CompactSpinBox {objectName:"settingsSpeedMax";from:1;to:100000;editable:true;implicitWidth:86;implicitHeight:20;value:Math.round(root.library.speedDynamicsMaxSpeed);onValueModified:root.setSpeedDynamicsMax(value);tooltip:"速度达到此值后使用曲线右端；单位像素/秒。"}
                        Label {text:"px/s";color:Theme.muted;font.pixelSize:9}
                    }
                    SpeedCurveEditor {library:root.library;target:"size";prefix:"speedSize";caption:"尺寸随速度变化";curveHelp:"横轴为归一化速度，纵轴为尺寸比例；Ctrl点击节点切换直线/曲线。";minimumHelp:"最低尺寸比例；不启用时保持原尺寸。";nodeModes:root.library.speedDynamics.size.nodeModes || [];smooth:root.library.speedDynamics.size.interpolation==="smooth-pchip-v1";curve:root.library.speedSizeCurve;minimum:root.library.speedSizeMinimum}
                    SpeedCurveEditor {library:root.library;target:"flow";prefix:"speedFlow";caption:"流量随速度变化";curveHelp:"横轴为归一化速度，纵轴为流量比例；Ctrl点击节点切换直线/曲线。";minimumHelp:"最低流量比例。";nodeModes:root.library.speedDynamics.flow.nodeModes || [];smooth:root.library.speedDynamics.flow.interpolation==="smooth-pchip-v1";curve:root.library.speedFlowCurve;minimum:root.library.speedFlowMinimum}
                    SpeedCurveEditor {library:root.library;target:"opacity";prefix:"speedOpacity";caption:"不透明度随速度变化";curveHelp:"横轴为归一化速度，纵轴为整笔不透明度比例；独立于流量曲线。";minimumHelp:"最低不透明度比例。";nodeModes:root.library.speedDynamics.opacity.nodeModes || [];smooth:root.library.speedDynamics.opacity.interpolation==="smooth-pchip-v1";curve:root.library.speedOpacityCurve;minimum:root.library.speedOpacityMinimum}
                    CompactCheckBox {objectName:"settingsTangentEnabled";text:"启用切向压力动态";checked:root.library.tangentDynamicsEnabled;font.pixelSize:10;onToggled:root.setTangentDynamicsEnabled(checked);tooltip:"使用数位笔的切向压力（-1到1归一化）控制尺寸、流量和整笔不透明度。"}
                    SpeedCurveEditor {library:root.library;target:"size";prefix:"tangentSize";caption:"尺寸随切向压力变化";curveHelp:"横轴为切向压力归一化值，纵轴为尺寸比例；Ctrl点击节点切换直线/曲线。";minimumHelp:"最低尺寸比例。";nodeModes:root.library.tangentDynamics.size.nodeModes || [];smooth:root.library.tangentDynamics.size.interpolation==="smooth-pchip-v1";curve:root.library.tangentSizeCurve;minimum:root.library.tangentSizeMinimum;setControl:function(points,minimum,modes){return root.setTangentControl("size",points,minimum,modes)};resetControl:function(){return root.resetTangentControl("size")}}
                    SpeedCurveEditor {library:root.library;target:"flow";prefix:"tangentFlow";caption:"流量随切向压力变化";curveHelp:"横轴为切向压力归一化值，纵轴为流量比例；Ctrl点击节点切换直线/曲线。";minimumHelp:"最低流量比例。";nodeModes:root.library.tangentDynamics.flow.nodeModes || [];smooth:root.library.tangentDynamics.flow.interpolation==="smooth-pchip-v1";curve:root.library.tangentFlowCurve;minimum:root.library.tangentFlowMinimum;setControl:function(points,minimum,modes){return root.setTangentControl("flow",points,minimum,modes)};resetControl:function(){return root.resetTangentControl("flow")}}
                    SpeedCurveEditor {library:root.library;target:"opacity";prefix:"tangentOpacity";caption:"不透明度随切向压力变化";curveHelp:"横轴为切向压力归一化值，纵轴为整笔不透明度比例；独立于流量曲线。";minimumHelp:"最低不透明度比例。";nodeModes:root.library.tangentDynamics.opacity.nodeModes || [];smooth:root.library.tangentDynamics.opacity.interpolation==="smooth-pchip-v1";curve:root.library.tangentOpacityCurve;minimum:root.library.tangentOpacityMinimum;setControl:function(points,minimum,modes){return root.setTangentControl("opacity",points,minimum,modes)};resetControl:function(){return root.resetTangentControl("opacity")}}
                    CompactCheckBox {objectName:"settingsDistanceEnabled";text:"启用距离动态";checked:root.library.distanceDynamicsEnabled;font.pixelSize:10;onToggled:root.setDistanceDynamicsEnabled(checked);tooltip:"按当前笔触累计文档距离控制尺寸、流量和整笔不透明度。"}
                    RowLayout {Layout.fillWidth:true;visible:root.library.distanceDynamicsEnabled
                        Label {text:"距离上限";color:Theme.text;Layout.fillWidth:true}
                        CompactSpinBox {objectName:"settingsDistanceMax";from:1;to:100000;editable:true;implicitWidth:86;implicitHeight:20;value:Math.round(root.library.distanceDynamicsMaxDistance);onValueModified:root.setDistanceDynamicsMax(value);tooltip:"笔触距离达到此值后使用曲线右端；单位文档像素。"}
                        Label {text:"px";color:Theme.muted;font.pixelSize:9}
                    }
                    SpeedCurveEditor {library:root.library;target:"size";prefix:"distanceSize";caption:"尺寸随笔触距离变化";curveHelp:"横轴为归一化笔触距离，纵轴为尺寸比例；Ctrl点击节点切换直线/曲线。";minimumHelp:"最低尺寸比例。";nodeModes:root.library.distanceDynamics.size.nodeModes || [];smooth:root.library.distanceDynamics.size.interpolation==="smooth-pchip-v1";curve:root.library.distanceSizeCurve;minimum:root.library.distanceSizeMinimum;setControl:function(points,minimum,modes){return root.setDistanceControl("size",points,minimum,modes)};resetControl:function(){return root.resetDistanceControl("size")}}
                    SpeedCurveEditor {library:root.library;target:"flow";prefix:"distanceFlow";caption:"流量随笔触距离变化";curveHelp:"横轴为归一化笔触距离，纵轴为流量比例；Ctrl点击节点切换直线/曲线。";minimumHelp:"最低流量比例。";nodeModes:root.library.distanceDynamics.flow.nodeModes || [];smooth:root.library.distanceDynamics.flow.interpolation==="smooth-pchip-v1";curve:root.library.distanceFlowCurve;minimum:root.library.distanceFlowMinimum;setControl:function(points,minimum,modes){return root.setDistanceControl("flow",points,minimum,modes)};resetControl:function(){return root.resetDistanceControl("flow")}}
                    SpeedCurveEditor {library:root.library;target:"opacity";prefix:"distanceOpacity";caption:"不透明度随笔触距离变化";curveHelp:"横轴为归一化笔触距离，纵轴为整笔不透明度比例；独立于流量曲线。";minimumHelp:"最低不透明度比例。";nodeModes:root.library.distanceDynamics.opacity.nodeModes || [];smooth:root.library.distanceDynamics.opacity.interpolation==="smooth-pchip-v1";curve:root.library.distanceOpacityCurve;minimum:root.library.distanceOpacityMinimum;setControl:function(points,minimum,modes){return root.setDistanceControl("opacity",points,minimum,modes)};resetControl:function(){return root.resetDistanceControl("opacity")}}
                    CompactCheckBox {objectName:"settingsTiltEnabled";text:"启用倾斜幅度动态";checked:root.library.tiltDynamicsEnabled;font.pixelSize:10;onToggled:root.setTiltDynamicsEnabled(checked);tooltip:"使用数位笔倾斜幅度（0到90度归一化）控制尺寸、流量和整笔不透明度。"}
                    SpeedCurveEditor {library:root.library;target:"size";prefix:"tiltSize";caption:"尺寸随倾斜幅度变化";curveHelp:"横轴为倾斜幅度归一化值，纵轴为尺寸比例；Ctrl点击节点切换直线/曲线。";minimumHelp:"最低尺寸比例。";nodeModes:root.library.tiltDynamics.size.nodeModes || [];smooth:root.library.tiltDynamics.size.interpolation==="smooth-pchip-v1";curve:root.library.tiltSizeCurve;minimum:root.library.tiltSizeMinimum;setControl:function(points,minimum,modes){return root.setTiltControl("size",points,minimum,modes)};resetControl:function(){return root.resetTiltControl("size")}}
                    SpeedCurveEditor {library:root.library;target:"flow";prefix:"tiltFlow";caption:"流量随倾斜幅度变化";curveHelp:"横轴为倾斜幅度归一化值，纵轴为流量比例；Ctrl点击节点切换直线/曲线。";minimumHelp:"最低流量比例。";nodeModes:root.library.tiltDynamics.flow.nodeModes || [];smooth:root.library.tiltDynamics.flow.interpolation==="smooth-pchip-v1";curve:root.library.tiltFlowCurve;minimum:root.library.tiltFlowMinimum;setControl:function(points,minimum,modes){return root.setTiltControl("flow",points,minimum,modes)};resetControl:function(){return root.resetTiltControl("flow")}}
                    SpeedCurveEditor {library:root.library;target:"opacity";prefix:"tiltOpacity";caption:"不透明度随倾斜幅度变化";curveHelp:"横轴为倾斜幅度归一化值，纵轴为整笔不透明度比例；独立于流量曲线。";minimumHelp:"最低不透明度比例。";nodeModes:root.library.tiltDynamics.opacity.nodeModes || [];smooth:root.library.tiltDynamics.opacity.interpolation==="smooth-pchip-v1";curve:root.library.tiltOpacityCurve;minimum:root.library.tiltOpacityMinimum;setControl:function(points,minimum,modes){return root.setTiltControl("opacity",points,minimum,modes)};resetControl:function(){return root.resetTiltControl("opacity")}}
                    CompactCheckBox {objectName:"settingsRotationEnabled";text:"启用笔旋转动态";checked:root.library.rotationDynamicsEnabled;font.pixelSize:10;onToggled:root.setRotationDynamicsEnabled(checked);tooltip:"使用数位笔旋转角（整圈归一化）控制尺寸、流量和整笔不透明度。"}
                    SpeedCurveEditor {library:root.library;target:"size";prefix:"rotationSize";caption:"尺寸随笔旋转变化";curveHelp:"横轴为笔旋转整圈归一化值，纵轴为尺寸比例；Ctrl点击节点切换直线/曲线。";minimumHelp:"最低尺寸比例。";nodeModes:root.library.rotationDynamics.size.nodeModes || [];smooth:root.library.rotationDynamics.size.interpolation==="smooth-pchip-v1";curve:root.library.rotationSizeCurve;minimum:root.library.rotationSizeMinimum;setControl:function(points,minimum,modes){return root.setRotationControl("size",points,minimum,modes)};resetControl:function(){return root.resetRotationControl("size")}}
                    SpeedCurveEditor {library:root.library;target:"flow";prefix:"rotationFlow";caption:"流量随笔旋转变化";curveHelp:"横轴为笔旋转整圈归一化值，纵轴为流量比例；Ctrl点击节点切换直线/曲线。";minimumHelp:"最低流量比例。";nodeModes:root.library.rotationDynamics.flow.nodeModes || [];smooth:root.library.rotationDynamics.flow.interpolation==="smooth-pchip-v1";curve:root.library.rotationFlowCurve;minimum:root.library.rotationFlowMinimum;setControl:function(points,minimum,modes){return root.setRotationControl("flow",points,minimum,modes)};resetControl:function(){return root.resetRotationControl("flow")}}
                    SpeedCurveEditor {library:root.library;target:"opacity";prefix:"rotationOpacity";caption:"不透明度随笔旋转变化";curveHelp:"横轴为笔旋转整圈归一化值，纵轴为整笔不透明度比例；独立于流量曲线。";minimumHelp:"最低不透明度比例。";nodeModes:root.library.rotationDynamics.opacity.nodeModes || [];smooth:root.library.rotationDynamics.opacity.interpolation==="smooth-pchip-v1";curve:root.library.rotationOpacityCurve;minimum:root.library.rotationOpacityMinimum;setControl:function(points,minimum,modes){return root.setRotationControl("opacity",points,minimum,modes)};resetControl:function(){return root.resetRotationControl("opacity")}}
                    CompactCheckBox {objectName:"settingsDirectionEnabled";text:"启用笔触方向动态";checked:root.library.directionDynamicsEnabled;font.pixelSize:10;onToggled:root.setDirectionDynamicsEnabled(checked);tooltip:"使用画布运动方向角控制尺寸、流量和整笔不透明度。"}
                    SpeedCurveEditor {library:root.library;target:"size";prefix:"directionSize";caption:"尺寸随笔触方向变化";curveHelp:"横轴为笔触方向整圈归一化值，纵轴为尺寸比例；Ctrl点击节点切换直线/曲线。";minimumHelp:"最低尺寸比例。";nodeModes:root.library.directionDynamics.size.nodeModes || [];smooth:root.library.directionDynamics.size.interpolation==="smooth-pchip-v1";curve:root.library.directionSizeCurve;minimum:root.library.directionSizeMinimum;setControl:function(points,minimum,modes){return root.setDirectionControl("size",points,minimum,modes)};resetControl:function(){return root.resetDirectionControl("size")}}
                    SpeedCurveEditor {library:root.library;target:"flow";prefix:"directionFlow";caption:"流量随笔触方向变化";curveHelp:"横轴为笔触方向整圈归一化值，纵轴为流量比例；Ctrl点击节点切换直线/曲线。";minimumHelp:"最低流量比例。";nodeModes:root.library.directionDynamics.flow.nodeModes || [];smooth:root.library.directionDynamics.flow.interpolation==="smooth-pchip-v1";curve:root.library.directionFlowCurve;minimum:root.library.directionFlowMinimum;setControl:function(points,minimum,modes){return root.setDirectionControl("flow",points,minimum,modes)};resetControl:function(){return root.resetDirectionControl("flow")}}
                    SpeedCurveEditor {library:root.library;target:"opacity";prefix:"directionOpacity";caption:"不透明度随笔触方向变化";curveHelp:"横轴为笔触方向整圈归一化值，纵轴为整笔不透明度比例；独立于流量曲线。";minimumHelp:"最低不透明度比例。";nodeModes:root.library.directionDynamics.opacity.nodeModes || [];smooth:root.library.directionDynamics.opacity.interpolation==="smooth-pchip-v1";curve:root.library.directionOpacityCurve;minimum:root.library.directionOpacityMinimum;setControl:function(points,minimum,modes){return root.setDirectionControl("opacity",points,minimum,modes)};resetControl:function(){return root.resetDirectionControl("opacity")}}
                    CompactCheckBox {objectName:"settingsFadeEnabled";text:"启用渐隐动态";checked:root.library.fadeDynamicsEnabled;font.pixelSize:10;onToggled:root.setFadeDynamicsEnabled(checked);tooltip:"按印章步数从完整响应渐隐到最低比例；适用于 ABR 渐隐控制。"}
                    RowLayout {Layout.fillWidth:true;visible:root.library.fadeDynamicsEnabled
                        Label {text:"步数";color:Theme.text;Layout.fillWidth:true}
                        CompactSpinBox {objectName:"settingsFadeSteps";from:1;to:10000;editable:true;implicitWidth:86;implicitHeight:20;value:root.library.fadeSteps;onValueModified:root.setFadeSteps(value);tooltip:"渐隐达到最低比例所需的印章数量。"}
                    }
                    SpeedCurveEditor {library:root.library;target:"size";prefix:"fadeSize";caption:"尺寸渐隐曲线";curveHelp:"横轴为渐隐进度，纵轴为尺寸比例；Ctrl点击节点切换直线/曲线。";minimumHelp:"最低尺寸比例。";nodeModes:root.library.fadeDynamics.size.nodeModes || [];smooth:root.library.fadeDynamics.size.interpolation==="smooth-pchip-v1";curve:root.library.fadeSizeCurve;minimum:root.library.fadeSizeMinimum;setControl:function(points,minimum,modes){return root.setFadeControl("size",points,minimum,modes)};resetControl:function(){return root.resetFadeControl("size")}}
                    SpeedCurveEditor {library:root.library;target:"flow";prefix:"fadeFlow";caption:"流量渐隐曲线";curveHelp:"横轴为渐隐进度，纵轴为流量比例；Ctrl点击节点切换直线/曲线。";minimumHelp:"最低流量比例。";nodeModes:root.library.fadeDynamics.flow.nodeModes || [];smooth:root.library.fadeDynamics.flow.interpolation==="smooth-pchip-v1";curve:root.library.fadeFlowCurve;minimum:root.library.fadeFlowMinimum;setControl:function(points,minimum,modes){return root.setFadeControl("flow",points,minimum,modes)};resetControl:function(){return root.resetFadeControl("flow")}}
                    SpeedCurveEditor {library:root.library;target:"opacity";prefix:"fadeOpacity";caption:"不透明度渐隐曲线";curveHelp:"横轴为渐隐进度，纵轴为整笔不透明度比例；Ctrl点击节点切换直线/曲线。";minimumHelp:"最低不透明度比例。";nodeModes:root.library.fadeDynamics.opacity.nodeModes || [];smooth:root.library.fadeDynamics.opacity.interpolation==="smooth-pchip-v1";curve:root.library.fadeOpacityCurve;minimum:root.library.fadeOpacityMinimum;setControl:function(points,minimum,modes){return root.setFadeControl("opacity",points,minimum,modes)};resetControl:function(){return root.resetFadeControl("opacity")}}
                    CompactCheckBox {objectName:"settingsAirbrushEnabled";text:"喷枪";checked:root.library.airbrush.enabled;font.pixelSize:10;onToggled:root.setAirbrushField("enabled",checked);tooltip:"按住不动时按固定时间间隔持续沉积颜色；用于画笔和橡皮，涂抹与湿混不启用。"}
                    RowLayout {Layout.fillWidth:true;visible:root.library.airbrush.enabled
                        Label {text:"间隔";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                        CompactSpinBox {objectName:"settingsAirbrushInterval";from:1;to:1000;editable:true;implicitWidth:70;implicitHeight:20;value:Math.round(root.library.airbrush.intervalMs);onValueModified:root.setAirbrushField("intervalMs",value);tooltip:"同一位置两次自动沉积之间的时间，单位毫秒。"}
                        Label {text:"ms";color:Theme.muted;font.pixelSize:9}
                    }
                }
                }
                Loader {objectName:"brushSettingsCategory:7";Layout.fillWidth:true;Layout.minimumWidth:0;active:root.category===7;visible:active
                    sourceComponent:ColumnLayout {
                    visible:root.category===7;Layout.fillWidth:true;spacing:10
                    CompactCheckBox {objectName:"brushCursorEnabled";text:"笔尖轮廓";checked:root.library.cursorOutline;font.pixelSize:10;onToggled:root.library.cursorOutline=checked;tooltip:"按画布明暗自动调整笔尖边界的黑白颜色。"}
                    CompactCheckBox {objectName:"brushCursorComposite";text:"复合光标";checked:root.library.cursorComposite;font.pixelSize:10;onToggled:root.library.cursorComposite=checked;tooltip:"同时显示允许溢出的第二基础笔尖范围；只影响显示，不改变实际笔触，不预演随机散布。"}
                    CompactCheckBox {objectName:"brushCursorImageEnabled";text:"半透明图片";checked:root.library.cursorImage;font.pixelSize:10;onToggled:root.library.cursorImage=checked;tooltip:"用白色显示实际图片笔尖覆盖，不影响绘画；简化圆形模式下隐藏。"}
                    ColumnLayout {
                        Layout.fillWidth:true;spacing:2
                        Label {text:"图片不透明度";color:Theme.text;font.pixelSize:10}
                        CompactSlider {objectName:"brushCursorImageOpacitySlider";Layout.fillWidth:true;Layout.minimumWidth:0;from:1;to:100;stepSize:1;value:root.library.cursorImageOpacity;enabled:root.library.cursorImage;onMoved:root.library.cursorImageOpacity=Math.round(value);ToolTip.delay:1000;ToolTip.visible:hovered;ToolTip.text:"仅调整光标图片的显示透明度，不影响实际笔触。"}
                        RowLayout {
                            CompactSpinBox {objectName:"brushCursorImageOpacity";from:1;to:100;editable:true;implicitWidth:66;implicitHeight:20;value:root.library.cursorImageOpacity;enabled:root.library.cursorImage;onValueModified:root.library.cursorImageOpacity=value;tooltip:"光标图片显示不透明度（%）。"}
                            Label {text:"%";color:Theme.text;font.pixelSize:10}
                        }
                    }
                    Label {visible:root.library.cursorImage && PaintClient.cursorImageMessage.length>0;text:PaintClient.cursorImageMessage;Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                    CompactCheckBox {objectName:"brushCursorSimple";text:"简化圆形";checked:root.library.cursorSimple;font.pixelSize:10;onToggled:root.library.cursorSimple=checked;tooltip:"使用标称大小的圆形轮廓，不改变实际笔尖。"}
                    Label {objectName:"brushCursorAdaptiveHint";text:"轮廓自动适应画布明暗：暗处亮、亮处暗；软件显示使用黑白轮廓。";Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                    CompactCheckBox {objectName:"brushCursorCenter";text:"中心十字";checked:root.library.cursorCenter;font.pixelSize:10;onToggled:root.library.cursorCenter=checked;tooltip:"在笔尖中心显示精确定位十字。"}
                    CompactCheckBox {objectName:"brushCursorHoverOnly";text:"仅悬停时显示";checked:root.library.cursorHoverOnly;font.pixelSize:10;onToggled:root.library.cursorHoverOnly=checked;tooltip:"开始绘画时隐藏笔尖预览，结束后恢复。"}
                    RowLayout {
                        Label {text:"线宽";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                        CompactSpinBox {objectName:"brushCursorWidth";from:0;to:3;editable:true;implicitWidth:66;implicitHeight:20;value:root.library.cursorWidth;onValueModified:root.library.cursorWidth=value;tooltip:"0为单条细线；1–3为双线轮廓宽度，单位为界面像素。"}
                    }
                    Label {text:"全局绘画偏好；不随笔刷包导出。CapsLock 切换精确光标。";Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                    Label {text:PaintClient.brushOutlineMessage;visible:text.length>0;Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                }
                }
                Loader {objectName:"brushSettingsCategory:8";Layout.fillWidth:true;Layout.minimumWidth:0;active:root.category===8;visible:active
                    sourceComponent:ColumnLayout {
                    visible:root.category===8;Layout.fillWidth:true;spacing:6
                    Label {objectName:"brushSourceDescription";textFormat:Text.PlainText;text:root.library.sourceResource.length?"导入时快照 · 当前编辑不会改变来源报告":"此预设没有 ABR 来源记录";Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:10}
                    RowLayout {Layout.fillWidth:true
                        Button {objectName:"refreshBrushSource";text:PaintClient.brushSourceBusy?"读取中…":"刷新来源";Layout.fillWidth:true;Layout.minimumWidth:0;implicitHeight:24;enabled:root.library.sourceResource.length>0 && !PaintClient.brushSourceBusy;onClicked:root.inspectSource()}
                        Button {objectName:"restoreBrushSource";text:"恢复来源基线";Layout.fillWidth:true;Layout.minimumWidth:0;implicitHeight:24;enabled:root.sourceInfo.baseline!==undefined && root.library.sourceResource.length>0;onClicked:root.library.restoreSourceBaseline(root.sourceInfo);ToolTip.delay:1000;ToolTip.visible:hovered;ToolTip.text:"恢复导入时的纹理、笔尖和动态参数；不改变预设身份或分组。"}
                    }
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
                Loader {objectName:"brushSettingsCategory:12";Layout.fillWidth:true;Layout.minimumWidth:0;active:root.category===12;visible:active
                    sourceComponent:ColumnLayout {Layout.fillWidth:true;spacing:5
                        CompactCheckBox {objectName:"settingsOutlineEnabled";text:"启用整笔描边";checked:root.outline.enabled;onToggled:if(!root.setOutlineField("enabled",checked))checked=Qt.binding(function(){return root.outline.enabled})}
                        Label {text:"算法";color:Theme.text;font.pixelSize:10}
                        CompactComboBox {objectName:"settingsOutlineAlgorithm";Layout.fillWidth:true;model:["建路径","SDF 遮罩"];property var values:["stroke-contour-v1","stroke-sdf-v1"];currentIndex:values.indexOf(root.outline.model);onActivated:root.setOutlineField("model",values[currentIndex])}
                        Label {visible:root.outline.model==="stroke-sdf-v1";text:"按描边宽度等距盖章，重叠取最大覆盖；不使用来源笔刷的大小、间距或散布。";color:Theme.muted;font.pixelSize:9;wrapMode:Text.WordWrap;Layout.fillWidth:true}
                        Label {text:"位置";color:Theme.text;font.pixelSize:10}
                        CompactComboBox {objectName:"settingsOutlinePlacement";Layout.fillWidth:true;model:["外描边","居中描边","内描边"];property var values:["outside","center","inside"];currentIndex:values.indexOf(root.outline.placement);onActivated:root.setOutlineField("placement",values[currentIndex])}
                        CompactCheckBox {objectName:"settingsOutlineSecondaryColor";text:"使用色轮第二颜色";checked:root.outline.secondaryColor;onToggled:root.setOutlineField("secondaryColor",checked)}
                        Label {text:"宽度方式";color:Theme.text;font.pixelSize:10}
                        CompactComboBox {objectName:"settingsOutlineWidthMode";Layout.fillWidth:true;model:["独立像素","相对主笔百分比"];currentIndex:root.outline.width.relative?1:0;onActivated:root.setOutlineWidth("relative",currentIndex===1)}
                        ParameterSlider {title:"宽度";sliderName:"settingsOutlinePixels";visible:!root.outline.width.relative;to:2000;decimals:0;unit:"px";value:root.outline.width.pixels;onCommitted:value=>root.setOutlineWidth("pixels",value)}
                        ParameterSlider {title:"相对宽度";sliderName:"settingsOutlinePercent";visible:root.outline.width.relative;to:10000;decimals:0;unit:"%";value:root.outline.width.percent;onCommitted:value=>root.setOutlineWidth("percent",value)}
                        Label {text:"描边来源";color:Theme.text;font.pixelSize:10}
                        CompactComboBox {objectName:"settingsOutlineSource";Layout.fillWidth:true;model:["自身笔刷","已有笔刷","纹理"];property var values:["self-brush","existing-brush","texture"];currentIndex:root.choosingOutlineTexture?2:values.indexOf(root.outline.source.model)
                            onActivated:{root.choosingOutlineTexture=currentIndex===2;if(currentIndex===0)root.setOutlineField("source",{model:"self-brush"});else if(currentIndex===1)root.library.usePresetAsOutline(root.library.selectedId)}}
                        CompactComboBox {objectName:"settingsOutlinePreset";visible:root.outline.source.model==="existing-brush";Layout.fillWidth:true;model:root.library.presetRows();textRole:"name";currentIndex:-1;displayText:"选择笔刷并捕获快照";onActivated:{if(currentIndex>=0)root.library.usePresetAsOutline(model[currentIndex].id);currentIndex=-1}}
                        ColumnLayout {visible:root.outline.source.model==="existing-brush" && root.outline.model!=="stroke-sdf-v1";Layout.fillWidth:true
                            ParameterSlider {title:"大小";sliderName:"settingsOutlineBrushSize";from:1;to:2000;decimals:0;unit:"px";value:root.outline.source.brush?.diameter||24;onCommitted:value=>root.setOutlineBrush("diameter",value)}
                            ParameterSlider {title:"间距";sliderName:"settingsOutlineBrushSpacing";from:1;to:1000;decimals:0;unit:"%";value:(root.outline.source.brush?.spacing||.25)*100;onCommitted:value=>root.setOutlineBrush("spacing",value/100)}
                            CompactCheckBox {text:"启用散布";checked:root.outline.source.brush?.scatterDynamics?.enabled||false;onToggled:root.setOutlineScatter("enabled",checked)}
                            CompactCheckBox {text:"双轴散布";checked:root.outline.source.brush?.scatterDynamics?.bothAxes||false;onToggled:root.setOutlineScatter("bothAxes",checked)}
                            ParameterSlider {title:"散布";sliderName:"settingsOutlineBrushScatter";to:1000;decimals:0;unit:"%";value:(root.outline.source.brush?.scatterDynamics?.amount||0)*100;onCommitted:value=>root.setOutlineScatter("amount",value/100)}
                            ParameterSlider {title:"数量";sliderName:"settingsOutlineBrushCount";from:1;to:16;value:root.outline.source.brush?.scatterDynamics?.count||1;onCommitted:value=>root.setOutlineScatter("count",Math.round(value))}
                        }
                        ColumnLayout {visible:root.choosingOutlineTexture || root.outline.source.model==="texture" || root.outline.source.model==="self-brush";Layout.fillWidth:true
                            BrushResourcePicker {objectName:"settingsOutlineTexturePicker";Layout.fillWidth:true;resource:root.outline.source.texture?.resource||"";resourceRole:root.outline.source.model==="self-brush" && !root.choosingOutlineTexture?4:3;showActions:true;importName:"importOutlineTexture";resetName:"resetOutlineTexture";onImportRequested:{outlineTextureDialog.selfBrush=root.outline.source.model==="self-brush" && !root.choosingOutlineTexture;outlineTextureDialog.open()} onResetRequested:{root.choosingOutlineTexture=root.outline.source.model==="texture" || root.choosingOutlineTexture;root.setOutlineField("source",{model:"self-brush"})}}
                            Label {objectName:"settingsOutlineTextureHint";text:root.outline.source.model==="self-brush" && !root.choosingOutlineTexture?"自身描边独立纹理；不改变主笔刷纹理。":"选择资源或直接导入描边纹理；独立于主笔刷纹理。";color:Theme.muted;font.pixelSize:9;wrapMode:Text.WordWrap;Layout.fillWidth:true}
                            CompactCheckBox {objectName:"settingsSelfOutlineTextureEnabled";visible:root.outline.source.model==="self-brush" && !root.choosingOutlineTexture;enabled:root.outline.source.texture!==undefined;text:"启用描边纹理";checked:root.outline.source.texture?.enabled||false;onToggled:root.setOutlineTexture("enabled",checked)}
                            CompactComboBox {objectName:"settingsOutlineTextureSpace";Layout.fillWidth:true;enabled:root.outline.source.texture!==undefined;model:["文档纸纹","笔尖局部"];currentIndex:root.outline.source.texture?.space==="tip"?1:0;onActivated:root.setOutlineTexture("space",currentIndex===1?"tip":"document")}
                            ParameterSlider {title:"深度";sliderName:"settingsOutlineTextureDepth";enabled:root.outline.source.texture!==undefined;unit:"%";value:(root.outline.source.texture?.depth||0)*100;onCommitted:value=>root.setOutlineTexture("depth",value/100)}
                        }
                        Label {text:"沿整条笔触轮廓生成；当前支持干画笔普通像素层。";color:Theme.muted;font.pixelSize:9;wrapMode:Text.WordWrap;Layout.fillWidth:true}
                    }
                }
                Loader {objectName:"brushSettingsCategory:11";Layout.fillWidth:true;Layout.minimumWidth:0;active:root.category===11;visible:active
                    sourceComponent:ColumnLayout {visible:root.category===11;Layout.fillWidth:true;Layout.minimumWidth:0;spacing:5
                    CompactCheckBox {objectName:"settingsTaperEnabled";text:"启用起收笔";checked:root.library.strokeTaper.enabled;onToggled:{let data=Object.assign({},root.library.strokeTaper);data.enabled=checked;root.library.setStrokeTaper(data)} tooltip:"按实际绘画距离调整笔触两端；关闭时保留所有参数。"}
                    Label {objectName:"settingsTaperEndLabel";text:"编辑笔端";color:Theme.text;font.pixelSize:10;Layout.fillWidth:true}
                    CompactComboBox {objectName:"settingsTaperEnd";Layout.fillWidth:true;model:["起笔","收笔"];currentIndex:root.taperEnd==="start"?0:1;onActivated:root.taperEnd=currentIndex===0?"start":"end"}
                    Label {text:"长度模式";color:Theme.text;font.pixelSize:10}
                    CompactComboBox {objectName:"settingsTaperLengthMode";Layout.fillWidth:true;model:["独立像素","相对笔刷尺寸"];currentIndex:root.taperRelative?1:0;onActivated:root.setTaperLengthMode(currentIndex===1)}
                    CompactCheckBox {objectName:"settingsTaperTransparent";text:"端部透明";checked:root.library.strokeTaper[root.taperEnd].opacity.minimum<1;onToggled:root.setTaperTransparent(checked);tooltip:"关闭后不额外减淡该端，尺寸曲线仍然生效。"}
                    RowLayout {Layout.fillWidth:true;visible:!root.taperRelative
                        Label {text:"长度";color:Theme.text;Layout.fillWidth:true}
                        CompactTextField {objectName:"settingsTaperLength";Layout.preferredWidth:90;implicitHeight:20;text:root.library.strokeTaper[root.taperEnd].lengthPx.toFixed(0);validator:IntValidator {bottom:0;top:4096;locale:"C"} onEditingFinished:{if(acceptableInput)root.setTaperEndField("lengthPx",Math.round(Number(text)));text=Qt.binding(function(){return root.library.strokeTaper[root.taperEnd].lengthPx.toFixed(0)})} hoverEnabled:true;ToolTip.delay:1000;ToolTip.visible:hovered;ToolTip.text:"文档像素距离，0 表示该端不处理；不随笔刷尺寸缩放。"}
                        Label {text:"px";color:Theme.muted}
                    }
                    CompactSlider {objectName:"settingsTaperLengthSlider";visible:!root.taperRelative;Layout.fillWidth:true;from:0;to:4096;value:root.library.strokeTaper[root.taperEnd].lengthPx;stepSize:1;snapMode:Slider.SnapAlways;onMoved:root.setTaperEndField("lengthPx",Math.round(value))}
                    RowLayout {Layout.fillWidth:true;visible:root.taperRelative
                        Label {text:"相对长度";color:Theme.text;Layout.fillWidth:true}
                        CompactTextField {objectName:"settingsTaperPercent";Layout.preferredWidth:90;implicitHeight:20;text:Number(root.library.strokeTaper[root.taperEnd].lengthPercent||0).toFixed(0);validator:IntValidator {bottom:0;top:10000;locale:"C"} onEditingFinished:{if(acceptableInput)root.setTaperEndField("lengthPercent",Math.round(Number(text)));text=Qt.binding(function(){return Number(root.library.strokeTaper[root.taperEnd].lengthPercent||0).toFixed(0)})}}
                        Label {text:"%";color:Theme.muted}
                    }
                    CompactSlider {objectName:"settingsTaperPercentSlider";visible:root.taperRelative;Layout.fillWidth:true;from:0;to:10000;value:root.library.strokeTaper[root.taperEnd].lengthPercent||0;stepSize:1;snapMode:Slider.SnapAlways;onMoved:root.setTaperEndField("lengthPercent",Math.round(value))}
                    Label {visible:root.taperRelative;text:"实际长度 "+Math.min(4096,root.library.radius*2*(root.library.strokeTaper[root.taperEnd].lengthPercent||0)/100).toFixed(2)+" px（上限4096）";color:Theme.muted;font.pixelSize:9}
                    PressureCurveEditor {library:taperControls;target:"size";prefix:"taperSize";fixedEndpoints:true;caption:"尺寸随距离变化";curveHelp:"横轴为从笔端向内的归一化距离，纵轴为尺寸比例；Ctrl点击节点切换直线/曲线。端点固定。";minimumHelp:"最低尺寸比例；100% 表示不缩小笔尖。"}
                    PressureCurveEditor {library:taperControls;target:"opacity";prefix:"taperOpacity";fixedEndpoints:true;caption:"不透明度随距离变化";curveHelp:"横轴为从笔端向内的归一化距离，纵轴为不透明度比例；独立于尺寸曲线。端点固定。";minimumHelp:"最低不透明度比例；100% 表示不额外减淡。"}
                    Label {text:"收笔随实际笔画末端更新；抬笔不补线。两端重叠时取较低比例。";Layout.fillWidth:true;wrapMode:Text.WordWrap;color:Theme.muted;font.pixelSize:9}
                }
                }
                Loader {objectName:"brushSettingsCategory:10";Layout.fillWidth:true;Layout.minimumWidth:0;active:root.category===10;visible:active
                    sourceComponent:ColumnLayout {visible:root.category===10;Layout.fillWidth:true;Layout.minimumWidth:0;spacing:5
                    WetCheck {objectName:"settingsWetEnabled";text:"启用湿混";checked:root.library.wetMix.enabled;onToggled:if(!root.setWetField("enabled",checked))checked=Qt.binding(function(){return root.library.wetMix.enabled});tooltip:"画笔和涂抹复用二维载色。涂抹仍使用工具栏强度、压感和取样设置；橡皮请先关闭湿混。关闭时保留参数。"}
                    WetField {title:"湿度";field:"wetness";help:"控制画布颜色参与拾色和混合的程度。"}
                    WetField {title:"拾色";field:"pickup";help:"每次印章将画布颜色带入载体的比例。"}
                    WetField {title:"混合";field:"mix";help:"沉积时将载色与当前位置的画布颜色混合。"}
                    WetField {title:"载色量";field:"load";help:"重新载色时装入前景色的量；不会缩放已经保留的颜色。"}
                    WetField {title:"沉积";field:"deposit";help:"将笔尖持有的颜色沉积到画布上的比例。"}
                    WetField {title:"消耗";field:"consumption";help:"沿绘画距离消耗载色；抬笔移动不消耗。"}
                    WetCheck {objectName:"settingsWetAllLayers";text:"对所有图层取样";checked:root.library.wetMix.allLayers;onToggled:root.setWetField("allLayers",checked);tooltip:"画笔从可见合成结果拾色，只修改当前图层。涂抹使用工具栏的同名开关；编辑蒙版时不能开启合成取样。"}
                    WetCheck {objectName:"settingsWetAutoClean";text:"每笔自动清洗";checked:root.library.wetMix.autoClean;onToggled:root.setWetField("autoClean",checked);tooltip:"每条笔触开始时先清除保留的颜色，再执行自动装载。"}
                    WetCheck {objectName:"settingsWetAutoLoad";text:"每笔自动载色";checked:root.library.wetMix.autoLoad;onToggled:root.setWetField("autoLoad",checked);tooltip:"每条笔触开始时装入前景色；涂抹还须开启手指绘画。关闭自动清洗和自动载色可保留上一笔或手动载色。"}
                    ColumnLayout {Layout.fillWidth:true;Layout.minimumWidth:0
                        Button {Layout.fillWidth:true;Layout.minimumWidth:0;objectName:"wetCleanBrush";text:"清洗";implicitHeight:24;enabled:root.library.wetMix.enabled && PaintClient.ready && !PaintClient.drawing && !PaintClient.wetActionBusy;onClicked:PaintClient.cleanWetBrush();ToolTip.delay:1000;ToolTip.visible:hovered;ToolTip.text:"清除当前主、第二笔尖保留的载色，不修改画布。"}
                        Button {Layout.fillWidth:true;Layout.minimumWidth:0;objectName:"wetLoadBrush";text:"载入前景色";implicitHeight:24;enabled:root.library.wetMix.enabled && PaintClient.ready && !PaintClient.drawing && !PaintClient.wetActionBusy;onClicked:PaintClient.loadWetBrush();ToolTip.delay:1000;ToolTip.visible:hovered;ToolTip.text:"按当前载色量装入前景色。关闭每笔自动清洗和自动载色可保留此次载色。"}
                    }
                }
                }
                Loader {objectName:"brushSettingsCategory:9";Layout.fillWidth:true;Layout.minimumWidth:0;active:root.category===9;visible:active
                    sourceComponent:ColumnLayout {visible:root.category===9;Layout.fillWidth:true;Layout.minimumWidth:0;spacing:5
                    CompactCheckBox {objectName:"settingsSmoothingEnabled";text:"启用平滑";checked:root.library.smoothing.enabled;onToggled:{let data=Object.assign({},root.library.smoothing);data.enabled=checked;root.library.setSmoothing(data)} tooltip:"平滑位置和压力输入；主笔刷与第二笔刷共用同一路径。关闭时仍保留参数。"}
                    SmoothingField {title:"位置平滑";field:"positionMs";to:250;unit:"ms";help:"减轻指针抖动；数值越大，跟随延迟越明显。"}
                    SmoothingField {title:"压力平滑";field:"pressureMs";to:250;unit:"ms";help:"独立平滑压力变化；不改变位置滤波的时间设置。"}
                    SmoothingField {title:"速度响应";field:"speedResponse";factor:100;to:100;unit:"%";help:"快速移动时减少位置平滑延迟；压力平滑保持独立。"}
                    SmoothingField {title:"稳定距离";field:"stabilizerPx";to:128;unit:"px";help:"指针移动超过此距离后画笔开始跟随；抬笔停在当前笔画末端，不补线。"}
                }
                }
                Button {objectName:"resetBrushPreset";visible:root.category===8;text:"重置预设";implicitHeight:22;Layout.minimumWidth:0;Layout.maximumWidth:parameterScroll.availableWidth;Layout.alignment:Qt.AlignLeft;Layout.topMargin:6;onClicked:root.library.resetPreset();ToolTip.delay:1000;ToolTip.visible:hovered;ToolTip.text:"恢复此预设的初始参数；不改变当前颜色、工具或画布历史。"}
            }
        }
    }
    Rectangle {
        objectName:"brushStrokePreview";Layout.fillWidth:true;Layout.preferredHeight:60;color:Theme.selected
        Image {objectName:"brushStrokePreviewImage";anchors.fill:parent;anchors.margins:4;source:root.library.selectedPreview;fillMode:Image.PreserveAspectFit;asynchronous:true}
        Text {anchors.centerIn:parent;text:root.library.previewMessage;font.pixelSize:9;color:Theme.muted;visible:!root.library.selectedPreview.length}
        ToolTip.delay:1000;ToolTip.visible:hover.containsMouse;ToolTip.text:"实际引擎压感笔触 · 预览固定白色及正常模式 · 双击重新生成"
        MouseArea {id:hover;anchors.fill:parent;hoverEnabled:true;onDoubleClicked:root.library.retryPreview()}
    }
    component WetCheck: CompactCheckBox {
        id:wetCheck
        Layout.fillWidth:true;Layout.minimumWidth:0;font.pixelSize:10
        contentItem:Text {text:wetCheck.text;font:wetCheck.font;color:wetCheck.enabled?Theme.text:Theme.disabled;leftPadding:19;verticalAlignment:Text.AlignVCenter;wrapMode:Text.WordWrap}
    }
    component WetField: ParameterSlider {
        property string field
        property string help:""
        unit:"%"
        sliderName:"settingsWet"+field
        value:root.library.wetMix[field]*100
        tooltip:help
        onCommitted:value=>root.setWetField(field,value/100)
    }
    component SmoothingField: ParameterSlider {
        property string field
        property real factor:1
        property string help:""
        sliderName:"settingsSmoothing"+field
        value:root.library.smoothing[field]*factor
        tooltip:help
        onCommitted:value=>{let data=Object.assign({},root.library.smoothing);data[field]=value/factor;root.library.setSmoothing(data)}
    }
    component DualField: ParameterSlider {
        property string field
        property string module:""
        property string prefix:"settingsDual"
        property real factor:100
        unit:"%"
        sliderName:prefix+module+field
        inputName:sliderName+(decimals>0?"Decimal":"Input")
        value:Number((module?root.dualModule(module):root.dual)[field]||0)*factor
        onCommitted:value=>root.setDualField(field,value/factor,module)
        tooltip:"第二笔尖的"+title+"；关闭功能后仍保留设置。"
    }
    component TextureField: ParameterSlider {
        property string field
        property real factor:100
        property string help:""
        sliderName:"settingsTexture"+field
        value:(root.library.texture[field]||0)*factor
        tooltip:help
        onCommitted:value=>root.setTextureField(field,value/factor)
    }
    component TipField: ParameterSlider {
        property string field
        property real factor:100
        property string help:""
        sliderName:"settingsBrush"+field
        value:root.library[field]*factor
        tooltip:help
        onCommitted:value=>root.library[field]=value/factor
    }
    component TransferField: ParameterSlider {
        property string field
        property string help:""
        unit:"%"
        sliderName:"settingsTransfer"+field
        value:root.library.transferJitter[field]*100
        tooltip:help
        onCommitted:value=>root.setTransferJitterField(field,value)
    }
    Timer {id:previewDelay;interval:100;onTriggered:root.library.requestPreview(root.library.selectedId)}
    FileDialog {id:tipDialog;title:"导入图片笔尖";nameFilters:["PNG 图片 (*.png)"];onAccepted:PaintClient.importBrushTip(selectedFile,root.tipInterpretationMode)}
    FileDialog {id:outlineTextureDialog;property bool selfBrush:false;title:"导入描边纹理";nameFilters:["PNG 图片 (*.png)"];onAccepted:PaintClient.importBrushTexture(selectedFile,root.textureInterpretationMode,true,selfBrush)}
    FileDialog {id:textureDialog;title:"导入纹理";nameFilters:["PNG 图片 (*.png)"];onAccepted:PaintClient.importBrushTexture(selectedFile,root.textureInterpretationMode)}
    FileDialog {id:dualDialog;title:"导入第二笔尖";nameFilters:["PNG 图片 (*.png)"];onAccepted:PaintClient.importDualBrushTip(selectedFile,root.dualInterpretationMode)}
    Component.onCompleted:previewDelay.restart()
    Connections {target:root.library;function onSettingsChanged(){if(root.outline.source.model==="texture")root.choosingOutlineTexture=false;root.wetError="";previewDelay.restart();if(root.category===8 && root.inspectedSource!==root.library.sourceResource)root.inspectSource()}}
}
