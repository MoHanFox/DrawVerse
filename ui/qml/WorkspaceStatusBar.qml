import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."
Rectangle {
    id:root;objectName:"workspaceStatusBar"
    required property var client
    required property var canvasView
    property bool hasDocument:true
    property real cornerRadius:0
    property string notice:""
    readonly property string toolName:client.selectionTool?(client.selectionTool===2?"椭圆选框":"矩形选框"):client.moveTool?"移动工具":client.eraser?"橡皮擦工具":"画笔工具"
    readonly property string operation:client.closing?"正在关闭核心…":client.fileBusy?"正在读写文件…":notice
    implicitHeight:26;color:Theme.surface;radius:cornerRadius;border.color:Theme.line
    Rectangle {x:1;y:1;width:parent.width-2;height:parent.height/2;color:parent.color}
    RowLayout {
        anchors.fill:parent;anchors.leftMargin:12;anchors.rightMargin:12;spacing:12
        Label {objectName:"statusTool";text:"工具  "+root.toolName;font.pixelSize:8;color:Theme.text}
        Label {objectName:"statusBrush";text:"画笔  "+root.client.brushLibrary.selectedName;font.pixelSize:8;color:Theme.text;elide:Text.ElideRight;Layout.maximumWidth:190}
        Label {objectName:"statusCanvas";text:"画布  "+(root.hasDocument?root.client.documentWidth+" × "+root.client.documentHeight:"—");font.pixelSize:8;color:Theme.text}
        Label {objectName:"statusLayers";text:"图层  "+(root.hasDocument?root.client.layers.filter(l=>!l.mask).length:0);font.pixelSize:8;color:Theme.text}
        Label {text:root.operation;visible:text.length>0;font.pixelSize:8;color:Theme.muted;elide:Text.ElideRight;Layout.fillWidth:visible}
        ToolButton {text:"取消文件任务";font.pixelSize:8;implicitHeight:22;visible:root.client.fileBusy;onClicked:root.client.cancelFile()}
        Item {Layout.fillWidth:true}
        RowLayout {
            spacing:4
            IconButton {objectName:"statusZoomOut";glyph:"zoomOut";tooltip:"缩小";implicitWidth:20;implicitHeight:20;padding:6;enabled:root.hasDocument && root.canvasView;onClicked:root.canvasView.zoomBy(1/1.15)}
            Label {objectName:"statusZoom";text:root.hasDocument && root.canvasView?(root.canvasView.zoom*100).toFixed(1)+"%":"—";font.pixelSize:8;color:Theme.text;horizontalAlignment:Text.AlignHCenter;Layout.preferredWidth:46;background:Rectangle {color:Theme.raised;radius:2}}
            IconButton {objectName:"statusZoomIn";glyph:"zoomIn";tooltip:"放大";implicitWidth:20;implicitHeight:20;padding:6;enabled:root.hasDocument && root.canvasView;onClicked:root.canvasView.zoomBy(1.15)}
            ToolButton {objectName:"statusFit";text:"适配";font.pixelSize:8;implicitHeight:20;padding:3;enabled:root.hasDocument && root.canvasView;onClicked:root.canvasView.fitToView();background:Rectangle {color:parent.hovered?Theme.hover:"transparent";radius:2}}
        }
    }
}
