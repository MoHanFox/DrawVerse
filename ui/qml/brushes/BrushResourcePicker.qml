import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../controls"
import "../theme"

ColumnLayout {
    id: root
    property string resource: ""
    property int resourceRole: 0
    property bool expanded: false
    property bool showActions: true
    property string importName: ""
    property string resetName: ""
    readonly property var model: PaintClient.brushResources
    readonly property int count: model.length
    property int currentIndex: model.indexOf(resource)
    signal activated(int index)
    signal importRequested()
    signal resetRequested()
    spacing: 4
    Layout.minimumWidth: 0
    onActivated: index => {if(index>=0 && index<model.length)PaintClient.selectBrushResource(model[index],resourceRole)}
    Component.onCompleted: PaintClient.refreshBrushResources()
    onExpandedChanged: if(expanded)PaintClient.refreshBrushResources()
    RowLayout {
        Layout.fillWidth: true
        spacing: 3
        BrushResourceThumbnail {
            objectName: root.objectName.replace("Picker","Thumbnail")
            resource: root.resource
            Layout.preferredWidth: 64
            Layout.preferredHeight: 64
            MouseArea {anchors.fill:parent;onClicked:root.expanded=!root.expanded}
        }
        IconButton {objectName:root.objectName+"Expand";glyph:"down";rotation:root.expanded?180:0;implicitWidth:18;implicitHeight:64;tooltip:root.expanded?"收起资源":"展开资源";onClicked:root.expanded=!root.expanded}
        Item {Layout.fillWidth:true}
        ColumnLayout {
            visible: root.showActions
            spacing: 2
            IconButton {objectName:root.importName;glyph:"plus";enabled:!PaintClient.brushFileBusy;tooltip:"导入图片";onClicked:root.importRequested()}
            IconButton {objectName:root.resetName;glyph:"minus";enabled:root.resource.length>0 && !PaintClient.brushFileBusy;tooltip:(root.resourceRole===1 || root.resourceRole===3 || root.resourceRole===4)?"移除纹理":"使用程序笔尖";onClicked:root.resetRequested()}
        }
    }
    GridView {
        id: grid
        objectName: root.objectName+"Grid"
        visible: root.expanded
        Layout.fillWidth: true
        Layout.preferredHeight: root.expanded?Math.min(204,Math.ceil(root.count/columns)*cellHeight):0
        readonly property int columns: Math.max(1,Math.floor(width/48))
        cellWidth: width/columns
        cellHeight: cellWidth
        clip: true
        reuseItems: true
        model: root.expanded?root.model:[]
        WheelHandler {
            target: null
            onWheel: event=>{
                grid.cancelFlick()
                const delta=event.pixelDelta.y!==0?event.pixelDelta.y:event.angleDelta.y*.5
                const first=grid.originY,last=first+Math.max(0,grid.contentHeight-grid.height)
                grid.contentY=Math.max(first,Math.min(last,grid.contentY-delta))
                event.accepted=true
            }
        }
        ScrollBar.vertical: ScrollBar {}
        delegate: Item {
            required property int index
            required property string modelData
            width: grid.cellWidth
            height: grid.cellHeight
            BrushResourceThumbnail {
                objectName:root.objectName+"Thumbnail:"+index
                anchors.fill:parent
                anchors.margins:1
                resource:parent.modelData
                border.color:root.resource===resource?Theme.accent:Theme.line
                MouseArea {anchors.fill:parent;enabled:!PaintClient.brushFileBusy;onClicked:root.activated(index)}
            }
        }
    }
    Rectangle {Layout.fillWidth:true;implicitHeight:1;color:Theme.line}
}
