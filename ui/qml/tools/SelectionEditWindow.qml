import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import "../windows"
import "../controls"
import "../theme"
PanelWindow {
    id:root
    objectName:"selectionEditWindow"
    property var controller
    property int kind:0
    property bool waiting:false
    flags:Qt.Dialog | Qt.FramelessWindowHint
    modality:Qt.ApplicationModal
    color:"transparent"
    readonly property int preferredHeight:kind===0?166:kind===4?250:216
    width:330;height:preferredHeight
    minimumWidth:330;maximumWidth:330
    minimumHeight:preferredHeight;maximumHeight:preferredHeight
    title:kind===0?"羽化选区":kind===3?"填充":"描边"
    function openFor(value) {
        if(!controller || !controller.selectionEnabled)return
        kind=value;waiting=false;controller.clearError();amount.value=kind===0?5:1;opacity.value=100;colors.currentIndex=0
        const owner=transientParent
        if(owner){x=owner.x+(owner.width-width)/2;y=owner.y+(owner.height-height)/2}
        show();requestActivate()
    }
    function apply() {
        if(waiting)return
        controller.clearError()
        waiting=controller.selectionPixels(kind,amount.value,colors.currentIndex,opacity.value/100)
    }
    Connections {target:root.controller
        function onStateChanged(){if(root.waiting && !root.controller.layerEditBusy){root.waiting=false;if(!root.controller.lastError.length)root.hide()}}
    }
    Shortcut {sequence:"Escape";enabled:root.visible && !root.waiting;onActivated:root.hide()}
    Shortcut {sequence:"Return";enabled:root.visible && !root.waiting;onActivated:root.apply()}
    onClosing:event=>{event.accepted=false;if(!waiting)hide()}
    Rectangle {x:0;y:28;width:parent.width;height:parent.height-28;color:Theme.surface}
    Rectangle {
        id:bar;width:parent.width;height:28;color:Theme.floatingPanelBar
        Label {anchors.left:parent.left;anchors.leftMargin:12;anchors.verticalCenter:parent.verticalCenter;text:root.title;color:Theme.text;font.pixelSize:9}
        MouseArea {
            anchors.fill:parent;anchors.rightMargin:28
            property point origin
            property point windowOrigin
            onPressed:mouse=>{origin=mapToGlobal(mouse.x,mouse.y);windowOrigin=Qt.point(root.x,root.y)}
            onPositionChanged:mouse=>{if(pressed){const p=mapToGlobal(mouse.x,mouse.y);root.x=Math.round(windowOrigin.x+p.x-origin.x);root.y=Math.round(windowOrigin.y+p.y-origin.y)}}
        }
        ToolButton {id:close;anchors.right:parent.right;width:28;height:28;text:"×";enabled:!root.waiting
            background:Rectangle {color:close.hovered?"#e81123":"transparent"}
            onClicked:root.hide()
        }
    }
    ColumnLayout {
        x:12;y:40;width:parent.width-24;spacing:10;enabled:!root.waiting
        RowLayout {visible:root.kind!==3;Layout.fillWidth:true
            Label {text:root.kind===0?"羽化半径":"宽度";Layout.fillWidth:true}
            SpinBox {id:amount;objectName:"selectionEditAmount";from:1;to:1000;value:5;editable:true;implicitWidth:128;implicitHeight:28}
            Label {text:"像素"}
        }
        RowLayout {visible:root.kind!==0;Layout.fillWidth:true
            Label {text:"颜色";Layout.fillWidth:true}
            CompactComboBox {id:colors;objectName:"selectionEditColor";model:["前景色","白色","黑色"];Layout.preferredWidth:168}
        }
        RowLayout {visible:root.kind!==0;Layout.fillWidth:true
            Label {text:"不透明度";Layout.fillWidth:true}
            SpinBox {id:opacity;objectName:"selectionEditOpacity";from:0;to:100;value:100;editable:true;implicitWidth:128;implicitHeight:28}
            Label {text:"%"}
        }
        Label {visible:root.kind===4;text:"位置：内部";color:Theme.text}
    }
    ColumnLayout {
        anchors.left:parent.left;anchors.right:parent.right;anchors.bottom:parent.bottom;anchors.margins:12;spacing:6
        Label {Layout.fillWidth:true;visible:root.controller && root.controller.lastError.length>0;text:root.controller?root.controller.lastError:"";wrapMode:Text.Wrap;color:"#e49a85"}
        RowLayout {Layout.fillWidth:true
            Item {Layout.fillWidth:true}
            Button {objectName:"selectionEditCancel";text:"取消";implicitWidth:62;implicitHeight:28;enabled:!root.waiting;onClicked:root.hide()}
            Button {objectName:"selectionEditApply";text:root.waiting?"应用中…":"确定";implicitWidth:62;implicitHeight:28;enabled:!root.waiting;onClicked:root.apply()}
        }
    }
}
