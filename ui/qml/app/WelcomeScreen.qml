import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../theme"

Item {
    id:root
    property bool ready:false
    property string stage:""
    property string error:""
    signal newCanvasRequested()
    signal openRequested()
    Column {
        anchors.centerIn:parent
        spacing:26
        width:Math.min(520,parent.width-48)
        Label {
            objectName:"welcomeTitle"
            width:parent.width
            text:"欢迎来到DrawVerse"
            horizontalAlignment:Text.AlignHCenter
            color:"#eeeeee";font.pixelSize:28;font.weight:Font.Medium
        }
        Item {
            width:parent.width;height:44
            Label {
                objectName:"welcomeLoadingStatus"
                anchors.centerIn:parent
                visible:!root.ready
                text:root.error.length?root.error:root.stage
                color:root.error.length?"#efb2ab":Theme.muted
                font.pixelSize:12
                width:parent.width;wrapMode:Text.Wrap
                horizontalAlignment:Text.AlignHCenter
                Accessible.role:Accessible.StaticText
                Accessible.name:text
            }
            Row {
                objectName:"welcomeActions"
                anchors.centerIn:parent;spacing:14
                visible:root.ready
                Repeater {
                    model:["新建画布","打开"]
                    delegate:Button {
                        id:actionButton
                        required property int index
                        required property string modelData
                        objectName:index===0?"welcomeNewCanvas":"welcomeOpen"
                        text:modelData
                        width:126;height:34
                        enabled:root.ready
                        hoverEnabled:true
                        contentItem:Text {
                            text:actionButton.text;color:"#ffffff";font.pixelSize:13
                            horizontalAlignment:Text.AlignHCenter;verticalAlignment:Text.AlignVCenter
                        }
                        background:Rectangle {
                            radius:height/2
                            color:actionButton.index===0?(actionButton.down?"#2459a8":actionButton.hovered?"#4288eb":"#3276d6"):(actionButton.down?Theme.hover:actionButton.hovered?"#36393e":"#282b30")
                            border.width:actionButton.index===1 || actionButton.visualFocus?1:0
                            border.color:actionButton.visualFocus?"#9ecbff":"#eeeeee"
                        }
                        onClicked: index===0?root.newCanvasRequested():root.openRequested()
                    }
                }
            }
        }
        Label {
            visible:root.ready && root.error.length>0
            width:parent.width;text:root.error;color:"#efb2ab";font.pixelSize:12
            wrapMode:Text.Wrap;horizontalAlignment:Text.AlignHCenter
        }
    }
}
