import QtQuick
import QtQuick.Window
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import "../controls"
import "../menus"
import "../theme"

ColumnLayout {
    id:root
    objectName:"brushLibraryPanel"
    property bool draggingBrush:false
    property string draggedBrushKey:""
    property string renamePresetId:""
    function beginRenamePreset(id){const preset=presetById(id);if(!preset.id)return;renamePresetId=id;renameName.text=preset.name;renameDialog.open()}
    property var dragHovered:null
    property real dragViewportY:-1
    property var canvasView
    property var collapsedGroups:[]
    function isGroupCollapsed(id) { return collapsedGroups.indexOf(id)>=0 }
    function toggleGroup(id) {
        const next=collapsedGroups.slice(),at=next.indexOf(id)
        if(at>=0) next.splice(at,1)
        else next.push(id)
        collapsedGroups=next
        refreshGroupedItems()
    }
    function finishSearch(clear) {
        if(clear) search.clear()
        if(canvasView) {canvasView.forceActiveFocus();canvasView.Window.window.requestActivate()}
        else root.forceActiveFocus()
    }
    property bool includeExportSource:false
    property bool groupSelectedMode:false
    property string selectionAnchor:""
    function beginSelectionExport() {
        if(library.selectedBrushItems.length===0)return
        includeExportSource=false
        if(library.brushSelectionHasSources){includeOriginal.checked=false;sourceOptions.open()}
        else exportDialog.open()
    }
    function itemKey(kind,id){return kind+":"+id}
    function isBrushItemSelected(kind,id) {
        const items=library.selectedBrushItems
        if(items.length===0)return kind==="preset" && id===library.selectedId
        return items.some(item=>item.kind===kind && item.id===id)
    }
    function handleBrushClick(modelData,modifiers) {
        const key=itemKey(modelData.kind,modelData.id),items=groupedModel
        const shift=(modifiers & Qt.ShiftModifier)!==0,control=(modifiers & (Qt.ControlModifier|Qt.MetaModifier))!==0
        if(shift && selectionAnchor.length>0) {
            const from=items.findIndex(item=>itemKey(item.kind,item.id)===selectionAnchor)
            const to=items.findIndex(item=>itemKey(item.kind,item.id)===key)
            if(from>=0 && to>=0) {
                const range=items.slice(Math.min(from,to),Math.max(from,to)+1).map(item=>({kind:item.kind,id:item.id}))
                library.selectBrushItems(range,modelData.kind==="preset"?modelData.id:"")
                return
            }
        }
        if(control) {
            const selected=library.selectedBrushItems.map(item=>({kind:item.kind,id:item.id}))
            const index=selected.findIndex(item=>itemKey(item.kind,item.id)===key)
            if(index>=0)selected.splice(index,1)
            else selected.push({kind:modelData.kind,id:modelData.id})
            library.selectBrushItems(selected,modelData.kind==="preset"?modelData.id:"")
            selectionAnchor=key
            return
        }
        selectionAnchor=key
        library.selectBrushItems([{kind:modelData.kind,id:modelData.id}],modelData.kind==="preset"?modelData.id:"")
    }
    function openSelectedGroupDialog() {
        if(library.selectedBrushItems.length===0)return
        groupSelectedMode=true;groupParentId="";groupName.text="";groupDialog.title="将选中项创建为组";groupDialog.open()
    }
    readonly property var library:PaintClient.brushLibrary
    property var groupedModel:[]
    property string groupedSignature:""
    property string contextGroupId:""
    property string contextGroupName:""
    property string contextPresetId:""
    property string groupParentId:""
    property var groupMenuGroups:[]
    property var presetLookup: ({})
    property var visiblePreviewIds: []
    function cancelVisiblePreviews() {
        if(library)for(const id of visiblePreviewIds)library.cancelPreview(id)
        visiblePreviewIds=[]
    }
    function requestVisiblePreviews() {
        if(!visible || presetList.height<=0)return
        const ids=[]
        for(const row of presetList.contentItem.children) {
            if(!row.modelData || row.modelData.kind!=="preset")continue
            if(row.y+row.height<=presetList.contentY || row.y>=presetList.contentY+presetList.height)continue
            ids.push(row.modelData.id)
        }
        visiblePreviewIds=ids
        for(const id of ids)library.requestPreview(id)
    }
    function refreshPresets() {
        const lookup={}
        for(const preset of library.presets)lookup[preset.id]=Object.assign({},preset)
        presetLookup=lookup
    }
    function presetById(id) { return presetLookup[id]||({}) }
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
                    const groupRow={kind:"group",id:item.id,name:groups[item.id].name,depth:depth,parent:parent,index:index}
                    if(!query.length && root.isGroupCollapsed(item.id)) { out.push(groupRow); continue }
                    const children=[]; visit(groups[item.id].items,children,depth+1,item.id)
                    if(children.length || !query.length) { out.push(groupRow); out.push(...children) }
                }
            }
        }
        if(!organization.roots) return library.presets.filter(p=>matches[p.id]).map((p,index)=>({kind:"preset",id:p.id,depth:0,parent:"",index:index}))
        const result=[]; visit(organization.roots,result,0,""); return result
    }
    function refreshGroupedItems() {
        const signature=JSON.stringify({organization:library.organization||{},presets:library.presets.map(p=>[p.id,p.name]),query:search.text,collapsed:collapsedGroups})
        if(signature===groupedSignature) return
        const keepY=search.text.trim().length>0?0:presetList.contentY
        groupedSignature=signature;groupedModel=groupedItems()
        Qt.callLater(()=>{presetList.contentY=Math.min(keepY,Math.max(0,presetList.contentHeight-presetList.height))})
    }
    function dropToRoot(drop) {
        const payload=drop.getDataAsString("application/x-drawverse-brush"),separator=payload.indexOf(":")
        if(separator<=0)return false
        const kind=payload.slice(0,separator),id=payload.slice(separator+1)
        if(kind!=="preset" && kind!=="group")return false
        if(kind==="preset")root.library.movePresetToGroupAt(id,"",-1)
        else root.library.moveGroupToGroupAt(id,"",-1)
        drop.acceptProposedAction()
        return true
    }
    spacing:4
    RowLayout {
        Layout.fillWidth:true
        Label {text:"大小";color:Theme.text;font.pixelSize:9;Layout.fillWidth:true}
        CompactSpinBox {objectName:"libraryBrushSize";from:1;to:2000;editable:true;implicitWidth:76;implicitHeight:20;value:Math.round(root.library.radius*2);onValueModified:root.library.radius=value/2}
        Label {text:"px";color:Theme.muted;font.pixelSize:8}
    }
    CompactSlider {Layout.fillWidth:true;from:1;to:2000;value:root.library.radius*2;onMoved:root.library.radius=value/2}
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
        id:presetList;interactive:!root.draggingBrush;model:root.groupedModel
        onContentYChanged:{root.cancelVisiblePreviews();visiblePreviewDelay.restart()}
        onHeightChanged:visiblePreviewDelay.restart()
        property var panel:root
        delegate:Rectangle {
            id:brushRow
            required property var modelData
            property var panel:root
            objectName:modelData.kind==="group"?"brushGroup:"+modelData.id:"brushPreset:"+modelData.id
            width:ListView.view.width;height:modelData.kind==="group"?30:58
            color:root.isBrushItemSelected(modelData.kind,modelData.id)?Theme.selected:modelData.kind==="group"?Theme.strip:Theme.surface
            border.color:Theme.line
            property int dropPlacement:0
            property point dropPoint
            function updateDrop(d){
                dropPoint=Qt.point(d.x,d.y);panel.dragHovered=brushRow;panel.dragViewportY=brushRow.mapToItem(presetList,d.x,d.y).y
                if(modelData.kind==="group") {
                    const expanded=!panel.isGroupCollapsed(modelData.id)
                    dropPlacement=expanded?(d.y<height/3?1:0):(d.y>height/3 && d.y<height*2/3?0:d.y<height/2?1:2)
                } else dropPlacement=d.y<height/2?1:2
            }
            visible:modelData.kind==="group" || root.presetById(modelData.id).id!==undefined
            Image {id:rowPreview;visible:modelData.kind!=="group";anchors.left:parent.left;anchors.right:parent.right;anchors.top:parent.top;anchors.leftMargin:4+modelData.depth*14;anchors.rightMargin:4;anchors.topMargin:4;height:35;source:modelData.kind==="group"?"":root.presetById(modelData.id).preview;fillMode:Image.PreserveAspectFit;asynchronous:true}
            Text {visible:modelData.kind==="group";x:8+modelData.depth*14;y:0;width:parent.width-x-8;height:30;textFormat:Text.PlainText;text:(root.isGroupCollapsed(modelData.id)?"▸  ":"▾  ")+modelData.name;color:Theme.text;font.pixelSize:10;verticalAlignment:Text.AlignVCenter;elide:Text.ElideRight}
            Text {visible:modelData.kind!=="group";x:6+modelData.depth*14;y:40;width:parent.width-x-6;textFormat:Text.PlainText;text:root.presetById(modelData.id).name||"";color:Theme.text;font.pixelSize:9;elide:Text.ElideRight}
            MouseArea {
                anchors.fill:parent;acceptedButtons:Qt.LeftButton|Qt.RightButton
                preventStealing:true
                property point origin
                property bool dragging:false
                property bool suppressClick:false
                onPressed:function(mouse){origin=Qt.point(mouse.x,mouse.y);dragging=false;suppressClick=false;brushRow.panel.draggingBrush=false}
                onPositionChanged:function(mouse){if(pressed && (pressedButtons&Qt.LeftButton) && !dragging && Math.abs(mouse.x-origin.x)+Math.abs(mouse.y-origin.y)>8){dragging=true;suppressClick=true;brushRow.panel.draggingBrush=true;brushRow.panel.draggedBrushKey=modelData.kind+":"+modelData.id;PaintClient.beginBrushDrag(brushRow.panel.draggedBrushKey);brushRow.panel.draggedBrushKey="";brushRow.panel.draggingBrush=false;brushRow.panel.draggedBrushKey="";brushRow.panel.dragViewportY=-1}}
                onReleased:function(){dragging=false;brushRow.panel.draggingBrush=false;brushRow.panel.draggedBrushKey="";brushRow.panel.dragViewportY=-1}
                onCanceled:function(){dragging=false;brushRow.panel.draggingBrush=false;brushRow.panel.draggedBrushKey="";brushRow.panel.dragViewportY=-1}
                onDoubleClicked:function(mouse){if(mouse.button===Qt.LeftButton && modelData.kind==="preset" && !suppressClick)brushRow.panel.beginRenamePreset(modelData.id)}
                onClicked:function(mouse) {
                    if(suppressClick){suppressClick=false;return}
                    if(mouse.button===Qt.RightButton) {
                        if(!brushRow.panel.isBrushItemSelected(modelData.kind,modelData.id))brushRow.panel.handleBrushClick(modelData,Qt.NoModifier)
                        if(modelData.kind==="group") {brushRow.panel.contextGroupId=modelData.id;brushRow.panel.contextGroupName=modelData.name;groupMenu.popup()}
                        else {brushRow.panel.contextPresetId=modelData.id;presetMenu.popup()}
                    } else {
                        brushRow.panel.handleBrushClick(modelData,mouse.modifiers)
                        if(modelData.kind==="group" && !(mouse.modifiers & (Qt.ControlModifier|Qt.MetaModifier|Qt.ShiftModifier)))brushRow.panel.toggleGroup(modelData.id)
                    }
                }
            }
            DropArea {
                id:brushDrop;objectName:"brushDrop:"+modelData.id
                anchors.fill:parent;keys:["application/x-drawverse-brush"];enabled:brushRow.panel.draggedBrushKey!==modelData.kind+":"+modelData.id
                onEntered:function(drop){brushRow.updateDrop(drop)}
                onPositionChanged:function(drop){brushRow.updateDrop(drop)}
                onExited:{if(brushRow.panel.dragHovered===brushRow)brushRow.panel.dragHovered=null}
                onDropped:function(drop){
                    brushRow.updateDrop(drop)
                    brushRow.panel.dragHovered=null
                    brushRow.panel.dragViewportY=-1
                    const payload=drop.getDataAsString("application/x-drawverse-brush"),separator=payload.indexOf(":")
                    if(separator<=0)return
                    const kind=payload.slice(0,separator),id=payload.slice(separator+1)
                    if(kind===modelData.kind && id===modelData.id)return
                    let parentId=modelData.parent,index=modelData.index
                    if(modelData.kind==="group" && brushRow.dropPlacement===0){parentId=modelData.id;index=0}
                    else if(brushRow.dropPlacement===2)index=modelData.index+1
                    if(kind==="preset")brushRow.panel.library.movePresetToGroupAt(id,parentId,index)
                    else if(kind==="group")brushRow.panel.library.moveGroupToGroupAt(id,parentId,index)
                    drop.acceptProposedAction()
                }
            }
            Rectangle {anchors.fill:parent;border.width:2;border.color:Theme.accent;visible:brushDrop.containsDrag && dropPlacement===0}
            Rectangle {anchors.left:parent.left;anchors.right:parent.right;height:2;y:dropPlacement===1?0:parent.height-2;color:Theme.accent;visible:brushDrop.containsDrag && dropPlacement!==0}
            Connections {
                target:brushRow.panel.library
                function onPreviewChanged(id,image){if(id===brushRow.modelData.id)rowPreview.source=image}
                function onPresetChanged(id){if(id===brushRow.modelData.id)rowPreview.source=brushRow.panel.library.presetInfo(id).preview||""}
                function onPresetsChanged(){Qt.callLater(()=>{rowPreview.source=brushRow.modelData.kind==="group"?"":brushRow.panel.presetById(brushRow.modelData.id).preview||""})}
            }
            Component.onCompleted:visiblePreviewDelay.restart()
        }
        WheelHandler {
            objectName:"brushListWheel"
            enabled:root.draggingBrush
            onWheel:function(event){const step=event.pixelDelta.y!==0?event.pixelDelta.y:event.angleDelta.y*.5;presetList.contentY=Math.max(0,Math.min(Math.max(0,presetList.contentHeight-presetList.height),presetList.contentY-step));event.accepted=true}
        }
        ScrollBar.vertical:ScrollBar {width:6}
        DropArea {
            objectName:"brushEmptyDrop";x:0;y:Math.max(0,presetList.contentHeight-presetList.contentY)
            width:presetList.width;height:Math.max(0,presetList.height-y)
            keys:["application/x-drawverse-brush"]
            Rectangle {anchors.fill:parent;color:parent.containsDrag?Theme.selected:"transparent"}
            onDropped:function(drop){presetList.panel.dropToRoot(drop)}
        }
        footer:DropArea {
            objectName:"brushRootDrop";width:ListView.view.width;height:24;keys:["application/x-drawverse-brush"]
            Rectangle {anchors.fill:parent;color:parent.containsDrag?Theme.selected:Theme.strip;border.color:Theme.line}
            Label {anchors.centerIn:parent;text:"拖到这里移到画笔库根目录";color:Theme.muted;font.pixelSize:9}
            onDropped:function(drop){presetList.panel.dropToRoot(drop)}
        }
    }
    Timer {
        interval:60;repeat:true;running:root.draggingBrush && root.dragViewportY>=0
        onTriggered:{const delta=root.dragViewportY<20?-16:root.dragViewportY>presetList.height-20?16:0;if(delta)presetList.contentY=Math.max(0,Math.min(Math.max(0,presetList.contentHeight-presetList.height),presetList.contentY+delta))}
    }
    RowLayout {
        Item {Layout.fillWidth:true}
        IconButton {objectName:"importBrushPackage";glyph:"folder";tooltip:"导入 VBR / ABR 笔刷包";implicitHeight:22;enabled:!PaintClient.brushFileBusy;onClicked:importDialog.open()}
        IconButton {glyph:"plus";tooltip:"保存当前设置为新画笔";implicitHeight:22;onClicked:saveDialog.open()}
        IconButton {objectName:"createBrushGroup";glyph:"folder";tooltip:root.library.selectedBrushItems.length>0?"将选中项创建为组":"新建画笔组";implicitHeight:22;onClicked:{if(root.library.selectedBrushItems.length>0)root.openSelectedGroupDialog();else{root.groupSelectedMode=false;root.groupParentId="";groupName.text="";groupDialog.title="新建画笔组";groupDialog.open()}}}
        IconButton {glyph:"trash";tooltip:"删除自定义画笔";implicitHeight:22;enabled:root.library.presets.some(p=>p.id===root.library.selectedId && !p.builtin);onClicked:root.library.remove(root.library.selectedId)}
    }
    GlassMenu {
        id:groupMenu
        objectName:"brushGroupContextMenu"
        topLevel:true
        Instantiator {
            model:root.library.selectedBrushItems.length>1?[0]:[]
            delegate:GlassMenuItem {text:"创建分组";onTriggered:root.openSelectedGroupDialog()}
            onObjectAdded:(index,object)=>groupMenu.insertItem(index,object)
            onObjectRemoved:(index,object)=>groupMenu.removeItem(object)
        }
        GlassMenuItem {text:"新建子组";onTriggered:{root.groupSelectedMode=false;root.groupParentId=root.contextGroupId;groupName.text="";groupDialog.title="新建子组";groupDialog.open()}}
        GlassMenuItem {text:"重命名";onTriggered:{root.groupSelectedMode=false;root.groupParentId="";groupName.text=root.contextGroupName;groupDialog.title="重命名组";groupDialog.open()}}
        GlassMenuItem {text:root.library.selectedBrushItems.length>1?"导出选中笔刷":"导出该组笔刷";onTriggered:root.beginSelectionExport()}
        MenuSeparator {}
        GlassMenuItem {text:"移动到根目录";onTriggered:root.library.moveGroupToGroup(root.contextGroupId,"")}
        GlassMenu {
            title:"移动到组"
            cascadeParent:groupMenu
            objectName:"brushGroupMoveToGroup"
            enabled:root.groupMenuGroups.length>0
            Repeater {
                model:root.groupMenuGroups
                GlassMenuItem {
                    required property var modelData
                    text:modelData.name
                    visible:modelData.id!==root.contextGroupId && !root.groupContains(root.contextGroupId,modelData.id)
                    onTriggered:root.library.moveGroupToGroup(root.contextGroupId,modelData.id)
                }
            }
        }
        MenuSeparator {}
        GlassMenuItem {text:"解散组";onTriggered:root.library.ungroup(root.contextGroupId)}
        GlassMenuItem {text:"删除组及内容";onTriggered:root.library.removeGroup(root.contextGroupId,true)}
    }
    GlassMenu {
        id:presetMenu;objectName:"brushPresetContextMenu"
        topLevel:true
        Instantiator {
            model:root.library.selectedBrushItems.length>1?[0]:[]
            delegate:GlassMenuItem {text:"创建分组";onTriggered:root.openSelectedGroupDialog()}
            onObjectAdded:(index,object)=>presetMenu.insertItem(index,object)
            onObjectRemoved:(index,object)=>presetMenu.removeItem(object)
        }
        GlassMenuItem {text:root.library.selectedBrushItems.length>1?"导出选中笔刷":"导出该笔刷";onTriggered:root.beginSelectionExport()}
        MenuSeparator {}
        GlassMenuItem {text:"画笔库根目录";onTriggered:root.library.movePresetToGroup(root.contextPresetId,"")}
        GlassMenu {
            title:"移动到组"
            cascadeParent:presetMenu
            objectName:"brushPresetMoveToGroup"
            enabled:root.groupMenuGroups.length>0
        Repeater {
            model:root.groupMenuGroups
            GlassMenuItem {
                required property var modelData
                text:modelData.name
                onTriggered:root.library.movePresetToGroup(root.contextPresetId,modelData.id)
            }
        }
        }
    }
    Dialog {
        id:renameDialog;objectName:"renameBrushDialog";title:"重命名画笔";modal:true;standardButtons:Dialog.Ok|Dialog.Cancel
        anchors.centerIn:Overlay.overlay
        contentItem:TextField {id:renameName;objectName:"renameBrushName";placeholderText:"画笔名称";maximumLength:80;selectByMouse:true;onAccepted:renameDialog.accept()}
        onOpened:{renameName.forceActiveFocus();renameName.selectAll()}
        onAccepted:root.library.rename(root.renamePresetId,renameName.text)
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
            else if(root.groupSelectedMode) root.library.groupSelectedItems(groupName.text)
            else root.library.createGroup(groupName.text,root.groupParentId)
            root.groupSelectedMode=false
        }
    }
    Dialog {
        id:sourceOptions;objectName:"brushSourceExportOptions";title:"导出来源范围";modal:true;anchors.centerIn:Overlay.overlay;width:320
        contentItem:ColumnLayout {
            spacing:8
            Label {text:"选中项包含 ABR 来源。原始包可能包含其他预设和资源；选择后将完整包含在 VBR 中。";textFormat:Text.PlainText;Layout.fillWidth:true;wrapMode:Text.WordWrap;font.pixelSize:10;color:Theme.text}
            CompactCheckBox {id:includeOriginal;objectName:"includeOriginalAbr";text:"包含原始 ABR 包";font.pixelSize:10;tooltip:"包含全部原始字节以及导入时参数和兼容报告；原始包可能包含其他预设和资源。"}
        }
        footer:RowLayout {
            Item {Layout.fillWidth:true}
            Button {text:"取消";implicitHeight:24;onClicked:sourceOptions.reject()}
            Button {objectName:"confirmSourceExport";text:"继续";implicitHeight:24;enabled:includeOriginal.checked;onClicked:{root.includeExportSource=true;sourceOptions.accept();exportDialog.open()}}
        }
    }
    FileDialog {id:importDialog;objectName:"brushImportDialog";title:"导入笔刷包";nameFilters:["笔刷包 (*.vbr *.abr)","DrawVerse 笔刷 (*.vbr)","Photoshop 笔刷 (*.abr)"];onAccepted:PaintClient.importBrushes(selectedFile)}
    FileDialog {id:exportDialog;title:"导出画笔 VBR";fileMode:FileDialog.SaveFile;defaultSuffix:"vbr";nameFilters:["DrawVerse 笔刷 (*.vbr)"];onAccepted:PaintClient.exportSelectedBrushes(selectedFile,root.includeExportSource)}
    Timer {id:previewDelay;interval:100;onTriggered:root.library.requestPreview(root.library.selectedId)}
    Timer {id:visiblePreviewDelay;interval:75;onTriggered:root.requestVisiblePreviews()}
    Connections {target:root.library;function onSettingsChanged(){previewDelay.restart()}function onPresetChanged(id){root.presetLookup[id]=Object.assign({},root.library.presetInfo(id));visiblePreviewDelay.restart()}function onPreviewChanged(id,image){if(root.presetLookup[id])root.presetLookup[id].preview=image}function onPresetsChanged(){root.refreshPresets();root.groupMenuGroups=(root.library.organization&&root.library.organization.groups)||[];root.refreshGroupedItems();visiblePreviewDelay.restart()}}
    onVisibleChanged:{if(visible)visiblePreviewDelay.restart();else root.cancelVisiblePreviews()}
    Component.onCompleted:{root.refreshPresets();root.groupMenuGroups=(root.library.organization&&root.library.organization.groups)||[];root.refreshGroupedItems();visiblePreviewDelay.restart()}
    Component.onDestruction:root.cancelVisiblePreviews()
}
