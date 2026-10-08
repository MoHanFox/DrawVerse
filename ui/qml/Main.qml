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
    width: 1480; height: 940; minimumWidth: 980; minimumHeight: 660
    title: "DrawVerse · "+PaintClient.documentName+(PaintClient.modified ? " *" : "")
    color: Theme.background
    font.family: Qt.platform.os === "windows" ? "Microsoft YaHei UI" : "sans-serif"
    font.pixelSize: 12
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
    StoragePreferences { id: storagePreferences; objectName: "storagePreferences" }
    menuBar: MenuBar {
        Menu {
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
        Menu {
            title: "编辑"
            Action { text: "性能与暂存盘…"; enabled: !PaintClient.closing; onTriggered: storagePreferences.open() }
            Action { text: "撤销"; shortcut: StandardKey.Undo; enabled: PaintClient.undoDepth>0 && !PaintClient.drawing; onTriggered: PaintClient.undo() }
            Action { text: "重做"; shortcut: "Ctrl+Shift+Z"; enabled: PaintClient.redoDepth>0 && !PaintClient.drawing; onTriggered: PaintClient.redo() }
        }
        Menu {
            title: "视图"
            Action { text: "适合窗口"; onTriggered: canvas.fitToView() }
            Action { text: "实际像素"; onTriggered: canvas.actualSize() }
        }
        Menu {
            title: "工作区"
            Action { text: "自定义面板…"; onTriggered: panelDialog.open() }
            Action { text: "保存当前布局"; onTriggered: Workspace.saveLayout() }
            Action { text: "恢复默认布局"; onTriggered: Workspace.resetLayout() }
        }
    }
    header: Rectangle {
        height: 64; color: Theme.surface
        RowLayout {
            anchors.fill: parent; anchors.leftMargin: 20; anchors.rightMargin: 20; spacing: 18
            Row {
                spacing: 3
                Label { text: "DRAW"; color: Theme.text; font.pixelSize: 18; font.weight: Font.DemiBold; font.letterSpacing: 2 }
                Label { text: "VERSE"; color: Theme.accent; font.pixelSize: 18; font.weight: Font.Light; font.letterSpacing: 2 }
            }
            Rectangle { width: 1; height: 28; color: Theme.line }
            Label { text: PaintClient.moveTool ? "移动图层" : PaintClient.eraser ? "橡皮擦" : "压感圆笔"; color: Theme.text }
            Label { text: "半径"; color: Theme.muted }
            SpinBox { from: 1; to: 256; value: Math.round(PaintClient.brushRadius); editable: true; implicitWidth: 105; onValueModified: PaintClient.brushRadius=value }
            Label { text: "不透明度"; color: Theme.muted }
            Slider { from: .01; to: 1; value: PaintClient.brushOpacity; Layout.preferredWidth: 115; onMoved: PaintClient.brushOpacity=value }
            Label { text: Math.round(PaintClient.brushOpacity*100)+"%"; color: Theme.muted; Layout.preferredWidth: 36 }
            Rectangle { width: 28; height: 28; radius: 6; color: PaintClient.brushColor; border.color: Theme.muted }
            Item { Layout.fillWidth: true }
            Button { text: "+ 自定义面板"; onClicked: panelDialog.open() }
        }
        Rectangle { anchors.bottom: parent.bottom; height: 1; width: parent.width; color: Theme.line }
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
            Rectangle {
                Layout.fillHeight: true; Layout.preferredWidth: 62; color: Theme.surface
                Column {
                    anchors.horizontalCenter: parent.horizontalCenter; anchors.top: parent.top; anchors.topMargin: 18; spacing: 10
                    ToolButton { objectName: "moveLayerTool"; width: 44; height: 44; text: "✥"; font.pixelSize: 26; checkable: true; checked: PaintClient.moveTool; onClicked: PaintClient.moveTool=true; ToolTip.visible: hovered; ToolTip.text: "移动图层 V（松开提交；方向键 1px / Shift 10px）" }
                    ToolButton { width: 44; height: 44; text: "✎"; font.pixelSize: 26; checkable: true; checked: !PaintClient.eraser && !PaintClient.moveTool; onClicked: PaintClient.eraser=false; ToolTip.visible: hovered; ToolTip.text: "画笔 B" }
                    ToolButton { width: 44; height: 44; text: "▱"; font.pixelSize: 26; checkable: true; checked: PaintClient.eraser && !PaintClient.moveTool; onClicked: PaintClient.eraser=true; ToolTip.visible: hovered; ToolTip.text: "橡皮擦 E" }
                    Rectangle { width: 28; height: 1; color: Theme.line; anchors.horizontalCenter: parent.horizontalCenter }
                    ToolButton { width: 44; height: 40; text: "↶"; font.pixelSize: 22; enabled: PaintClient.undoDepth>0 && !PaintClient.drawing; onClicked: PaintClient.undo(); ToolTip.visible: hovered; ToolTip.text: "撤销" }
                    ToolButton { width: 44; height: 40; text: "↷"; font.pixelSize: 22; enabled: PaintClient.redoDepth>0 && !PaintClient.drawing; onClicked: PaintClient.redo() }
                }
                Column {
                    anchors.bottom: parent.bottom; anchors.horizontalCenter: parent.horizontalCenter; anchors.bottomMargin: 24; spacing: 5
                    Rectangle { width: 32; height: 32; radius: 6; color: PaintClient.brushColor; border.color: Theme.muted }
                    Text { text: "B / E"; color: Theme.muted; font.pixelSize: 10 }
                }
            }
            SplitView {
                Layout.fillWidth: true; Layout.fillHeight: true; orientation: Qt.Horizontal
                handle: Rectangle { implicitWidth: 6; color: SplitHandle.pressed ? Theme.accent : Theme.background }
                Rectangle {
                    id: leftDock
                    SplitView.preferredWidth: Workspace.leftGroups.length>0 ? 270 : 18
                    SplitView.minimumWidth: Workspace.leftGroups.length>0 ? 240 : 18
                    SplitView.maximumWidth: Workspace.leftGroups.length>0 ? 520 : 18
                    color: leftDrop.containsDrag ? Theme.selected : Theme.background
                    DropArea {
                        id: leftDrop; anchors.left: parent.left; anchors.top: parent.top; anchors.bottom: parent.bottom
                        width: Workspace.leftGroups.length>0 ? 4 : parent.width
                        keys: ["application/x-drawverse-panel"]
                        onDropped: d => { if(Workspace.dockPayload(d.getDataAsString("application/x-drawverse-panel"),"left")) d.acceptProposedAction() }
                    }
                    SplitView {
                        anchors.fill: parent; anchors.margins: 4; orientation: Qt.Vertical
                        Repeater {
                            model: Workspace.leftGroups
                            PanelGroup { required property var modelData; groupData: modelData; canvasView: canvas; SplitView.fillHeight: true; SplitView.minimumHeight: minimumPanelHeight }
                        }
                    }
                }
                ColumnLayout {
                    SplitView.fillWidth: true; SplitView.minimumWidth: 400; spacing: 0
                    Rectangle {
                        Layout.fillWidth: true; height: 42; color: Theme.surface
                        RowLayout {
                            anchors.fill: parent; anchors.leftMargin: 16; anchors.rightMargin: 16
                            Rectangle { width: 6; height: 6; radius: 3; color: PaintClient.modified ? Theme.accent : Theme.muted }
                            Label { text: PaintClient.documentName+(PaintClient.modified ? " *" : ""); color: Theme.text }
                            Label { text: " / "+PaintClient.documentWidth+" × "+PaintClient.documentHeight; color: Theme.muted; font.pixelSize: 11 }
                            Item { Layout.fillWidth: true }
                            Label { text: "线性 sRGB  ·  32F"; color: Theme.muted; font.pixelSize: 10 }
                        }
                    }
                    Rectangle {
                        Layout.fillWidth: true; Layout.fillHeight: true; color: "#15191d"; clip: true
                        Rectangle { x: canvas.documentRect.x+8; y: canvas.documentRect.y+10; width: canvas.documentRect.width; height: canvas.documentRect.height; color: "#0e1114" }
                        TransparencyGrid { objectName: "canvasTransparency"; x: canvas.documentRect.x; y: canvas.documentRect.y; width: canvas.documentRect.width; height: canvas.documentRect.height }
                        PaintCanvas { id: canvas; objectName: "mainCanvas"; anchors.fill: parent; client: PaintClient; focus: true; enabled: !root.savingBeforeAction && !closeDialog.opened }
                        Label { anchors.horizontalCenter: parent.horizontalCenter; anchors.bottom: parent.bottom; anchors.bottomMargin: 14; text: PaintClient.moveTool ? "拖动松开移动图层  ·  方向键 1 px / Shift 10 px  ·  Esc 取消" : "空格 / 中键平移    ·    滚轮缩放    ·    F 适合窗口"; color: Theme.muted; font.pixelSize: 10 }
                        BusyIndicator { objectName: "canvasBusy"; anchors.centerIn: parent; running: !PaintClient.ready; visible: running }
                    }
                }
                Rectangle {
                    SplitView.preferredWidth: 305; SplitView.minimumWidth: 260; SplitView.maximumWidth: 520
                    color: rightDrop.containsDrag ? Theme.selected : Theme.background
                    DropArea {
                        id: rightDrop; anchors.left: parent.left; anchors.top: parent.top; anchors.bottom: parent.bottom; width: 8
                        keys: ["application/x-drawverse-panel"]
                        onDropped: d => { if (Workspace.dockPayload(d.getDataAsString("application/x-drawverse-panel"),"right")) d.acceptProposedAction() }
                    }
                    SplitView {
                        anchors.fill: parent; anchors.margins: 8; orientation: Qt.Vertical
                        handle: Rectangle { implicitHeight: 8; color: SplitHandle.pressed ? Theme.accent : Theme.background }
                        Repeater {
                            model: Workspace.rightGroups
                            PanelGroup { required property var modelData; groupData: modelData; canvasView: canvas; SplitView.fillHeight: true; SplitView.minimumHeight: minimumPanelHeight }
                        }
                    }
                }
            }
        }
        Rectangle {
            Layout.fillWidth: true; height: 30; color: Theme.surface
            RowLayout {
                anchors.fill: parent; anchors.leftMargin: 18; anchors.rightMargin: 18
                Label { text: PaintClient.closing ? "正在关闭核心…" : PaintClient.fileBusy ? "正在读写文件…" : PaintClient.drawing ? "绘画中" : root.fileNotice.length ? root.fileNotice : "就绪"; color: Theme.accent; font.pixelSize: 10 }
                ToolButton { text: "取消文件任务"; visible: PaintClient.fileBusy; onClicked: PaintClient.cancelFile() }
                Label { text: "  ·  Rust CPU 视口 / Qt Quick 显示"; color: Theme.muted; font.pixelSize: 10 }
                Item { Layout.fillWidth: true }
                Label { text: "拖动面板标签组合 · 双击浮动 · 拖动顶部圆点移动整组"; color: Theme.muted; font.pixelSize: 10 }
                Label { text: "   "+Math.round(canvas.zoom*100)+"%"; color: Theme.text; font.pixelSize: 10 }
            }
        }
    }
    Shortcut { sequence: "V"; enabled: canvas.activeFocus; onActivated: PaintClient.moveTool=true }
    Shortcut { sequence: "B"; enabled: canvas.activeFocus; onActivated: PaintClient.eraser=false }
    Shortcut { sequence: "E"; enabled: canvas.activeFocus; onActivated: PaintClient.eraser=true }
    Shortcut { sequence: "F"; enabled: canvas.activeFocus; onActivated: canvas.fitToView() }
    Shortcut { sequence: "Esc"; enabled: PaintClient.drawing; onActivated: PaintClient.cancelStroke() }
    Shortcut { sequence: Qt.platform.os === "osx" ? "Meta+Alt+G" : "Ctrl+Alt+G"; context:Qt.ApplicationShortcut; enabled: PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy && !PaintClient.fileBusy; onActivated: PaintClient.toggleActiveClipping() }
    Instantiator {
        model: Workspace.floatingGroups
        delegate: FloatingPanel { required property var modelData; groupData: modelData; canvasView: canvas }
    }
    Dialog {
        id: newDialog
        title: "新建画布"; modal: true; anchors.centerIn: parent; width: 390
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: root.requestAction("new")
        ColumnLayout {
            width: parent.width; spacing: 14
            Label { text: "新建前可保存当前绘画。大画布按需分配瓦片。"; wrapMode: Text.WordWrap; Layout.fillWidth: true; color: Theme.muted }
            RowLayout { Label { text: "宽度" } SpinBox { id: newWidth; from: 1; to: 1000000; value: 960; editable: true } }
            RowLayout { Label { text: "高度" } SpinBox { id: newHeight; from: 1; to: 1000000; value: 640; editable: true } }
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
            ComboBox { id: panelKind; model: ["色板","画笔参数"]; Layout.fillWidth: true }
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
