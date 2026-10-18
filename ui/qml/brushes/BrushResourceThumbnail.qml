import QtQuick
import "../theme"

Rectangle {
    id:root
    property string resource:""
    readonly property string imageSource: {
        PaintClient.brushResourcePreviewRevision
        return PaintClient.brushResourcePreview(resource)
    }
    implicitWidth:36;implicitHeight:28
    color:Theme.input;border.color:Theme.line;radius:2
    function request(){if(visible && resource.length)PaintClient.requestBrushResourcePreview(resource)}
    onResourceChanged:request()
    onVisibleChanged:request()
    Component.onCompleted:request()
    Connections {
        target:PaintClient
        function onBrushResourcePreviewsChanged(){root.request()}
        function onBrushLibraryReadyChanged(){root.request()}
    }
    Image {objectName:root.objectName+"Image";anchors.fill:parent;anchors.margins:2;source:root.imageSource;sourceSize:Qt.size(64,64);fillMode:Image.PreserveAspectFit;asynchronous:true}
    Text {anchors.centerIn:parent;visible:!root.imageSource.length;text:"—";color:Theme.muted;font.pixelSize:9}
}
