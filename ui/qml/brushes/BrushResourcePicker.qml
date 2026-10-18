import QtQuick
import QtQuick.Controls
import "../controls"
import "../theme"

CompactComboBox {
    id:root
    property string resource:""
    property int resourceRole:0
    model:PaintClient.brushResources
    currentIndex:model.indexOf(resource)
    displayText:currentIndex<0?"选择已导入图片":"图片 "+(currentIndex+1)+" · "+resource.slice(0,8)
    enabled:!PaintClient.brushFileBusy && count>0
    tooltip:"复用已导入的图片覆盖；先预览再选择。"
    Component.onCompleted:PaintClient.refreshBrushResources()
    Connections {target:PaintClient;function onBrushLibraryReadyChanged(){if(PaintClient.brushLibraryReady)PaintClient.refreshBrushResources()}}
    Connections {target:root.popup;function onAboutToShow(){PaintClient.refreshBrushResources()}}
    onActivated:{const identity=model[currentIndex];if(identity)PaintClient.selectBrushResource(identity,resourceRole);currentIndex=Qt.binding(function(){return root.model.indexOf(root.resource)})}
    delegate:ItemDelegate {
        required property int index
        required property var modelData
        objectName:root.objectName+"Option:"+index
        width:root.popup.availableWidth;height:20;padding:2
        highlighted:root.highlightedIndex===index
        contentItem:Row {
            spacing:5
            BrushResourceThumbnail {objectName:root.objectName+"Thumbnail:"+index;width:18;height:16;resource:modelData}
            Text {text:"图片 "+(index+1)+" · "+modelData.slice(0,8);color:Theme.text;font:root.font;anchors.verticalCenter:parent.verticalCenter}
        }
        background:Rectangle {color:parent.hovered || parent.highlighted?Theme.hover:Theme.surface}
    }
}
