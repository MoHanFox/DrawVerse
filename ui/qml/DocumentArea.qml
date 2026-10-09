import QtQuick
import QtQuick.Controls
import QtQuick.Window
import "."
Rectangle {
    id:root
    required property var documents
    required property var presentation
    property string hostId:"main"
    property var groupData:({documents:[],active:""})
    property var currentPane:null
    property var tabsModel:[]
    objectName:"documentArea:"+hostId
    color:"#17191C"
    function refresh(){
        groupData=documents.group(hostId)
        if(tabsModel.join("|")!==groupData.documents.join("|"))tabsModel=groupData.documents
        const pane=presentation.paneFor(groupData.active)
        if(currentPane && currentPane!==pane && currentPane.parent===canvasHost)currentPane.park()
        currentPane=pane
        if(pane && pane.parent!==canvasHost)pane.attach(canvasHost)
    }
    function releaseCanvas(){if(currentPane && currentPane.parent===canvasHost)currentPane.park();currentPane=null}
    Component.onCompleted:{documents.registerTarget(hostId,root);refresh()}
    Component.onDestruction:releaseCanvas()
    Connections {target:root.documents;function onGroupsChanged(){root.refresh()}function onDocumentsChanged(){root.refresh()}}
    Rectangle {
        id:tabs;height:groupData.documents.length?22:0;width:parent.width;color:Theme.strip
        Flickable {
            anchors.left:parent.left;anchors.right:addButton.left;height:parent.height;contentWidth:tabRow.width;clip:true
            Row {
                id:tabRow;spacing:3;height:parent.height
                Repeater {
                    model:root.tabsModel
                    Rectangle {
                        required property string modelData
                        property var docClient:root.documents.client(modelData)
                        objectName:"documentTab:"+modelData
                        width:Math.max(100,Math.min(320,title.implicitWidth+28));height:22
                        color:root.groupData.active===modelData?Theme.background:Theme.tabInactive
                        Text {id:title;x:10;anchors.verticalCenter:parent.verticalCenter;width:parent.width-30;elide:Text.ElideRight;font.pixelSize:9;color:Theme.text;text:docClient?docClient.documentName+(docClient.modified?" *":"")+"  @ "+Math.round((root.presentation.paneFor(modelData)?root.presentation.paneFor(modelData).canvasView.zoom:1)*100)+"% · "+docClient.documentWidth+" × "+docClient.documentHeight:""}
                        MouseArea {
                            objectName:root.presentation.isInitial(modelData)?"canvasGrip":"documentGrip:"+modelData
                            anchors.fill:parent;anchors.rightMargin:20;property point start
                            onPressed:m=>{start=Qt.point(m.x,m.y);root.documents.activate(modelData)}
                            onPositionChanged:m=>{if(pressed && Math.abs(m.x-start.x)+Math.abs(m.y-start.y)>8)Qt.callLater(()=>root.documents.beginDrag(modelData,false))}
                            onDoubleClicked:{if(root.hostId==="main")root.documents.floatDocument(modelData);else root.documents.moveDocument(modelData,"main")}
                        }
                        IconButton {anchors.right:parent.right;width:20;height:22;padding:6;glyph:"close";tooltip:"关闭画布";onClicked:root.documents.requestClose(modelData)}
                    }
                }
            }
        }
        IconButton {id:addButton;anchors.right:parent.right;width:24;height:22;padding:4;glyph:"plus";tooltip:"新建画布";onClicked:root.presentation.newDocumentRequested()}
    }
    Item {id:canvasHost;y:tabs.height;width:parent.width;height:parent.height-y}
    Rectangle {objectName:"documentDockPreview:"+root.hostId;visible:root.documents.dragTarget===root.hostId;width:parent.width;height:28;color:"#2423b5ee";border.color:Theme.accent;border.width:2}
}
