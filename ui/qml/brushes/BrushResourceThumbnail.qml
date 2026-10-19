import QtQuick
import "../theme"

Rectangle {
    id:root
    property string resource:""
    property string imageSource:""
    implicitWidth:36;implicitHeight:28
    color:Theme.input;border.color:Theme.line;radius:2
    function refresh(){
        if(!resource.length){imageSource="";return}
        const cached=PaintClient.brushResourcePreview(resource)
        if(cached.length)imageSource=cached
        else if(visible && !imageSource.length)PaintClient.requestBrushResourcePreview(resource)
    }
    onResourceChanged:{imageSource="";refresh()}
    onVisibleChanged:if(visible)refresh()
    Component.onCompleted:refresh()
    Connections {
        target:PaintClient
        function onBrushResourcePreviewsChanged(){root.refresh()}
        function onBrushLibraryReadyChanged(){root.refresh()}
    }
    Image {objectName:root.objectName+"Image";anchors.fill:parent;anchors.margins:2;source:root.imageSource;sourceSize:Qt.size(64,64);fillMode:Image.PreserveAspectFit;asynchronous:false}
    Text {anchors.centerIn:parent;visible:!root.imageSource.length;text:"—";color:Theme.muted;font.pixelSize:9}
}
