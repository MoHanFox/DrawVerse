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
    property string exportPreset:""
    function beginExport() {
        includeExportSource=false
        exportPreset=library.selectedId
        if(library.sourceResource.length>0){includeOriginal.checked=false;sourceOptions.open()}
        else exportDialog.open()
    }
    readonly property var library:PaintClient.brushLibrary
    property var groupedModel:[]
    property string groupedSignature:""
    function presetById(id) {
        for(const preset of library.presets) if(preset.id===id) return preset
        return ({})
    }
    function groupedItems() {
        const presets={}; for(const p of library.presets) presets[p.id]=p
        const organization=library.organization||{}
        const groups={}; for(const g of (organization.groups||[])) groups[g.id]=g
        const query=search.text.trim().toLowerCase()
        const matches={}; for(const p of library.presets) matches[p.id]=!query.length||p.name.toLowerCase().indexOf(query)>=0
        function visit(items,out,depth) {
            for(const item of (items||[])) {
                if(item.kind==="preset") { if(matches[item.id]) out.push({kind:"preset",id:item.id,depth:depth}) }
                else if(item.kind==="group" && groups[item.id]) {
                    const children=[]; visit(groups[item.id].items,children,depth+1)
                    if(children.length) { out.push({kind:"group",id:item.id,name:groups[item.id].name,depth:depth}); out.push(...children) }
                }
            }
        }
        if(!organization.roots) return library.presets.filter(p=>matches[p.id]).map(p=>({kind:"preset",id:p.id,depth:0}))
        const result=[]; visit(organization.roots,result,0); return result
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
            visible:modelData.kind==="group" || root.presetById(modelData.id).id!==undefined
            Image {visible:modelData.kind!=="group";anchors.left:parent.left;anchors.right:parent.right;anchors.top:parent.top;anchors.leftMargin:4+modelData.depth*14;anchors.rightMargin:4;anchors.topMargin:4;height:35;source:modelData.kind==="group"?"":root.presetById(modelData.id).preview;fillMode:Image.PreserveAspectFit;asynchronous:true}
            Text {visible:modelData.kind==="group";x:8+modelData.depth*14;y:0;width:parent.width-x-8;height:30;textFormat:Text.PlainText;text:"▾  "+modelData.name;color:Theme.text;font.pixelSize:10;verticalAlignment:Text.AlignVCenter;elide:Text.ElideRight}
            Text {visible:modelData.kind!=="group";x:6+modelData.depth*14;y:40;width:parent.width-x-6;textFormat:Text.PlainText;text:root.presetById(modelData.id).name||"";color:Theme.text;font.pixelSize:9;elide:Text.ElideRight}
            MouseArea {anchors.fill:parent;enabled:modelData.kind!=="group";onClicked:root.library.select(modelData.id)}
            Component.onCompleted:if(modelData.kind!=="group")root.library.requestPreview(modelData.id)
        }
        ScrollBar.vertical:ScrollBar {width:6}
    }
    RowLayout {
        Item {Layout.fillWidth:true}
        IconButton {objectName:"importBrushPackage";glyph:"folder";tooltip:"导入 VBR / ABR 笔刷包";implicitHeight:22;enabled:!PaintClient.brushFileBusy;onClicked:importDialog.open()}
        IconButton {objectName:"exportBrushPackage";glyph:"save";tooltip:"导出当前笔刷为 VBR";implicitHeight:22;enabled:!PaintClient.brushFileBusy;onClicked:root.beginExport()}
        IconButton {glyph:"plus";tooltip:"保存当前设置为新画笔";implicitHeight:22;onClicked:saveDialog.open()}
        IconButton {glyph:"trash";tooltip:"删除自定义画笔";implicitHeight:22;enabled:root.library.presets.some(p=>p.id===root.library.selectedId && !p.builtin);onClicked:root.library.remove(root.library.selectedId)}
    }
    Dialog {
        id:saveDialog;title:"保存画笔";modal:true;standardButtons:Dialog.Ok|Dialog.Cancel
        anchors.centerIn:Overlay.overlay
        TextField {id:brushName;placeholderText:"画笔名称";maximumLength:80;text:root.library.selectedName+" 副本"}
        onAccepted:root.library.saveCopy(brushName.text)
    }
    Dialog {
        id:sourceOptions;objectName:"brushSourceExportOptions";title:"导出来源范围";modal:true;anchors.centerIn:Overlay.overlay;width:320
        contentItem:ColumnLayout {
            spacing:8
            Label {text:"该笔刷包含 ABR 来源。原始包可能包含其他预设和资源；选择后将完整包含在 VBR 中。";textFormat:Text.PlainText;Layout.fillWidth:true;wrapMode:Text.WordWrap;font.pixelSize:10;color:Theme.text}
            CompactCheckBox {id:includeOriginal;objectName:"includeOriginalAbr";text:"包含原始 ABR 包";font.pixelSize:10;tooltip:"包含全部原始字节以及导入时参数和兼容报告；原始包可能包含其他预设和资源。"}
        }
        footer:RowLayout {
            Item {Layout.fillWidth:true}
            Button {text:"取消";implicitHeight:24;onClicked:sourceOptions.reject()}
            Button {objectName:"confirmSourceExport";text:"继续";implicitHeight:24;enabled:includeOriginal.checked;onClicked:{root.includeExportSource=true;sourceOptions.accept();exportDialog.open()}}
        }
    }
    FileDialog {id:importDialog;objectName:"brushImportDialog";title:"导入笔刷包";nameFilters:["笔刷包 (*.vbr *.abr)","DrawVerse 笔刷 (*.vbr)","Photoshop 笔刷 (*.abr)"];onAccepted:PaintClient.importBrushes(selectedFile)}
    FileDialog {id:exportDialog;title:"导出当前笔刷";fileMode:FileDialog.SaveFile;defaultSuffix:"vbr";nameFilters:["DrawVerse 笔刷 (*.vbr)"];onAccepted:PaintClient.exportBrush(selectedFile,root.includeExportSource,root.exportPreset)}
    Timer {id:previewDelay;interval:100;onTriggered:root.library.requestPreview(root.library.selectedId)}
    Connections {target:root.library;function onSettingsChanged(){previewDelay.restart()}function onPresetsChanged(){root.refreshGroupedItems()}}
    Component.onCompleted:root.refreshGroupedItems()
}
