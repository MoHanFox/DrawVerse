import QtQuick
import QtQuick.Window
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import "."

ColumnLayout {
    id:root
    property var canvasView
    function finishSearch(clear) {
        if(clear) search.clear()
        if(canvasView) {canvasView.forceActiveFocus();canvasView.Window.window.requestActivate()}
        else root.forceActiveFocus()
    }
    property bool includeExportSource:false
    property bool exportAll:false
    property string exportPreset:""
    function beginExport(all) {
        exportAll=!!all
        includeExportSource=false
        exportPreset=library.selectedId
        if(library.presets.some(p=>p.sourceResource.length>0)){includeOriginal.checked=false;sourceOptions.open()}
        else exportDialog.open()
    }
    readonly property var library:PaintClient.brushLibrary
    property var groupedModel:[]
    property string groupedSignature:""
    property string contextGroupId:""
    property string contextGroupName:""
    property string contextPresetId:""
    property string groupParentId:""
    property var groupMenuGroups:[]
    function presetById(id) {
        for(const preset of library.presets) if(preset.id===id) return preset
        return ({})
    }
    function groupContains(groupId,targetId) {
        const groups={};for(const group of groupMenuGroups)groups[group.id]=group
        const group=groups[groupId];if(!group)return false
        for(const item of (group.items||[]))if(item.kind==="group" && (item.id===targetId || groupContains(item.id,targetId)))return true
        return false
    }
    function groupedItems() {
        const presets={}; for(const p of library.presets) presets[p.id]=p
        const organization=library.organization||{}
        const groups={}; for(const g of (organization.groups||[])) groups[g.id]=g
        const query=search.text.trim().toLowerCase()
        const matches={}; for(const p of library.presets) matches[p.id]=!query.length||p.name.toLowerCase().indexOf(query)>=0
        function visit(items,out,depth,parent) {
            for(let index=0;index<(items||[]).length;++index) {
                const item=items[index]
                if(item.kind==="preset") { if(matches[item.id]) out.push({kind:"preset",id:item.id,depth:depth,parent:parent,index:index}) }
                else if(item.kind==="group" && groups[item.id]) {
                    const children=[]; visit(groups[item.id].items,children,depth+1,item.id)
                    if(children.length || !query.length) { out.push({kind:"group",id:item.id,name:groups[item.id].name,depth:depth,parent:parent,index:index}); out.push(...children) }
                }
            }
        }
        if(!organization.roots) return library.presets.filter(p=>matches[p.id]).map((p,index)=>({kind:"preset",id:p.id,depth:0,parent:"",index:index}))
        const result=[]; visit(organization.roots,result,0,""); return result
    }
    function refreshGroupedItems() {
        const signature=JSON.stringify({organization:library.organization||{},presets:library.presets.map(p=>[p.id,p.name]),query:search.text})
        if(signature===groupedSignature) return
        groupedSignature=signature;groupedModel=groupedItems()
    }
    spacing:4
    RowLayout {
        Layout.fillWidth:true
        Label {text:"大小";color:Theme.text;font.pixelSize:9;Layout.fillWidth:true}
        CompactSpinBox {objectName:"libraryBrushSize";from:1;to:512;editable:true;implicitWidth:76;implicitHeight:20;value:Math.round(root.library.radius*2);onValueModified:root.library.radius=value/2}
        Label {text:"px";color:Theme.muted;font.pixelSize:8}
    }
    CompactSlider {Layout.fillWidth:true;from:1;to:512;value:root.library.radius*2;onMoved:root.library.radius=value/2}
    TextField {id:search;objectName:"brushSearch";Layout.fillWidth:true;implicitHeight:22;font.pixelSize:9;placeholderText:"搜索画笔";color:Theme.text;background:Rectangle {color:Theme.input;border.color:Theme.line} onTextChanged:root.refreshGroupedItems();onAccepted:root.finishSearch(false);Keys.onEscapePressed:event=>{root.finishSearch(true);event.accepted=true}}
    RowLayout {
        Layout.fillWidth:true;spacing:2
        Repeater {
            model:[6,12,24,48]
            Button {
                required property int modelData
                Layout.fillWidth:true;implicitWidth:26;implicitHeight:30;padding:1
                onClicked:root.library.radius=modelData/2
                background:Rectangle {color:parent.hovered?Theme.hover:Theme.strip;border.color:Theme.line}
                contentItem:Column {
                    spacing:2
                    Rectangle {width:Math.min(12,modelData/4+3);height:width;radius:width/2;color:Theme.text;anchors.horizontalCenter:parent.horizontalCenter}
                    Text {text:modelData;font.pixelSize:8;color:Theme.muted;anchors.horizontalCenter:parent.horizontalCenter}
                }
            }
        }
    }
    Label {objectName:"brushImportSummary";textFormat:Text.PlainText;text:PaintClient.brushImportMessage;visible:text.length>0;Layout.fillWidth:true;wrapMode:Text.WordWrap;font.pixelSize:9;color:Theme.muted}
    RowLayout {Icon {name:"folder";width:14;height:14} Label {text:"基础与自定义画笔";font.pixelSize:9;color:Theme.text}}
    ListView {
        objectName:"brushPresetList";Layout.fillWidth:true;Layout.fillHeight:true;clip:true;spacing:2
        id:presetList;model:root.groupedModel
        delegate:Rectangle {
            required property var modelData
            objectName:modelData.kind==="group"?"brushGroup:"+modelData.id:"brushPreset:"+modelData.id
            width:ListView.view.width;height:modelData.kind==="group"?30:58
            color:modelData.kind==="group"?Theme.strip:root.library.selectedId===modelData.id?Theme.selected:Theme.surface
            border.color:Theme.line
            property int dropPlacement:0
            function updateDrop(d){dropPlacement=modelData.kind==="group" && d.y>8 && d.y<height-8?0:d.y<height/2?1:2}
            visible:modelData.kind==="group" || root.presetById(modelData.id).id!==undefined
            Image {visible:modelData.kind!=="group";anchors.left:parent.left;anchors.right:parent.right;anchors.top:parent.top;anchors.leftMargin:4+modelData.depth*14;anchors.rightMargin:4;anchors.topMargin:4;height:35;source:modelData.kind==="group"?"":root.presetById(modelData.id).preview;fillMode:Image.PreserveAspectFit;asynchronous:true}
            Text {visible:modelData.kind==="group";x:8+modelData.depth*14;y:0;width:parent.width-x-8;height:30;textFormat:Text.PlainText;text:"▾  "+modelData.name;color:Theme.text;font.pixelSize:10;verticalAlignment:Text.AlignVCenter;elide:Text.ElideRight}
            Text {visible:modelData.kind!=="group";x:6+modelData.depth*14;y:40;width:parent.width-x-6;textFormat:Text.PlainText;text:root.presetById(modelData.id).name||"";color:Theme.text;font.pixelSize:9;elide:Text.ElideRight}
            MouseArea {
                anchors.fill:parent;acceptedButtons:Qt.LeftButton|Qt.RightButton
                property point origin
                property bool dragging:false
                property bool suppressClick:false
                onPressed:function(mouse){origin=Qt.point(mouse.x,mouse.y);dragging=false;suppressClick=false}
                onPositionChanged:function(mouse){if(pressed && (pressedButtons&Qt.LeftButton) && !dragging && Math.abs(mouse.x-origin.x)+Math.abs(mouse.y-origin.y)>8){dragging=true;suppressClick=true;PaintClient.beginBrushDrag(modelData.kind+":"+modelData.id)}}
                onReleased:function(){dragging=false}
                onClicked:function(mouse) {
                    if(suppressClick){suppressClick=false;return}
                    if(mouse.button===Qt.RightButton) {
                        if(modelData.kind==="group") {root.contextGroupId=modelData.id;root.contextGroupName=modelData.name;groupMenu.popup()}
                        else {root.contextPresetId=modelData.id;root.library.select(modelData.id);presetMenu.popup()}
                    } else if(modelData.kind!=="group") root.library.select(modelData.id)
                }
            }
            DropArea {
                id:brushDrop;objectName:"brushDrop:"+modelData.id
                anchors.fill:parent;keys:["application/x-drawverse-brush"]
                onEntered:function(drop){parent.updateDrop(drop)}
                onPositionChanged:function(drop){parent.updateDrop(drop)}
                onDropped:function(drop){
                    parent.updateDrop(drop)
                    const payload=drop.getDataAsString("application/x-drawverse-brush"),separator=payload.indexOf(":")
                    if(separator<=0)return
                    const kind=payload.slice(0,separator),id=payload.slice(separator+1)
                    let parentId=modelData.parent,index=modelData.index
                    if(modelData.kind==="group" && parent.dropPlacement===0){parentId=modelData.id;index=-1}
                    else if(parent.dropPlacement===2)index=modelData.index+1
                    if(kind==="preset")root.library.movePresetToGroupAt(id,parentId,index)
                    else if(kind==="group")root.library.moveGroupToGroupAt(id,parentId,index)
                    drop.acceptProposedAction()
                }
            }
            Rectangle {anchors.fill:parent;border.width:2;border.color:Theme.accent;visible:brushDrop.containsDrag && dropPlacement===0}
            Rectangle {anchors.left:parent.left;anchors.right:parent.right;height:2;y:dropPlacement===1?0:parent.height-2;color:Theme.accent;visible:brushDrop.containsDrag && dropPlacement!==0}
            Component.onCompleted:if(modelData.kind!=="group")root.library.requestPreview(modelData.id)
        }
        ScrollBar.vertical:ScrollBar {width:6}
        footer:DropArea {
            objectName:"brushRootDrop";width:ListView.view.width;height:24;keys:["application/x-drawverse-brush"]
            Rectangle {anchors.fill:parent;color:parent.containsDrag?Theme.selected:Theme.strip;border.color:Theme.line}
            Label {anchors.centerIn:parent;text:"拖到这里移到画笔库根目录";color:Theme.muted;font.pixelSize:9}
            onDropped:function(drop){
                const payload=drop.getDataAsString("application/x-drawverse-brush"),separator=payload.indexOf(":")
                if(separator<=0)return
                const kind=payload.slice(0,separator),id=payload.slice(separator+1)
                if(kind==="preset")root.library.movePresetToGroupAt(id,"",-1)
                else if(kind==="group")root.library.moveGroupToGroupAt(id,"",-1)
                drop.acceptProposedAction()
            }
        }
    }
    RowLayout {
        Item {Layout.fillWidth:true}
        IconButton {objectName:"importBrushPackage";glyph:"folder";tooltip:"导入 VBR / ABR 笔刷包";implicitHeight:22;enabled:!PaintClient.brushFileBusy;onClicked:importDialog.open()}
        IconButton {objectName:"exportBrushPackage";glyph:"save";tooltip:"导出当前笔刷为 VBR";implicitHeight:22;enabled:!PaintClient.brushFileBusy;onClicked:root.beginExport()}
        IconButton {objectName:"exportAllBrushPackages";glyph:"save";tooltip:"导出全部画笔与分组为 VBR";implicitHeight:22;enabled:!PaintClient.brushFileBusy;onClicked:root.beginExport(true)}
        IconButton {glyph:"plus";tooltip:"保存当前设置为新画笔";implicitHeight:22;onClicked:saveDialog.open()}
        IconButton {objectName:"createBrushGroup";glyph:"folder";tooltip:"新建画笔组";implicitHeight:22;onClicked:{root.groupParentId="";groupName.text="";groupDialog.title="新建画笔组";groupDialog.open()}}
        IconButton {glyph:"trash";tooltip:"删除自定义画笔";implicitHeight:22;enabled:root.library.presets.some(p=>p.id===root.library.selectedId && !p.builtin);onClicked:root.library.remove(root.library.selectedId)}
    }
    Menu {
        id:groupMenu
        MenuItem {text:"新建子组";onTriggered:{root.groupParentId=root.contextGroupId;groupName.text="";groupDialog.title="新建子组";groupDialog.open()}}
        MenuItem {text:"重命名";onTriggered:{root.groupParentId="";groupName.text=root.contextGroupName;groupDialog.title="重命名组";groupDialog.open()}}
        MenuSeparator {}
        MenuItem {text:"移动到根目录";onTriggered:root.library.moveGroupToGroup(root.contextGroupId,"")}
        Menu {
            title:"移动到组"
            Repeater {
                model:root.groupMenuGroups
                MenuItem {
                    required property var modelData
                    text:modelData.name
                    visible:modelData.id!==root.contextGroupId && !root.groupContains(root.contextGroupId,modelData.id)
                    onTriggered:root.library.moveGroupToGroup(root.contextGroupId,modelData.id)
                }
            }
        }
        MenuSeparator {}
        MenuItem {text:"解散组";onTriggered:root.library.ungroup(root.contextGroupId)}
        MenuItem {text:"删除组及内容";onTriggered:root.library.removeGroup(root.contextGroupId,true)}
    }
    Menu {
        id:presetMenu;title:"移动到组"
        MenuItem {text:"画笔库根目录";onTriggered:root.library.movePresetToGroup(root.contextPresetId,"")}
        Repeater {
            model:root.groupMenuGroups
            MenuItem {
                required property var modelData
                text:modelData.name
                onTriggered:root.library.movePresetToGroup(root.contextPresetId,modelData.id)
            }
        }
    }
    Dialog {
        id:saveDialog;title:"保存画笔";modal:true;standardButtons:Dialog.Ok|Dialog.Cancel
        anchors.centerIn:Overlay.overlay
        TextField {id:brushName;placeholderText:"画笔名称";maximumLength:80;text:root.library.selectedName+" 副本"}
        onAccepted:root.library.saveCopy(brushName.text)
    }
    Dialog {
        id:groupDialog;objectName:"brushGroupDialog";title:"新建画笔组";modal:true;standardButtons:Dialog.Ok|Dialog.Cancel
        anchors.centerIn:Overlay.overlay
        contentItem:TextField {id:groupName;objectName:"brushGroupName";placeholderText:"组名称";maximumLength:80;selectByMouse:true}
        onAccepted: {
            if(title==="重命名组") root.library.renameGroup(root.contextGroupId,groupName.text)
            else root.library.createGroup(groupName.text,root.groupParentId)
        }
    }
    Dialog {
        id:sourceOptions;objectName:"brushSourceExportOptions";title:"导出来源范围";modal:true;anchors.centerIn:Overlay.overlay;width:320
        contentItem:ColumnLayout {
            spacing:8
            Label {text:root.exportAll?"导出范围包含 ABR 来源。原始包可能包含其他预设和资源；选择后将完整包含在 VBR 中。":"该笔刷包含 ABR 来源。原始包可能包含其他预设和资源；选择后将完整包含在 VBR 中。";textFormat:Text.PlainText;Layout.fillWidth:true;wrapMode:Text.WordWrap;font.pixelSize:10;color:Theme.text}
            CompactCheckBox {id:includeOriginal;objectName:"includeOriginalAbr";text:"包含原始 ABR 包";font.pixelSize:10;tooltip:"包含全部原始字节以及导入时参数和兼容报告；原始包可能包含其他预设和资源。"}
        }
        footer:RowLayout {
            Item {Layout.fillWidth:true}
            Button {text:"取消";implicitHeight:24;onClicked:sourceOptions.reject()}
            Button {objectName:"confirmSourceExport";text:"继续";implicitHeight:24;enabled:includeOriginal.checked;onClicked:{root.includeExportSource=true;sourceOptions.accept();exportDialog.open()}}
        }
    }
    FileDialog {id:importDialog;objectName:"brushImportDialog";title:"导入笔刷包";nameFilters:["笔刷包 (*.vbr *.abr)","DrawVerse 笔刷 (*.vbr)","Photoshop 笔刷 (*.abr)"];onAccepted:PaintClient.importBrushes(selectedFile)}
    FileDialog {id:exportDialog;title:root.exportAll?"导出全部画笔":"导出当前笔刷";fileMode:FileDialog.SaveFile;defaultSuffix:"vbr";nameFilters:["DrawVerse 笔刷 (*.vbr)"];onAccepted:root.exportAll?PaintClient.exportAllBrushes(selectedFile,root.includeExportSource):PaintClient.exportBrush(selectedFile,root.includeExportSource,root.exportPreset)}
    Timer {id:previewDelay;interval:100;onTriggered:root.library.requestPreview(root.library.selectedId)}
    Connections {target:root.library;function onSettingsChanged(){previewDelay.restart()}function onPresetsChanged(){root.groupMenuGroups=(root.library.organization&&root.library.organization.groups)||[];root.refreshGroupedItems()}}
    Component.onCompleted:{root.groupMenuGroups=(root.library.organization&&root.library.organization.groups)||[];root.refreshGroupedItems()}
}
