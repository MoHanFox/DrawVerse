import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import "../windows"
import "../controls"
import "../brushes"
import "../theme"

PanelWindow {
    id: root
    objectName: "adjustmentWindow"
    flags: Qt.Dialog | Qt.FramelessWindowHint
    modality: Qt.ApplicationModal
    color: "transparent"
    readonly property int preferredHeight: kind===0?530:kind===1?400:305
    width: 430; height: preferredHeight
    minimumWidth: 430; maximumWidth: 430
    minimumHeight: preferredHeight; maximumHeight: preferredHeight
    property int kind: 0
    property int channel: 0
    property var curves: [[[0,0],[1,1]],[[0,0],[1,1]],[[0,0],[1,1]],[[0,0],[1,1]]]
    property var modes: [[],[],[],[]]
    property real black: 0
    property real white: 255
    property real gamma: 1
    property real outputBlack: 0
    property real outputWhite: 255
    property real hue: 0
    property real saturation: 0
    property real lightness: 0
    property bool preview: true
    property bool colorize: false
    property var histogram: PaintClient.adjustmentHistogram
    readonly property var bins: histogram.length===4 ? histogram[channel] : []
    property bool dirty: false
    property int ending: 0
    property bool opened: false
    title: kind===0?"曲线":kind===1?"色阶":"色相 / 饱和度 / 明度"
    function reset() {
        curves=[[[0,0],[1,1]],[[0,0],[1,1]],[[0,0],[1,1]],[[0,0],[1,1]]];modes=[[],[],[],[]]
        black=0;white=255;gamma=1;outputBlack=0;outputWhite=255;hue=0;saturation=0;lightness=0;colorize=false
        dirty=true
    }
    function openFor(value) {
        kind=value;channel=0;preview=true;ending=0;reset()
        if(!PaintClient.beginAdjustment(kind))return
        opened=true
        const owner=transientParent
        if(owner){x=owner.x+(owner.width-width)/2;y=owner.y+(owner.height-height)/2}
        show();requestActivate()
    }
    function process() {
        if(!opened || PaintClient.adjustmentBusy)return
        if(!PaintClient.adjustmentActive){opened=false;hide();return}
        if(ending===3){PaintClient.finishAdjustment(false);return}
        if(dirty) {
            const neutral=!preview && ending!==2
            if(PaintClient.updateAdjustmentControls(kind,{neutral:neutral,curves:curves,modes:modes,levels:[black,white,gamma,outputBlack,outputWhite],hue:hue,saturation:saturation,lightness:lightness,colorize:colorize})){dirty=false;if(ending===2)Qt.callLater(process)}
            return
        }
        if(ending===2) {
            if(PaintClient.adjustmentFailed){ending=0;return}
            PaintClient.finishAdjustment(true)
        }
    }
    onDirtyChanged:if(dirty)Qt.callLater(process)
    onOpenedChanged:if(opened)Qt.callLater(process)
    Shortcut {sequence:"Escape";enabled:root.visible;onActivated:{root.ending=3;root.process()}}
    Shortcut {sequence:"Return";enabled:root.visible && root.ending===0;onActivated:{root.ending=2;root.dirty=true;root.process()}}
    onVisibleChanged:if(visible)body.forceActiveFocus()
    onClosing: event=>{event.accepted=false;ending=3;process()}
    onBlackChanged:dirty=true
    onWhiteChanged:dirty=true
    onGammaChanged:dirty=true
    onOutputBlackChanged:dirty=true
    onOutputWhiteChanged:dirty=true
    onHueChanged:dirty=true
    onSaturationChanged:dirty=true
    onLightnessChanged:dirty=true
    onPreviewChanged:dirty=true
    onColorizeChanged:dirty=true
    Connections {target:PaintClient;property bool wasGpu:false
        function onStateChanged(){const gpu=PaintClient.adjustmentGpuPreview;if(wasGpu && !gpu && root.opened && root.ending===0)root.dirty=true;wasGpu=gpu;if(root.opened && (root.dirty || root.ending!==0))Qt.callLater(root.process)}
    }
    // Like a floating panel, only the title has glass; no docking controller owns this window.
    Rectangle {x:0;y:34;width:parent.width;height:parent.height-34;color:Theme.surface}
    Rectangle {
        id:titleBar;x:0;y:0;width:parent.width;height:34;color:Theme.floatingPanelBar
        Label {anchors.left:parent.left;anchors.leftMargin:12;anchors.verticalCenter:parent.verticalCenter;text:root.title;color:Theme.text;font.pixelSize:13}
        MouseArea {
            objectName:"adjustmentTitleDrag";anchors.fill:parent;anchors.rightMargin:32;preventStealing:true
            property point origin
            property point windowOrigin
            onPressed:mouse=>{origin=mapToGlobal(mouse.x,mouse.y);windowOrigin=Qt.point(root.x,root.y)}
            onPositionChanged:mouse=>{if(pressed){const p=mapToGlobal(mouse.x,mouse.y);root.x=Math.round(windowOrigin.x+p.x-origin.x);root.y=Math.round(windowOrigin.y+p.y-origin.y)}}
        }
        ToolButton {
            id:closeButton;objectName:"adjustmentClose";anchors.right:parent.right;width:32;height:32;text:"×";hoverEnabled:true
            background:Rectangle {objectName:"adjustmentCloseBackground";color:closeButton.hovered?"#e81123":"transparent"}
            contentItem:Text {text:closeButton.text;color:closeButton.hovered?"white":Theme.text;font.pixelSize:16;horizontalAlignment:Text.AlignHCenter;verticalAlignment:Text.AlignVCenter}
            onClicked:{root.ending=3;root.process()}
        }
    }
    ColumnLayout {
        id:body;focus:true
        x:12;y:44;width:parent.width-24;spacing:8
        ColumnLayout {
            Layout.fillWidth:true;enabled:root.ending===0;visible:root.kind===0;spacing:8
            CompactComboBox {objectName:"adjustmentChannel";Layout.fillWidth:true;model:["RGB","红","绿","蓝"];currentIndex:root.channel;onActivated:root.channel=currentIndex}
            AdjustmentCurve {
                Layout.fillWidth:true;Layout.preferredHeight:400
                points:root.curves[root.channel];modes:root.modes[root.channel];bins:root.bins
                lineColor:root.channel===1?"#e98d8d":root.channel===2?"#97d697":root.channel===3?"#9292e9":"#d7d7d7"
                onEdited:(points,modes)=>{let c=root.curves.slice(),m=root.modes.slice();c[root.channel]=points;m[root.channel]=modes;root.curves=c;root.modes=m;root.dirty=true}
            }
        }
        AdjustmentLevels {Layout.fillWidth:true;visible:root.kind===1;enabled:root.ending===0;settings:root;bins:root.histogram.length===4?root.histogram[0]:[]}
        ColumnLayout {
            Layout.fillWidth:true;enabled:root.ending===0;visible:root.kind===2;spacing:9
            AdjustmentColorSlider {Layout.fillWidth:true;name:"adjustHue";caption:"色相";unit:"°";from:root.colorize?0:-180;to:root.colorize?360:180;value:root.hue;spectrum:0;onEdited:number=>root.hue=number}
            AdjustmentColorSlider {Layout.fillWidth:true;name:"adjustSaturation";caption:"饱和度";from:root.colorize?0:-100;value:root.saturation;spectrum:1;tint:Qt.hsla(((root.hue%360)+360)%360/360,1,.5,1);onEdited:number=>root.saturation=number}
            AdjustmentColorSlider {Layout.fillWidth:true;name:"adjustLightness";caption:"明度";value:root.lightness;spectrum:2;tint:Qt.hsla(((root.hue%360)+360)%360/360,1,.5,1);onEdited:number=>root.lightness=number}
            CompactCheckBox {objectName:"adjustmentColorize";text:"着色";checked:root.colorize;onToggled:{root.colorize=checked;root.hue=0;root.saturation=checked?25:0}}
        }
    }
    ColumnLayout {
        anchors.left:parent.left;anchors.right:parent.right;anchors.bottom:parent.bottom;anchors.margins:12;spacing:4
        Label {Layout.fillWidth:true;visible:PaintClient.adjustmentFailed;text:PaintClient.lastError;wrapMode:Text.Wrap;color:"#e49a85"}
        RowLayout {
            Layout.fillWidth:true;spacing:6
            CompactCheckBox {text:"预览";checked:root.preview;enabled:root.ending===0;onToggled:root.preview=checked}
            Item {Layout.fillWidth:true}
            Button {text:"重置";implicitWidth:62;implicitHeight:28;font.pixelSize:12;enabled:root.ending===0;onClicked:root.reset()}
            Button {objectName:"adjustmentCancel";text:"取消";implicitWidth:62;implicitHeight:28;font.pixelSize:12;onClicked:{root.ending=3;root.process()}}
            Button {objectName:"adjustmentApply";text:PaintClient.adjustmentBusy && root.ending===2?"应用中…":"确定";implicitWidth:62;implicitHeight:28;font.pixelSize:12;enabled:root.ending===0;onClicked:{root.ending=2;root.dirty=true;root.process()}}
        }
    }
}
