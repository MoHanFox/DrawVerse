import QtQuick
import DrawVerse 1.0

Item {
    id:root
    required property var client
    required property var canvasSurface
    // The transform cage presents the live boundary; publish the precise
    // transformed selection once final sampling completes.
    visible:!(canvasSurface.forwardPreview && client.transformActive)
    readonly property bool resident:canvasSurface.gpuDisplay && client.documentWidth>0 && client.documentHeight>0 && client.documentWidth<=4096 && client.documentHeight<=4096
    Loader {
        anchors.fill:parent
        active:root.resident && root.client.selectionEnabled
        sourceComponent:Item {
            SelectionOutline {
                id:maskItem
                objectName:"selectionOutline"
                width:root.client.documentWidth;height:root.client.documentHeight
                textureSize:Qt.size(width,height)
                documentSize:Qt.size(width,height);documentRect:Qt.rect(0,0,width,height);zoom:1
                enabledSelection:true;steps:root.client.selectionSteps;maskOnly:true
            }
            ShaderEffectSource {
                id:maskTexture
                sourceItem:maskItem
                sourceRect:Qt.rect(0,0,maskItem.width,maskItem.height)
                textureSize:Qt.size(maskItem.width,maskItem.height)
                hideSource:true;live:true;visible:false;smooth:true;mipmap:true
            }
            ShaderEffect {
                objectName:"residentSelectionAntsShader"
                anchors.fill:parent
                property var mask:maskTexture
                property vector4d documentRect:Qt.vector4d(root.canvasSurface.documentRect.x,root.canvasSurface.documentRect.y,root.canvasSurface.documentRect.width,root.canvasSurface.documentRect.height)
                property vector2d viewportSize:Qt.vector2d(width,height)
                property real phase:maskItem.dashPhase
                fragmentShader:"qrc:/shaders/selection-ants.frag.qsb"
            }
        }
    }
    SelectionOutline {
        objectName:root.resident?"selectionPreviewOutline":"selectionOutline"
        anchors.fill:parent
        enabledSelection:!root.resident && root.client.selectionEnabled
        steps:root.resident?[]:root.client.selectionSteps
        documentSize:Qt.size(root.client.documentWidth,root.client.documentHeight)
        documentRect:root.canvasSurface.documentRect;zoom:root.canvasSurface.zoom
        preview:root.canvasSurface.selectionPreview;previewKind:root.canvasSurface.selectionPreviewKind;previewPoints:root.canvasSurface.selectionPreviewPoints
    }
}
