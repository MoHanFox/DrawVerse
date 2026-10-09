import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import QtQuick.Dialogs as FileDialogs
import DrawVerse 1.0
import "."

ApplicationWindow {
    id: root
    objectName: "mainWindow"
    visible: true
    flags: Qt.Window | Qt.FramelessWindowHint
    width: 1480; height: 780; minimumWidth: 980; minimumHeight: 640
    title: "DrawVerse · "+PaintClient.documentName+(PaintClient.modified ? " *" : "")
    color: "transparent"
    readonly property int cornerRadius: visibility===Window.Maximized ? 0 : Theme.windowRadius
    background: Item {
        // Keep the translucent menu bar clear of the opaque workspace backing.
        Rectangle {
            objectName: "opaqueWorkspaceBackground"
            y: root.menuBar.height
            width: parent.width; height: parent.height-y
            color: Theme.background; radius: root.cornerRadius
        }
        Rectangle {
            y: root.menuBar.height
            width: parent.width; height: root.cornerRadius
            color: Theme.background
        }
    }
    font.family: Qt.platform.os === "windows" ? "Microsoft YaHei UI" : "sans-serif"
    font.pixelSize: 10
    palette.window: Theme.surface
    palette.windowText: Theme.text
    palette.base: Theme.background
    palette.text: Theme.text
    palette.button: Theme.raised
    palette.buttonText: Theme.text
    palette.highlight: Theme.selected
    palette.highlightedText: Theme.accent
    property bool quitting: false
    property string pendingAction: ""
    property url pendingOpen: ""
    property bool savingBeforeAction: false
    property string fileNotice: ""
    property var floatingWindows: ({})
    property var toolsWindow: null
    function syncTools() {
        if(Workspace.toolsFloating && !toolsWindow) toolsWindow=toolsFactory.createObject(root,{canvasView:canvas})
        else if(!Workspace.toolsFloating && toolsWindow) {toolsWindow.visible=false;toolsWindow.destroy();toolsWindow=null}
    }
    function syncFloating() {
        const groups=Workspace.floatingGroups, live={}
        for(let i=0;i<groups.length;i++) {
            const g=groups[i]; live[g.id]=true
            if(floatingWindows[g.id]) floatingWindows[g.id].updateLayout(g)
            else floatingWindows[g.id]=floatingFactory.createObject(root,{groupData:g})
        }
        for(const id in floatingWindows) if(!live[id]) {
            floatingWindows[id].visible=false; floatingWindows[id].destroy(); delete floatingWindows[id]
        }
    }
    property bool menuBarBlurActive: false
    Component.onCompleted: {
        // Set initial placement once; live size bindings move maximized windows.
        x=Screen.virtualX+Math.max(16,(Screen.width-width)/2)
        y=Screen.virtualY+Math.max(16,(Screen.height-height)/3)
        menuBarBlurActive=Workspace.setMenuBarBlur(root,true)
        syncFloating();syncTools()
    }
    Connections {target:Workspace;function onToolStripChanged(){root.syncTools()}}
    Component {id:toolsFactory;ToolStripWindow {}}
    Connections { target: Workspace; function onGroupsChanged() { root.syncFloating() } }
    Component { id: floatingFactory; FloatingPanel { canvasView: canvas } }
    function executePending() {
        const action = pendingAction
        pendingAction = ""; savingBeforeAction = false
        if (action === "close") quitApp()
        else if (action === "open") PaintClient.openDocument(pendingOpen)
        else if (action === "new") { if(transparentBackground.checked) PaintClient.newTransparentDocument(newWidth.value,newHeight.value); else PaintClient.newDocument(newWidth.value,newHeight.value) }
    }
    function requestAction(action) {
        pendingAction = action
        if (PaintClient.modified) closeDialog.open()
        else executePending()
    }
    function saveCurrent() {
        if (PaintClient.documentUrl.toString().length > 0) {
            if (!PaintClient.saveDocument(PaintClient.documentUrl)) savingBeforeAction = false
        } else saveDialog.open()
    }
    function quitApp() { Workspace.saveLayout(); quitting=true; PaintClient.shutdown() }
    onClosing: event => {
        if (PaintClient.closing) { event.accepted=true; return }
        event.accepted=false
        if (quitting) return
        if (PaintClient.drawing) PaintClient.endStroke()
        if (PaintClient.fileBusy) busyCloseDialog.open()
        else requestAction("close")
    }
    ResizeFrame {targetWindow:root}
    StoragePreferences { id: storagePreferences; objectName: "storagePreferences" }
    menuBar: MenuBar {
        objectName: "mainMenuBar"
        implicitHeight: 28; leftPadding: 30; rightPadding: 100
        background: MenuSurface {
            objectName: "menuBarGlassBackground"
            tint: Theme.menuBarGlass
            topCornersOnly: true
            radius: root.visibility===Window.Maximized ? 0 : Theme.menuRadius
            MouseArea {
                anchors.fill: parent
                onPressed: root.startSystemMove()
                onDoubleClicked: {if(root.visibility===Window.Maximized)root.showNormal();else root.showMaximized()}
            }
            Row {
                anchors.right: parent.right; height: parent.height
                IconButton { objectName:"windowMinimize";width:32;height:28;padding:10;glyph:"minimize";tooltip:"最小化";onClicked:root.showMinimized() }
                IconButton { objectName:"windowMaximize";width:32;height:28;padding:10;glyph:root.visibility===Window.Maximized?"restore":"maximize";tooltip:"最大化 / 还原";onClicked:{if(root.visibility===Window.Maximized)root.showNormal();else root.showMaximized()} }
                IconButton { objectName:"windowClose";width:32;height:28;padding:10;glyph:"close";tooltip:"关闭";onClicked:root.close() }
            }
            Image { objectName: "applicationLogo"; x: 8; y: 5; width: 18; height: 18; source: "assets/logo.png"; fillMode: Image.PreserveAspectFit; mipmap: true }
        }
        delegate: MenuBarItem {
            id: menuEntry
            objectName: "menuEntry:"+text
            implicitHeight: 28; implicitWidth:contentItem.implicitWidth+16
            leftPadding:8;rightPadding:8;font.pixelSize:9
            contentItem: Text { text: menuEntry.text; font: menuEntry.font; color: menuEntry.highlighted ? "#eeeeee" : Theme.text; verticalAlignment: Text.AlignVCenter }
            background: Rectangle { radius: 0; color: menuEntry.highlighted ? Theme.hover : "transparent" }
        }
        GlassMenu {
            title: "文件"
            Action { text: "新建画布…"; shortcut: StandardKey.New; enabled: PaintClient.ready && !PaintClient.drawing && !PaintClient.fileBusy; onTriggered: newDialog.open() }
            Action { text: "打开…"; shortcut: StandardKey.Open; enabled: PaintClient.ready && !PaintClient.drawing && !PaintClient.fileBusy; onTriggered: openDialog.open() }
            MenuSeparator {}
            Action { text: "保存"; shortcut: StandardKey.Save; enabled: PaintClient.ready && !PaintClient.drawing && !PaintClient.fileBusy; onTriggered: root.saveCurrent() }
            Action { text: "另存为 OpenRaster…"; shortcut: StandardKey.SaveAs; enabled: PaintClient.ready && !PaintClient.drawing && !PaintClient.fileBusy; onTriggered: saveDialog.open() }
            Action { text: "导出图片…"; enabled: PaintClient.ready && !PaintClient.drawing && !PaintClient.fileBusy; onTriggered: exportDialog.open() }
            MenuSeparator {}
            Action { text: "退出"; onTriggered: root.close() }
        }
        GlassMenu {
            title: "编辑"
            Action { text: "性能与暂存盘…"; enabled: !PaintClient.closing; onTriggered: storagePreferences.open() }
            Action { text: "撤销"; shortcut: StandardKey.Undo; enabled: PaintClient.undoDepth>0 && !PaintClient.drawing; onTriggered: PaintClient.undo() }
            Action { text: "重做"; shortcut: "Ctrl+Shift+Z"; enabled: PaintClient.redoDepth>0 && !PaintClient.drawing; onTriggered: PaintClient.redo() }
        }
        GlassMenu {
            title: "选择"
            Action { text: "矩形选框（M）"; enabled: PaintClient.ready && !PaintClient.drawing; onTriggered: PaintClient.selectionTool=1 }
            Action { text: "椭圆选框（Shift+M）"; enabled: PaintClient.ready && !PaintClient.drawing; onTriggered: PaintClient.selectionTool=2 }
            MenuSeparator {}
            Action { objectName:"selectionAllAction"; text: "全选"; enabled: PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy; onTriggered: PaintClient.selectAll() }
            Action { objectName:"selectionClearAction"; text: "取消选择"; enabled: PaintClient.selectionEnabled && !PaintClient.drawing && !PaintClient.layerEditBusy; onTriggered: PaintClient.clearSelection() }
            Action { objectName:"selectionInvertAction"; text: "反向选择"; enabled: PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy; onTriggered: PaintClient.invertSelection() }
        }
        GlassMenu {
            title: "视图"
            Action { text: "适合窗口"; onTriggered: canvas.fitToView() }
            Action { text: "实际像素"; onTriggered: canvas.actualSize() }
        }
        GlassMenu {
            id: windowMenu
            objectName: "windowMenu"
            title: "窗口"
            Action {text:"工具条归位";enabled:Workspace.toolsFloating;onTriggered:Workspace.dockToolStrip("drawverse-tools-v1")}
            MenuSeparator {}
            Instantiator {
                model: Workspace.allPanels
                delegate: GlassMenuItem {
                    required property string modelData
                    objectName: "windowPanel:"+modelData
                    text: Workspace.panelDefinition(modelData).title
                    checkable: true; checked: Workspace.visiblePanels.indexOf(modelData)>=0
                    onTriggered: {
                        if(checked) Workspace.showPanel(modelData)
                        else {
                            const groups=Workspace.leftGroups.concat(Workspace.rightGroups,Workspace.floatingGroups)
                            for(let i=0;i<groups.length;i++) if(groups[i].panels.indexOf(modelData)>=0) { Workspace.hidePanel(groups[i].id,modelData); break }
                        }
                    }
                }
                onObjectAdded: (index,object) => windowMenu.insertItem(index,object)
                onObjectRemoved: (index,object) => windowMenu.removeItem(object)
            }
        }
        GlassMenu {
            objectName: "workspaceMenu"
            title: "工作区"
            Action { text: "自定义面板…"; onTriggered: panelDialog.open() }
            Action { text: "保存当前布局"; onTriggered: Workspace.saveLayout() }
            Action { text: "参考图布局"; onTriggered: Workspace.applyReferenceLayout(root.x,root.y,root.width,root.height) }
            Action { text: "双列停靠布局"; onTriggered: Workspace.resetLayout() }
        }
    }
    header: Rectangle {
        objectName: "brushOptionsBar"
        height: 28; color: Theme.raised
        RowLayout {
            visible: !PaintClient.selectionTool
            anchors.fill: parent; anchors.leftMargin: 12; anchors.rightMargin: 12; spacing: 8
            Icon { name: PaintClient.selectionTool ? (PaintClient.selectionTool===2 ? "ellipseSelection":"rectangleSelection") : PaintClient.moveTool ? "move" : PaintClient.eraser ? "eraser" : "brush"; Layout.preferredWidth: 17; Layout.preferredHeight: 17 }
            Label { text: PaintClient.selectionTool ? (PaintClient.selectionTool===2?"椭圆选框":"矩形选框") : PaintClient.moveTool ? "移动图层" : PaintClient.eraser ? "橡皮擦" : "画笔"; color: Theme.text; Layout.preferredWidth: 42 }
            Rectangle { width: 1; height: 22; color: Theme.line }
            Label { text: "大小"; color: Theme.muted }
            CompactSpinBox { objectName: "brushRadius"; from: 1; to: 512; value: Math.round(PaintClient.brushRadius*2); editable: true; implicitWidth: 76; implicitHeight: 20; onValueModified: PaintClient.brushRadius=value/2 }
            Label { text: "px"; color: Theme.muted }
            Label { text: "不透明度"; color: Theme.muted }
            CompactSlider { from: .01; to: 1; value: PaintClient.brushOpacity; Layout.preferredWidth: 95; onMoved: PaintClient.brushOpacity=value }
            Label { text: Math.round(PaintClient.brushOpacity*100)+"%"; color: Theme.text; Layout.preferredWidth: 36 }
            Rectangle { width: 16; height: 16; color: PaintClient.brushColor; border.color: Theme.muted }
            Label { text: "压感"; color: Theme.muted }
            Item { Layout.fillWidth: true }
            IconButton { glyph: "settings"; tooltip: "性能与暂存盘"; onClicked: storagePreferences.open() }
            IconButton { glyph: "plus"; tooltip: "创建自定义面板"; onClicked: panelDialog.open() }
        }
        RowLayout {
            visible: PaintClient.selectionTool>0
            anchors.fill:parent;anchors.leftMargin:12;anchors.rightMargin:12;spacing:8
            Icon {name:PaintClient.selectionTool===2?"ellipseSelection":"rectangleSelection";Layout.preferredWidth:17;Layout.preferredHeight:17}
            CompactComboBox {model:["矩形选框","椭圆选框"];currentIndex:Math.max(0,PaintClient.selectionTool-1);implicitWidth:90;implicitHeight:20;onActivated:PaintClient.selectionTool=currentIndex+1}
            Label {text:"Shift 添加 · Alt 减去 · Shift+Alt 相交";color:Theme.muted}
            Item {Layout.fillWidth:true}
            ToolButton {text:"全选";enabled:PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy;onClicked:PaintClient.selectAll()}
            ToolButton {text:"取消选择";enabled:PaintClient.selectionEnabled && !PaintClient.drawing && !PaintClient.layerEditBusy;onClicked:PaintClient.clearSelection()}
        }
        Rectangle { anchors.bottom: parent.bottom; height: 1; width: parent.width; color: Theme.panelBar }
    }
    ColumnLayout {
        anchors.fill: parent; spacing: 0
        Rectangle {
            Layout.fillWidth: true; height: PaintClient.lastError.length>0 ? 40 : 0; visible: height>0; color: "#5d3435"
            RowLayout {
                anchors.fill: parent; anchors.margins: 6
                Label { text: PaintClient.lastError; color: "#ffe4df"; Layout.fillWidth: true; elide: Text.ElideRight }
                ToolButton { text: "×"; onClicked: PaintClient.clearError() }
            }
        }
        RowLayout {
            Layout.fillWidth: true; Layout.fillHeight: true; spacing: 0
            ToolStrip {Layout.fillHeight:true;Layout.preferredWidth:36;visible:!Workspace.toolsFloating;canvasView:canvas}
            SplitView {
                Layout.fillWidth: true; Layout.fillHeight: true; orientation: Qt.Horizontal
                handle: Rectangle { implicitWidth: 4; color: SplitHandle.hovered || SplitHandle.pressed ? Theme.accent : Theme.panelBar }
                DockColumn {
                    side: "left"; groups: Workspace.leftGroups; canvasView: canvas
                    SplitView.preferredWidth: implicitWidth
                    SplitView.minimumWidth: empty ? 6 : collapsed ? 28 : 150
                    SplitView.maximumWidth: empty ? 6 : collapsed ? 28 : 520
                }
                ColumnLayout {
                    SplitView.fillWidth: true; SplitView.minimumWidth: 400; spacing: 0
                    Rectangle {
                        Layout.fillWidth: true; height: 22; color: Theme.strip
                        Rectangle {
                            objectName:"documentTab";x:6;y:2;width:Math.min(parent.width-35,documentTitle.implicitWidth+24);height:parent.height-2
                            color:Theme.background;radius:Theme.documentTabRadius
                            Rectangle {anchors.left:parent.left;anchors.right:parent.right;anchors.bottom:parent.bottom;height:parent.radius;color:parent.color}
                            Label {id:documentTitle;anchors.centerIn:parent;font.pixelSize:9;text:PaintClient.documentName+(PaintClient.modified?" *":"")+"  @ "+Math.round(canvas.zoom*100)+"% · "+PaintClient.documentWidth+" × "+PaintClient.documentHeight;color:Theme.text}
                        }
                        IconButton {anchors.right:parent.right;width:24;height:22;padding:4;glyph:"plus";tooltip:"新建画布";onClicked:newDialog.open()}
                    }
                    Rectangle {
                        Layout.fillWidth: true; Layout.fillHeight: true; color: Theme.background; clip: true
                        Rectangle { x: canvas.documentRect.x+8; y: canvas.documentRect.y+10; width: canvas.documentRect.width; height: canvas.documentRect.height; color: "#0c0d0f" }
                        TransparencyGrid { objectName: "canvasTransparency"; x: canvas.documentRect.x; y: canvas.documentRect.y; width: canvas.documentRect.width; height: canvas.documentRect.height }
                        PaintCanvas { id: canvas; initialFitRatio: .76; objectName: "mainCanvas"; anchors.fill: parent; client: PaintClient; focus: true; enabled: !root.savingBeforeAction && !closeDialog.opened }
                        Loader {
                            objectName:"selectionOutlineLoader";anchors.fill:parent
                            // No painted item or texture exists while selection is inactive.
                            active:PaintClient.selectionEnabled || (canvas.selectionPreview.width>0 && canvas.selectionPreview.height>0)
                            sourceComponent:SelectionOutline {objectName:"selectionOutline";enabledSelection:PaintClient.selectionEnabled;steps:PaintClient.selectionSteps;documentRect:canvas.documentRect;zoom:canvas.zoom;preview:canvas.selectionPreview;previewKind:canvas.selectionPreviewKind}
                        }
                        BusyIndicator { objectName: "canvasBusy"; anchors.centerIn: parent; running: !PaintClient.ready; visible: running }
                    }
                }
                DockColumn {
                    side: "right"; groups: Workspace.rightGroups; canvasView: canvas
                    SplitView.preferredWidth: implicitWidth
                    SplitView.minimumWidth: empty ? 6 : collapsed ? 28 : 190
                    SplitView.maximumWidth: empty ? 6 : collapsed ? 28 : 520
                }
            }
        }
        Rectangle {
            Layout.fillWidth: true; height: 22; color: Theme.strip;radius:root.cornerRadius
            Rectangle {anchors.left:parent.left;anchors.right:parent.right;anchors.top:parent.top;height:parent.height/2;color:parent.color}
            RowLayout {
                anchors.fill: parent; anchors.leftMargin: 18; anchors.rightMargin: 18
                Label { text: PaintClient.closing ? "正在关闭核心…" : PaintClient.fileBusy ? "正在读写文件…" : PaintClient.drawing ? "绘画中" : root.fileNotice.length ? root.fileNotice : "就绪"; color: Theme.accent; font.pixelSize: 10 }
                ToolButton { text: "取消文件任务"; visible: PaintClient.fileBusy; onClicked: PaintClient.cancelFile() }
                Label { text: "  "+PaintClient.documentWidth+" × "+PaintClient.documentHeight+" px  ·  sRGB / 32F"; color: Theme.muted; font.pixelSize: 10 }
                Item { Layout.fillWidth: true }
                Label { text: "空格平移  ·  滚轮缩放"; color: Theme.muted; font.pixelSize: 10 }
                Label { text: "   "+Math.round(canvas.zoom*100)+"%"; color: Theme.text; font.pixelSize: 10 }
            }
        }
    }
    DropArea {
        objectName:"toolStripDockTarget";x:0;y:0;width:Workspace.toolsFloating?28:36;height:root.contentItem.height-22
        keys:["application/x-drawverse-tool-strip"]
        onEntered:drag=>{drag.accepted=!Workspace.dockingSuppressed}
        onPositionChanged:drag=>{drag.accepted=!Workspace.dockingSuppressed}
        onDropped:drop=>{if(!Workspace.dockingSuppressed && Workspace.dockToolStrip(drop.getDataAsString("application/x-drawverse-tool-strip")))drop.acceptProposedAction()}
        Rectangle {anchors.fill:parent;color:"#3023b5ee";border.color:Theme.accent;border.width:2;visible:parent.containsDrag && !Workspace.dockingSuppressed}
    }
    Shortcut { sequence: "V"; enabled: canvas.activeFocus; onActivated: PaintClient.moveTool=true }
    Shortcut { sequence: "M"; enabled:canvas.activeFocus && !PaintClient.drawing; onActivated:PaintClient.selectionTool=1 }
    Shortcut { sequence: "Shift+M"; enabled:canvas.activeFocus && !PaintClient.drawing; onActivated:PaintClient.selectionTool=PaintClient.selectionTool===2?1:2 }
    Shortcut { sequence:StandardKey.SelectAll; enabled:canvas.activeFocus && PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy; onActivated:PaintClient.selectAll() }
    Shortcut { sequence:Qt.platform.os==="osx"?"Meta+D":"Ctrl+D"; enabled:canvas.activeFocus && PaintClient.selectionEnabled && !PaintClient.drawing && !PaintClient.layerEditBusy; onActivated:PaintClient.clearSelection() }
    Shortcut { sequence:Qt.platform.os==="osx"?"Meta+Shift+I":"Ctrl+Shift+I"; enabled:canvas.activeFocus && PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy; onActivated:PaintClient.invertSelection() }
    Shortcut { sequence: "B"; enabled: canvas.activeFocus; onActivated: PaintClient.eraser=false }
    Shortcut { sequence: "E"; enabled: canvas.activeFocus; onActivated: PaintClient.eraser=true }
    Shortcut { sequence: "F"; enabled: canvas.activeFocus; onActivated: canvas.fitToView() }
    Shortcut { sequence: "Esc"; enabled: PaintClient.drawing; onActivated: PaintClient.cancelStroke() }
    Shortcut { sequence: Qt.platform.os === "osx" ? "Meta+Alt+G" : "Ctrl+Alt+G"; context:Qt.ApplicationShortcut; enabled: PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy && !PaintClient.fileBusy; onActivated: PaintClient.toggleActiveClipping() }
    Dialog {
        id: newDialog
        title: "新建画布"; modal: true; anchors.centerIn: parent; width: 390
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: root.requestAction("new")
        ColumnLayout {
            width: parent.width; spacing: 14
            Label { text: "新建前可保存当前绘画。大画布按需分配瓦片。"; wrapMode: Text.WordWrap; Layout.fillWidth: true; color: Theme.muted }
            RowLayout { Label { text: "宽度" } CompactSpinBox { id: newWidth; from: 1; to: 1000000; value: 960; editable: true } }
            RowLayout { Label { text: "高度" } CompactSpinBox { id: newHeight; from: 1; to: 1000000; value: 640; editable: true } }
            CheckBox {id:transparentBackground;objectName:"transparentBackground";text:"透明背景";checked:false}
            Label { text: "默认白色背景 · 线性 sRGB · RGBA 32F"; color: Theme.muted; font.pixelSize: 11 }
        }
    }
    Dialog {
        id: panelDialog
        title: "创建自定义面板"; modal: true; anchors.centerIn: parent; width: 390
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: Workspace.addCustomPanel(panelName.text,["palette","brush"][panelKind.currentIndex])
        ColumnLayout {
            width: parent.width; spacing: 12
            TextField { id: panelName; placeholderText: "面板名称"; text: "自定义色板"; maximumLength: 80; Layout.fillWidth: true }
            CompactComboBox { id: panelKind; model: ["色板","画笔参数"]; Layout.fillWidth: true }
            Label { text: "可自由组合、浮动为独立窗口，随工作区保存。"; wrapMode: Text.WordWrap; Layout.fillWidth: true; color: Theme.muted }
        }
    }
    Dialog {
        id: closeDialog
        title: "保存当前绘画？"; modal: true; anchors.centerIn: parent; width: 390
        standardButtons: Dialog.Save | Dialog.Discard | Dialog.Cancel
        onAccepted: { root.savingBeforeAction=true; root.saveCurrent() }
        onDiscarded: root.executePending()
        onRejected: { root.pendingAction=""; root.savingBeforeAction=false }
        Label { text: "当前绘画尚未保存。OpenRaster 保存图层，工作区布局会保留。"; width: parent.width; wrapMode: Text.WordWrap; color: Theme.muted }
    }
    Dialog {
        id: busyCloseDialog
        title: "文件操作尚未完成"; modal: true; anchors.centerIn: parent; width: 390
        standardButtons: Dialog.Discard | Dialog.Cancel
        onDiscarded: root.quitApp()
        Label { text: "关闭会取消未完成的文件任务，并丢弃尚未保存的绘画。"; width: parent.width; wrapMode: Text.WordWrap; color: Theme.muted }
    }
    FileDialogs.FileDialog {
        id: openDialog; title: "打开绘画或图片"
        nameFilters: ["绘画与图片 (*.ora *.png *.jpg *.jpeg *.webp)"]
        onAccepted: { root.pendingOpen=selectedFile; root.requestAction("open") }
    }
    FileDialogs.FileDialog {
        id: saveDialog; title: "保存图层 · OpenRaster（8 位 sRGB）"
        fileMode: FileDialogs.FileDialog.SaveFile; defaultSuffix: "ora"
        nameFilters: ["OpenRaster (*.ora)"]
        onAccepted: { if (!PaintClient.saveDocument(selectedFile)) root.savingBeforeAction=false }
        onRejected: { root.savingBeforeAction=false; root.pendingAction="" }
    }
    FileDialogs.FileDialog {
        id: exportDialog; title: "导出合成图 · JPEG 使用白色背景"
        fileMode: FileDialogs.FileDialog.SaveFile
        defaultSuffix: ["png","jpg","webp"][selectedNameFilter.index]
        nameFilters: ["PNG (*.png)","JPEG (*.jpg *.jpeg)","WebP (*.webp)"]
        onAccepted: PaintClient.saveDocument(selectedFile,selectedNameFilter.index+1)
    }
    Connections {
        target: PaintClient
        function onFileFinished(success) {
            root.fileNotice=success ? "文件操作完成" : "文件操作未完成"
            if (root.savingBeforeAction) {
                root.savingBeforeAction=false
                if (success && !PaintClient.modified) root.executePending()
                else {
                    root.pendingAction=""
                    if (success) root.fileNotice="保存期间又有修改，请再次保存后继续"
                }
            }
        }
    }
}
