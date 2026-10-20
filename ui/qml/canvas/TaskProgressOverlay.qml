import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../theme"

Item {
    id: root
    objectName: "longTaskProgress"
    required property var controller
    readonly property var progress: controller.computationProgress
    visible: controller.longTaskVisible
    MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons }
    Rectangle {
        anchors.centerIn: parent
        width: Math.min(340,parent.width-24);height:110
        color: Theme.background; radius:6; border.color: Theme.line
        ColumnLayout {
            anchors.fill:parent;anchors.margins:14;spacing:8
            Label { text: ["","正在应用变换","正在应用液化","正在翻转图像"][root.progress.kind||0];color:Theme.text;font.pixelSize:12 }
            Label {objectName:"longTaskPhase";text:["准备计算…","生成图像","更新选区","整理结果","保存撤销记录…"][root.progress.phase||0];color:Theme.muted;font.pixelSize:10}
            RowLayout {
                Layout.fillWidth:true
                ProgressBar {objectName:"longTaskBar";Layout.fillWidth:true;from:0;to:1;indeterminate:!(root.progress.total>0);value:root.progress.total>0?root.progress.completed/root.progress.total:0}
                Label {visible:root.progress.total>0;text:"当前阶段 "+Math.floor(100*(root.progress.completed||0)/Math.max(1,root.progress.total||0))+"%";color:Theme.muted;font.pixelSize:9}
            }
        }
    }
}
