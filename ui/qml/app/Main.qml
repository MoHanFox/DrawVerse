import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import QtQuick.Dialogs as FileDialogs
import DrawVerse 1.0
import "../canvas"
import "../controls"
import "../documents"
import "../menus"
import "../preferences"
import "../theme"
import "../tools"
import "../windows"
import "../workspace"

ApplicationWindow {
    AdjustmentWindow {id:adjustmentWindow;transientParent:root}
    id: root
    objectName: "mainWindow"
    visible: true
    flags: Qt.Window | Qt.FramelessWindowHint | Qt.WindowSystemMenuHint | Qt.WindowMinimizeButtonHint | Qt.WindowMaximizeButtonHint
    width: 1480; height: 780; minimumWidth: 980; minimumHeight: 640
    title: "DrawVerse"+(documents.activeId.length?" · "+PaintClient.documentName+(PaintClient.modified ? " *" : ""):"")
    color: "transparent"
    readonly property int cornerRadius: visibility===Window.Maximized || visibility===Window.FullScreen ? 0 : Theme.windowRadius
    property bool windowCornersReady: false
    function updateWindowCorners() {
        if(windowCornersReady && visibility!==Window.Minimized && visibility!==Window.Hidden) Workspace.setWindowCornerRadius(root,cornerRadius)
    }
    property int restoredVisibility:Window.Windowed
    property bool restoringFromMinimize:false
    property bool restoringNormalGeometry:false
    property rect normalGeometry:Qt.rect(0,0,1480,780)
    function rememberNormalGeometry() {
        if(windowCornersReady && visibility===Window.Windowed && !restoringFromMinimize && !restoringNormalGeometry)
            normalGeometry=Qt.rect(x,y,width,height)
    }
    onVisibilityChanged: visibility=>{
        if(visibility===Window.Minimized)restoringFromMinimize=true
        else if(visibility!==Window.Hidden) {
            if(restoringFromMinimize && restoredVisibility===Window.Maximized && visibility!==Window.Maximized) {
                Qt.callLater(()=>{
                    if(root.visibility!==Window.Windowed || !restoringFromMinimize)return
                    root.showMaximized();restoringFromMinimize=false
                })
            } else if(Workspace.windowsWindowFrames && visibility===Window.Windowed && restoredVisibility===Window.Maximized) {
                restoringFromMinimize=false;restoringNormalGeometry=true
                root.x=normalGeometry.x;root.y=normalGeometry.y
                root.width=normalGeometry.width;root.height=normalGeometry.height
                restoringNormalGeometry=false;restoredVisibility=Window.Windowed;Qt.callLater(updateWindowCorners)
            } else {restoringFromMinimize=false;restoredVisibility=visibility;Qt.callLater(updateWindowCorners)}
        }
    }
    onCornerRadiusChanged: Qt.callLater(updateWindowCorners)
    onXChanged:Qt.callLater(rememberNormalGeometry)
    onYChanged:Qt.callLater(rememberNormalGeometry)
    onWidthChanged: {Qt.callLater(updateWindowCorners);Qt.callLater(rememberNormalGeometry)}
    onHeightChanged: {Qt.callLater(updateWindowCorners);Qt.callLater(rememberNormalGeometry)}
    onScreenChanged: Qt.callLater(updateWindowCorners)
    Screen.onDevicePixelRatioChanged: Qt.callLater(updateWindowCorners)
    WindowOutline {hostWindow:root}
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
    // Explicit document bootstrap for embedded preview/benchmark hosts.
    property bool createInitialDocument: false
    property var startupClient: null
    readonly property bool canAdjustImage:hasDocuments && PaintClient.ready && !editing && !PaintClient.drawing && !PaintClient.fileBusy && !PaintClient.layerEditBusy && PaintClient.layers.some(function(layer){return layer.id===PaintClient.activeLayer && !layer.group && !layer.mask && !layer.image && !(layer.effectiveLocks & 4)})
    readonly property bool hasDocuments: documents.documents.length>0
    readonly property bool startupReady: startupClient!==null && startupClient.ready && startupClient.brushLibraryReady && startupClient.brushCatalogPrepared
    property bool quitting: false
    property string pendingAction: ""
    property url pendingOpen: ""
    property bool savingBeforeAction: false
    property string fileNotice: ""
    readonly property bool liquifying:PaintClient.liquifyActive
    readonly property bool editing:liquifying || PaintClient.transformActive
    onEditingChanged:if(!editing && (fileNotice==="请先确定或取消液化" || fileNotice==="请先确定或取消自由变换"))fileNotice=""
    property var floatingWindows: ({})
    property var dockingHint:null
    Component {id:hintFactory;DockHintWindow {}}
    function updateDockHint(){
        let edge=Workspace.dragPlacement.replace("column-","")
        const rect=Workspace.dragPreviewRect
        if(!Workspace.dragging || !Workspace.dragTarget || ["left","right","before","after"].indexOf(edge)<0 || rect.width<=0 || rect.height<=0){if(dockingHint){dockingHint.visible=false;dockingHint.destroy();dockingHint=null}return}
        if(!dockingHint)dockingHint=hintFactory.createObject(root,{targetRect:rect,edge:edge})
        else {dockingHint.targetRect=rect;dockingHint.edge=edge}
    }
    Connections {target:Workspace;function onDragModifiersChanged(){root.updateDockHint()}}
    property var documentWindows: ({})
    property var documentPanes: ({})
    property string initialDocumentId:""
    property var canvas: documentPane.canvasView
    signal newDocumentRequested()
    onNewDocumentRequested:newDialog.open()
    readonly property bool documentsBlocked:root.savingBeforeAction || closeDialog.opened || busyCloseDialog.opened || saveDialog.visible || exportDialog.visible
    DocumentManager {id:documents;objectName:"documentManager";interactionBlocked:root.documentsBlocked || (PaintClient.liquifyActive || PaintClient.transformActive)}
    Item {id:canvasParking;visible:false}
    CanvasPane {id:documentPane;parent:canvasParking;initialDocument:true;parkingParent:canvasParking;mainWindow:root;savingBlocked:root.documentsBlocked || PaintClient.liquifyActive && !documentPane.ownClient.liquifyActive}
    function paneFor(id){return documentPanes[id] || null}
    function isInitial(id){return id===initialDocumentId}
    property bool syncingDocuments:false
    function syncDocuments(){
        if(syncingDocuments){Qt.callLater(syncDocuments);return}
        syncingDocuments=true
        try {
        const live={},all=documents.documents
        for(let i=0;i<all.length;i++) {
            const d=all[i];live[d.id]=true
            if(!documentPanes[d.id]) {
                if(!initialDocumentId){initialDocumentId=d.id;documentPane.ownClient=d.client;documentPane.documentId=d.id;documentPanes[d.id]=documentPane}
                else documentPanes[d.id]=documentFactory.createObject(canvasParking,{documentId:d.id,ownClient:d.client,parkingParent:canvasParking,mainWindow:root,savingBlocked:Qt.binding(()=>root.documentsBlocked || PaintClient.liquifyActive && !d.client.liquifyActive)})
            }
        }
        for(const id in documentPanes)if(!live[id]) {documentPanes[id].park();if(id!==initialDocumentId)documentPanes[id].destroy();delete documentPanes[id]}
        const windows=documents.windows,keep={}
        for(let i=0;i<windows.length;i++) {const g=windows[i];keep[g.id]=true;if(documentWindows[g.id])documentWindows[g.id].updateLayout(g);else documentWindows[g.id]=documentWindowFactory.createObject(root,{windowData:g,documents:documents,presentation:root})}
        for(const id in documentWindows)if(!keep[id]){documentWindows[id].releaseCanvas();documentWindows[id].visible=false;documentWindows[id].destroy();delete documentWindows[id]}
        const active=paneFor(documents.activeId);canvas=active?active.canvasView:documentPane.canvasView
        if(mainDocumentArea)mainDocumentArea.refresh()
        } finally {syncingDocuments=false}
    }
    Component {id:documentFactory;CanvasPane {}}
    Component {id:documentWindowFactory;DocumentWindow {}}
    DocumentArea {id:mainDocumentArea;parent:canvasParking;documents:documents;presentation:root;function park(){parent=canvasParking}function attach(host){parent=host;width=Qt.binding(()=>host.width);height=Qt.binding(()=>host.height)}}
    Connections {target:documents;function onDocumentsChanged(){root.syncDocuments()}function onGroupsChanged(){root.syncDocuments()}function onActiveChanged(){root.syncDocuments()}function onCloseRequested(id){documents.activate(id);root.pendingDocument=id;root.requestAction("closeDocument")}function onFileFailed(message){root.fileNotice=message}function onStopped(){Qt.quit()}}
    function syncFloating() {
        const groups=Workspace.floatingWindows, live={}
        for(let i=0;i<groups.length;i++) {
            const g=groups[i]; live[g.id]=true
            if(floatingWindows[g.id]) floatingWindows[g.id].updateLayout(g)
            else floatingWindows[g.id]=floatingFactory.createObject(root,{windowData:g,canvasPane:mainDocumentArea,canvasView:Qt.binding(()=>root.canvas),appClosing:Qt.binding(()=>root.quitting),liquifyHidden:Qt.binding(()=>root.liquifying),workspaceHidden:Qt.binding(()=>!root.hasDocuments)})
        }
        for(const id in floatingWindows) if(!live[id]) {
            floatingWindows[id].releaseCanvas();floatingWindows[id].visible=false; floatingWindows[id].destroy(); delete floatingWindows[id]
        }
    }
    property bool menuBarBlurActive: false
    Component.onCompleted: {
        documentPane.ownClient=PaintClient
        startupClient=PaintClient
        documents.configure(PaintClient,root,createInitialDocument)
        // Set initial placement once; live size bindings move maximized windows.
        x=Screen.virtualX+Math.max(16,(Screen.width-width)/2)
        y=Screen.virtualY+Math.max(16,(Screen.height-height)/3)
        normalGeometry=Qt.rect(x,y,width,height)
        menuBarBlurActive=Workspace.setMenuBarBlur(root,true,root.menuBar.height)
        windowCornersReady=true
        updateWindowCorners()
        syncFloating()
    }
    Connections { target: Workspace; function onGroupsChanged() { root.syncFloating() } }
    Component { id: floatingFactory; FloatingPanel {} }
    onActiveChanged:if(active && documents.activeId && documents.group("main").active && documents.activeId!==documents.group("main").active)documents.activate(documents.group("main").active)
    property string pendingDocument:""
    property bool exiting:false
    function nextExit(){
        if(documents.documents.length){pendingDocument=documents.documents[0].id;documents.activate(pendingDocument);requestAction("closeDocument")}
        else quitApp()
    }
    function executePending() {
        const action = pendingAction
        pendingAction = ""; savingBeforeAction = false
        if (action === "close") {exiting=true;nextExit()}
        else if(action==="closeDocument") {documents.closeDocument(pendingDocument,true);pendingDocument="";if(exiting)Qt.callLater(nextExit)}
        else if (action === "open") documents.openDocument(pendingOpen)
        else if (action === "new") documents.newDocument(newWidth.value,newHeight.value,transparentBackground.checked)
    }
    function requestAction(action) {
        if(editing){fileNotice=PaintClient.transformActive?"请先确定或取消自由变换":"请先确定或取消液化";return}

        pendingAction = action
        if (action==="close") {executePending();return}
        if (action==="closeDocument" && PaintClient.fileBusy) busyCloseDialog.open()
        else if (action==="closeDocument" && PaintClient.modified) closeDialog.open()
        else executePending()
    }
    function saveCurrent() {
        if(!hasDocuments)return
        if (PaintClient.documentUrl.toString().length > 0) {
            if (!PaintClient.saveDocument(PaintClient.documentUrl)) {savingBeforeAction=false;pendingAction="";exiting=false}
        } else saveDialog.open()
    }
    function quitApp() { Workspace.saveLayout(); quitting=true; documents.shutdown() }
    onClosing: event => {
        if (quitting || (documents.documents.length<=1 && PaintClient.closing)) { event.accepted=true; return }
        event.accepted=false
        if (quitting) return
        if (PaintClient.drawing) PaintClient.endStroke()
        requestAction("close")
    }
    ResizeFrame {targetWindow:root}
    StoragePreferences { id: storagePreferences; objectName: "storagePreferences" }
    PreferencesPanel { id: preferencesDialog; objectName: "preferencesDialog" }
    KeyboardShortcutsDialog { id: shortcutsDialog; objectName: "keyboardShortcuts" }
    menuBar: MenuBar {
        objectName: "mainMenuBar"
        implicitHeight: 28; leftPadding: 30; rightPadding: 100
        background: MenuSurface {
            objectName: "menuBarGlassBackground"
            tint: Theme.menuBarGlass
            topCornersOnly: true
            radius: root.cornerRadius
            MouseArea {
                objectName:"mainWindowDragArea"
                anchors.fill: parent
                property bool restoreDrag:false
                property bool movingRestored:false
                property bool movingNormal:false
                property point pressGlobal
                property real pressFraction:0
                property real pressY:0
                property point moveOffset
                onPressed: mouse=>{
                    restoreDrag=root.visibility===Window.Maximized;movingRestored=false;movingNormal=false
                    pressGlobal=mapToGlobal(mouse.x,mouse.y)
                    pressFraction=mouse.x/width;pressY=mouse.y
                }
                onPositionChanged: mouse=>{
                    if(!pressed || root.visibility===Window.FullScreen)return
                    const global=mapToGlobal(mouse.x,mouse.y)
                    if(!restoreDrag) {
                        if(!movingNormal && Math.abs(global.x-pressGlobal.x)+Math.abs(global.y-pressGlobal.y)>=Qt.styleHints.startDragDistance){movingNormal=true;root.startSystemMove()}
                        return
                    }
                    if(!movingRestored) {
                        if(Math.abs(global.x-pressGlobal.x)+Math.abs(global.y-pressGlobal.y)<Qt.styleHints.startDragDistance)return
                        root.showNormal()
                        root.updateWindowCorners()
                        moveOffset=Qt.point(Math.round(root.width*pressFraction),pressY)
                        movingRestored=true
                        Workspace.continueRestoredWindowDrag(root,global,moveOffset)
                    }
                    root.x=Math.round(global.x-moveOffset.x);root.y=Math.round(global.y-moveOffset.y)
                }
                onReleased:{restoreDrag=false;movingRestored=false;movingNormal=false}
                onCanceled:{restoreDrag=false;movingRestored=false;movingNormal=false}
                onDoubleClicked: {restoreDrag=false;movingRestored=false;movingNormal=false;if(root.visibility===Window.Maximized)root.showNormal();else root.showMaximized()}
            }
            Row {
                anchors.right: parent.right; height: parent.height
                IconButton { objectName:"windowMinimize";width:32;height:28;padding:10;glyph:"minimize";tooltip:"最小化";onClicked:root.showMinimized() }
                IconButton { objectName:"windowMaximize";width:32;height:28;padding:10;glyph:root.visibility===Window.Maximized?"restore":"maximize";tooltip:"最大化 / 还原";onClicked:{if(root.visibility===Window.Maximized)root.showNormal();else root.showMaximized()} }
                IconButton {
                    id: closeButton
                    objectName:"windowClose";width:32;height:28;padding:10;glyph:"close";tooltip:"关闭";onClicked:root.close()
                    background: MenuSurface {
                        objectName: "windowCloseBackground"
                        radius: root.cornerRadius
                        topRightCornerOnly: true
                        tint: closeButton.down ? "#b5222c" : closeButton.hovered ? "#f04450" : "transparent"
                    }
                    contentItem: Icon { name: "close"; color: "#ffffff" }
                }
            }
            Image { objectName: "applicationLogo"; x: 8; y: 5; width: 18; height: 18; source: "../assets/logo.png"; fillMode: Image.PreserveAspectFit; mipmap: true }
        }
        delegate: MenuBarItem {
            id: menuEntry
            hoverEnabled:true
            objectName: "menuEntry:"+text
            implicitHeight: 28; implicitWidth:contentItem.implicitWidth+16
            leftPadding:8;rightPadding:8;font.pixelSize:9
            contentItem: Text { text: menuEntry.text; font: menuEntry.font; color: menuEntry.highlighted ? "#eeeeee" : Theme.text; verticalAlignment: Text.AlignVCenter }
            background: Rectangle { radius: 0; color: menuEntry.highlighted ? Theme.hover : "transparent" }
        }
        GlassMenu {
            objectName:"fileMenuPopup";topLevel:true
            title: "文件"
            Action { text: "新建画布…"; shortcut: ShortcutSettings.bindings.new; enabled: root.startupReady && (!root.hasDocuments || PaintClient.ready && !PaintClient.drawing && !PaintClient.fileBusy); onTriggered: newDialog.open() }
            Action { text: "打开…"; shortcut: ShortcutSettings.bindings.open; enabled: root.startupReady && (!root.hasDocuments || PaintClient.ready && !PaintClient.drawing && !PaintClient.fileBusy); onTriggered: openDialog.open() }
            MenuSeparator {}
            Action { text: "保存"; shortcut: ShortcutSettings.bindings.save; enabled: root.hasDocuments && (!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.ready && !PaintClient.drawing && !PaintClient.fileBusy); onTriggered: root.saveCurrent() }
            Action { text: "另存为 OpenRaster…"; shortcut: ShortcutSettings.bindings.saveAs; enabled: root.hasDocuments && (!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.ready && !PaintClient.drawing && !PaintClient.fileBusy); onTriggered: saveDialog.open() }
            MenuSeparator {}
            Action { text: "导出为 PNG…"; enabled: root.hasDocuments && (!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.ready && !PaintClient.drawing && !PaintClient.fileBusy); onTriggered: exportDialog.open() }
            Action { text: "导出 WebP…"; enabled: false }
            Action { text: "导出 JPEG…"; enabled: false }
            Action { text: "导出 TIFF…（待支持）"; enabled: false }
            Action { text: "导出 GIF…（待支持）"; enabled: false }
            MenuSeparator {}
            Action { objectName:"placeImageAction";text:"置入图片…";enabled: root.hasDocuments && (PaintClient.ready && !PaintClient.fileBusy && !PaintClient.drawing && !root.editing);onTriggered:placeDialog.open() }
            MenuSeparator {}
            Action { text: "退出"; onTriggered: root.close() }
        }
        GlassMenu {
            topLevel:true
            title: "编辑"
            Action { objectName:"storagePreferencesAction"; text: "首选项…"; shortcut: ShortcutSettings.bindings.preferences; enabled: !PaintClient.closing; onTriggered: preferencesDialog.open() }
            Action { objectName:"keyboardShortcutsAction"; text: "键盘快捷键…"; enabled: !PaintClient.closing; onTriggered: shortcutsDialog.open() }
            MenuSeparator {}
            Action { text: "撤销"; shortcut: ShortcutSettings.bindings.undo; enabled: root.hasDocuments && (PaintClient.undoDepth>0 && !PaintClient.drawing); onTriggered: PaintClient.undo() }
            Action {objectName:"freeTransformAction";text:"自由变换";shortcut: ShortcutSettings.bindings.transform;enabled: root.hasDocuments && (!PaintClient.textInputFocused && PaintClient.ready && !root.editing && !PaintClient.drawing && !PaintClient.fileBusy);onTriggered:PaintClient.beginTransform()}
            Action { text: "重做"; shortcut: ShortcutSettings.bindings.redo; enabled: root.hasDocuments && (PaintClient.redoDepth>0 && !PaintClient.drawing); onTriggered: PaintClient.redo() }
        }
        GlassMenu {
            id:imageMenu;objectName:"imageMenu";topLevel:true
            title: "图像"
            GlassMenu {
                objectName:"imageAdjustSubmenu";title:"调整";cascadeParent:imageMenu
                Action { objectName:"adjustCurvesAction";shortcut: ShortcutSettings.bindings.curves;text:"曲线…";enabled:root.canAdjustImage;onTriggered:adjustmentWindow.openFor(0) }
                Action { objectName:"adjustLevelsAction";shortcut: ShortcutSettings.bindings.levels;text:"色阶…";enabled:root.canAdjustImage;onTriggered:adjustmentWindow.openFor(1) }
                Action { objectName:"adjustHsvAction";shortcut: ShortcutSettings.bindings.hsv;text:"色相/饱和度/明度…";enabled:root.canAdjustImage;onTriggered:adjustmentWindow.openFor(2) }
            }
            MenuSeparator {}
            Action { objectName:"canvasSizeAction";text:"画布大小…（待支持）";enabled:false }
            GlassMenu {
                objectName:"imageRotateSubmenu";title:"画布旋转";cascadeParent:imageMenu
                Action { objectName:"canvasRotateCwAction";text:"顺时针 90°（待支持）";enabled:false }
                Action { objectName:"canvasRotateCcwAction";text:"逆时针 90°（待支持）";enabled:false }
                Action { objectName:"canvasFlipVerticalAction";text:"垂直翻转";enabled: root.hasDocuments && (PaintClient.ready && !PaintClient.fileBusy && !PaintClient.drawing && !PaintClient.layerEditBusy && !PaintClient.transformActive && !PaintClient.liquifyActive);onTriggered:PaintClient.flipImage(false) }
                Action { objectName:"canvasFlipHorizontalAction";text:"水平翻转";enabled: root.hasDocuments && (PaintClient.ready && !PaintClient.fileBusy && !PaintClient.drawing && !PaintClient.layerEditBusy && !PaintClient.transformActive && !PaintClient.liquifyActive);onTriggered:PaintClient.flipImage(true) }
            }
            MenuSeparator {}
            Action { text: "图像大小…（待支持）"; enabled: false }
            Action { text: "裁剪…（待支持）"; enabled: false }
        }
        GlassMenu {
            topLevel:true
            title: "图层"
            Action {objectName:"rasterizeImageAction";text:"栅格化图片";enabled: root.hasDocuments && (PaintClient.activeImage && !root.editing && !PaintClient.fileBusy && !PaintClient.drawing);onTriggered:PaintClient.rasterizeImage()}
            Action { text: "新建图层"; enabled: root.hasDocuments && (!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy); onTriggered: PaintClient.addDefaultLayer() }
            Action {objectName:"duplicateLayersAction";text:"复制图层";enabled: root.hasDocuments && (!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy && !PaintClient.fileBusy);onTriggered:PaintClient.duplicateSelectedLayers()}
            Action {objectName:"mergeLayersAction";text:PaintClient.mergeLayersText;enabled: root.hasDocuments && (PaintClient.canMergeLayers);onTriggered:PaintClient.mergeSelectedLayers()}
            Action { text: "删除图层"; enabled: root.hasDocuments && (!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.ready && !PaintClient.drawing && PaintClient.layers.length>0); onTriggered: PaintClient.requestSelectedLayerRemoval() }
            MenuSeparator {}
            Action { objectName:"newLayerGroupAction";text: "新建组"; enabled: root.hasDocuments && (!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy); onTriggered: PaintClient.groupSelectedLayers("组") }
            Action { text: "解组（移除组属性）"; enabled: root.hasDocuments && (!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy); onTriggered: PaintClient.ungroupLayer(PaintClient.activeLayer) }
            MenuSeparator {}
            Action { text: "添加白色蒙版"; enabled: root.hasDocuments && (!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy); onTriggered: PaintClient.addMask(PaintClient.activeLayer) }
            Action { text: "创建剪贴蒙版"; enabled: root.hasDocuments && (!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.ready && !PaintClient.drawing && !PaintClient.fileBusy && !PaintClient.layerEditBusy); onTriggered: PaintClient.toggleActiveClipping() }
        }
        GlassMenu {
            topLevel:true
            title: "选择"
            Action { text: "矩形选框（M）"; enabled: root.hasDocuments && (!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.ready && !PaintClient.drawing); onTriggered: PaintClient.selectionTool=1 }
            Action { text: "椭圆选框（Shift+M）"; enabled: root.hasDocuments && (!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.ready && !PaintClient.drawing); onTriggered: PaintClient.selectionTool=2 }
            MenuSeparator {}
            Action { objectName:"selectionAllAction"; text: "全选"; enabled: root.hasDocuments && (!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy); onTriggered: PaintClient.selectAll() }
                Action { objectName:"selectionLayerPixelsAction"; text: "选取不透明像素"; enabled: root.hasDocuments && PaintClient.ready && !root.editing && !PaintClient.drawing && !PaintClient.layerEditBusy && !PaintClient.fileBusy; onTriggered: PaintClient.selectLayerPixels() }
            Action { objectName:"selectionClearAction"; text: "取消选择"; enabled: root.hasDocuments && (PaintClient.selectionEnabled && !PaintClient.drawing && !PaintClient.layerEditBusy); onTriggered: PaintClient.clearSelection() }
            Action { objectName:"selectionInvertAction"; text: "反向选择"; enabled: root.hasDocuments && (!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy); onTriggered: PaintClient.invertSelection() }
        }
        GlassMenu {
            id:filterMenu;objectName:"filterMenu";topLevel:true
            title: "滤镜"
            Action {objectName:"liquifyAction";text:"液化…";enabled: root.hasDocuments && (!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy && !PaintClient.fileBusy);onTriggered:PaintClient.beginLiquify()}
            MenuSeparator {}
            GlassMenu {
                objectName: "filterBlurSubmenu"
                title: "模糊";cascadeParent:filterMenu
                Action { objectName:"filterGaussianBlurAction"; text: "高斯模糊…（待支持）"; enabled: false }
                Action { text: "动感模糊…（待支持）"; enabled: false }
                Action { text: "径向模糊…（待支持）"; enabled: false }
                Action { text: "方框模糊…（待支持）"; enabled: false }
            }
            Action { text: "锐化…（待支持）"; enabled: false }
            Action { text: "杂色…（待支持）"; enabled: false }
            Action { text: "风格化…（待支持）"; enabled: false }
            MenuSeparator {}
            Action { text: "像素化…（待支持）"; enabled: false }
            Action { text: "扭曲…（待支持）"; enabled: false }
            Action { text: "渲染…（待支持）"; enabled: false }
        }
        GlassMenu {
            topLevel:true
            id: windowMenu
            objectName: "windowMenu"
            title: "窗口"
            Action {text:"工具条归位";enabled:Workspace.toolsFloating;onTriggered:Workspace.dockToolStrip("drawverse-tools-v1")}
            Action {text:"浮动画布";enabled: root.hasDocuments && (documents.activeId.length>0 && !PaintClient.drawing);onTriggered:documents.floatDocument(documents.activeId)}
            Action {text:"画布返回工作区";enabled: root.hasDocuments && (documents.activeId.length>0 && !PaintClient.drawing);onTriggered:documents.moveDocument(documents.activeId,"main")}
            MenuSeparator {
                objectName:"windowMenuSeparator:tools"
                contentItem: Rectangle { implicitWidth: 200; implicitHeight: 1; color: Theme.line }
            }
            Action { enabled:root.hasDocuments; text: "适合窗口"; shortcut: ShortcutSettings.bindings.fit; onTriggered: canvas.fitToView() }
            Action { enabled:root.hasDocuments; text: "实际像素"; onTriggered: canvas.actualSize() }
            MenuSeparator {
                objectName:"windowMenuSeparator:view"
                contentItem: Rectangle { implicitWidth: 200; implicitHeight: 1; color: Theme.line }
            }
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
            Action { objectName:"customPanelAction"; text: "自定义面板…"; onTriggered: panelDialog.open() }
            Action { text: "保存当前布局"; onTriggered: Workspace.saveLayout() }
            Action { objectName:"defaultLayoutAction";text: "默认布局"; enabled: root.hasDocuments && (!PaintClient.drawing);onTriggered: Workspace.resetLayout() }
        }
    }
    header: Rectangle {
        objectName: "brushOptionsBar"
        visible:root.hasDocuments
        height: visible?36:0; color: Theme.raised
        RowLayout {
            visible: !PaintClient.liquifyActive && !PaintClient.transformActive && !PaintClient.selectionTool && !PaintClient.eyedropperTool && !PaintClient.bucketTool && !PaintClient.smudgeTool
            anchors.fill: parent; anchors.leftMargin: 12; anchors.rightMargin: 12; spacing: 8
            Icon { name: PaintClient.selectionTool ? (PaintClient.selectionTool===2 ? "ellipseSelection":"rectangleSelection") : PaintClient.moveTool ? "move" : PaintClient.eraser ? "eraser" : "brush"; Layout.preferredWidth: 17; Layout.preferredHeight: 17 }
            Label { text: PaintClient.selectionTool ? (PaintClient.selectionTool===2?"椭圆选框":"矩形选框") : PaintClient.moveTool ? "移动图层" : PaintClient.eraser ? "橡皮擦" : "画笔"; color: Theme.text; Layout.preferredWidth: 42 }
            Rectangle { width: 1; height: 22; color: Theme.line }
            Label {visible:!PaintClient.moveTool && !PaintClient.eraser;text:"模式";color:Theme.muted}
            CompactComboBox {highPriorityMenu:true;objectName:"brushWriteBlend";visible:!PaintClient.moveTool && !PaintClient.eraser;implicitWidth:126;Layout.preferredWidth:126;model:["正常","溶解","变暗","正片叠底","颜色加深","线性加深","深色","变亮","滤色","颜色减淡","线性减淡（添加）","浅色","叠加","柔光","强光","亮光","线性光","点光","实色混合","差值","排除","减去","划分","色相","饱和度","颜色","明度"];currentIndex:PaintClient.brushWriteBlend;onActivated:PaintClient.brushWriteBlend=currentIndex;tooltip:"笔触颜色直接按此模式与目标层像素混合；不改变图层模式。"}
            Label { text: "大小"; color: Theme.muted }
            CompactSpinBox { objectName: "brushRadius"; from: 1; to: 2000; value: Math.round(PaintClient.brushRadius*2); editable: true; implicitWidth: 76; implicitHeight: 20; onValueModified: PaintClient.brushRadius=value/2 }
            Label { text: "px"; color: Theme.muted }
            Label { visible:!PaintClient.moveTool;text: "不透明度"; color: Theme.muted }
            CompactSlider {objectName:"brushToolbarOpacity";visible:!PaintClient.moveTool;from:0;to:1;stepSize:.01;value:PaintClient.brushLibrary.opacity;Layout.preferredWidth:85;onMoved:PaintClient.brushLibrary.opacity=value;tooltip:"当前笔刷的整笔不透明度上限；重复覆盖不会超过此上限。"}
            Label {visible:!PaintClient.moveTool;text:Math.round(PaintClient.brushLibrary.opacity*100)+"%";color:Theme.text;Layout.preferredWidth:32}
            CompactCheckBox {objectName:"brushToolbarPressureOpacity";visible:!PaintClient.moveTool;text:"压感";checked:PaintClient.brushLibrary.pressureOpacity;onToggled:PaintClient.brushLibrary.pressureOpacity=checked;tooltip:"使用数位笔压力控制局部整笔不透明度上限；关闭时保留压力曲线。"}
            Label {visible:!PaintClient.moveTool;text:"流量";color:Theme.muted}
            CompactSlider {objectName:"brushToolbarFlow";visible:!PaintClient.moveTool;from:0;to:1;stepSize:.01;value:PaintClient.brushLibrary.flow;Layout.preferredWidth:85;onMoved:PaintClient.brushLibrary.flow=value;tooltip:"当前笔刷每个印章的流量，控制颜色或擦除累积到整笔上限的速度。"}
            Label {visible:!PaintClient.moveTool;text:Math.round(PaintClient.brushLibrary.flow*100)+"%";color:Theme.text;Layout.preferredWidth:32}
            CompactCheckBox {objectName:"brushToolbarPressureFlow";visible:!PaintClient.moveTool;text:"压感";checked:PaintClient.brushLibrary.pressureFlow;onToggled:PaintClient.brushLibrary.pressureFlow=checked;tooltip:"使用数位笔压力控制绘画或擦除流量；关闭时保留压力曲线。"}
            Item { Layout.fillWidth: true }
        }
        TransformOptionsBar {visible:PaintClient.transformActive;anchors.fill:parent;anchors.leftMargin:8;anchors.rightMargin:8;controller:PaintClient}
        LiquifyOptionsBar {visible:PaintClient.liquifyActive;anchors.fill:parent;anchors.leftMargin:8;anchors.rightMargin:8;controller:PaintClient}
        SelectionOptionsBar {
            visible:!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.selectionTool>0 && PaintClient.selectionTool<5
            anchors.fill:parent;anchors.leftMargin:12;anchors.rightMargin:12
            client:PaintClient
        }
        BucketOptionsBar {
            visible:!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.bucketTool
            anchors.fill:parent;anchors.leftMargin:8;anchors.rightMargin:8
            client:PaintClient
        }
        WandOptionsBar {
            visible:!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.selectionTool===5
            anchors.fill:parent;anchors.leftMargin:8;anchors.rightMargin:8
            client:PaintClient
        }
        SmudgeOptionsBar {
            visible:!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.smudgeTool
            anchors.fill:parent;anchors.leftMargin:8;anchors.rightMargin:8
            client:PaintClient
        }
        RowLayout {
            visible:!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.eyedropperTool
            anchors.fill:parent;anchors.leftMargin:12;anchors.rightMargin:12;spacing:8
            Icon {name:"eyedropper";Layout.preferredWidth:17;Layout.preferredHeight:17}
            Label {text:"吸色";color:Theme.text}
            Label {text:"按下并拖动采样合成颜色";color:Theme.muted}
            CheckBox {
                objectName:"showSamplingRing";text:"显示取色环";checked:PaintClient.samplingRingEnabled
                onToggled:PaintClient.samplingRingEnabled=checked
                hoverEnabled:true;ToolTip.delay:1000;ToolTip.visible:hovered
                ToolTip.text:"按下取色时，上半环显示当前取样色，下半环显示取色前的前景色。"
                font.pixelSize:12;padding:0;spacing:6
                indicator:Rectangle {
                    implicitWidth:13;implicitHeight:13;x:0;y:(parent.height-height)/2
                    radius:2;color:parent.checked?Theme.text:Theme.input;border.color:Theme.muted
                    Icon {anchors.fill:parent;anchors.margins:2;name:"check";color:Theme.input;visible:parent.parent.checked}
                }
                contentItem:Text {text:parent.text;font:parent.font;color:Theme.text;leftPadding:19;verticalAlignment:Text.AlignVCenter}
            }
            Item {Layout.fillWidth:true}
        }
        Rectangle { anchors.bottom: parent.bottom; height: 1; width: parent.width; color: Theme.panelBar }
    }
    WelcomeScreen {
        objectName:"welcomeScreen"
        anchors.fill:parent;visible:!root.hasDocuments
        ready:root.startupReady
        stage:!root.startupClient || !root.startupClient.brushLibraryReady?"正在加载笔刷与偏好设置…":!root.startupClient.ready?"正在初始化绘画引擎…":"正在准备笔刷资源…"
        error:root.fileNotice.length?root.fileNotice:PaintClient.lastError
        onNewCanvasRequested:newDialog.open()
        onOpenRequested:openDialog.open()
    }
    ColumnLayout {
        objectName:"editorWorkspace"
        visible:root.hasDocuments
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
            DockWorkspace {objectName:"mainDockWorkspace";Layout.fillWidth:true;Layout.fillHeight:true;canvasPane:mainDocumentArea;canvasView:canvas}
        }
        WorkspaceStatusBar {
            Layout.fillWidth:true
            client:PaintClient;canvasView:root.canvas;hasDocument:documents.activeId.length>0
            cornerRadius:root.cornerRadius;notice:root.fileNotice
        }
    }
    Shortcut {context:Qt.ApplicationShortcut;sequence: ShortcutSettings.bindings.copy;enabled: root.hasDocuments && (PaintClient.layerShortcutsAllowed);onActivated:PaintClient.copySelectedLayers()}
    Shortcut {context:Qt.ApplicationShortcut;sequence: ShortcutSettings.bindings.cut;enabled: root.hasDocuments && (PaintClient.layerShortcutsAllowed);onActivated:PaintClient.cutSelectedLayers()}
    Shortcut {context:Qt.ApplicationShortcut;sequence: ShortcutSettings.bindings.paste;enabled: root.hasDocuments && (PaintClient.layerShortcutsAllowed && PaintClient.canPasteLayers);onActivated:PaintClient.pasteLayers()}
    Shortcut {context:Qt.ApplicationShortcut;sequence: ShortcutSettings.bindings.delete;enabled: root.hasDocuments && (PaintClient.layerShortcutsAllowed);onActivated:PaintClient.deleteSelectionOrLayers()}
    Shortcut {context:Qt.ApplicationShortcut;sequence: ShortcutSettings.bindings.duplicate;enabled: root.hasDocuments && (PaintClient.layerShortcutsAllowed);onActivated:PaintClient.duplicateSelectedLayers()}
    Shortcut {context:Qt.ApplicationShortcut;sequence: ShortcutSettings.bindings.merge;enabled: root.hasDocuments && (PaintClient.layerShortcutsAllowed && PaintClient.canMergeLayers);onActivated:PaintClient.mergeSelectedLayers()}
    Shortcut {context:Qt.ApplicationShortcut;sequence: ShortcutSettings.bindings.group;enabled: root.hasDocuments && (PaintClient.layerShortcutsAllowed);onActivated:PaintClient.groupSelectedLayers("组")}
    Shortcut { context:Qt.ApplicationShortcut; sequence: "Esc"; enabled: root.hasDocuments && (PaintClient.drawing); onActivated: PaintClient.cancelStroke() }
    Shortcut { context:Qt.ApplicationShortcut; sequence: ShortcutSettings.bindings.clip; enabled: root.hasDocuments && (!PaintClient.liquifyActive && !PaintClient.transformActive && PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy && !PaintClient.fileBusy); onActivated: PaintClient.toggleActiveClipping() }
    Connections {
        target:PaintClient
        function onGroupDeletionRequested() {groupDeleteDialog.owner=PaintClient;groupDeleteDialog.open()}
    }
    Dialog {
        id:groupDeleteDialog;objectName:"groupDeleteDialog"
        property var owner:null
        title:"删除图层组";modal:true;anchors.centerIn:parent;width:340
        background:Rectangle {color:Theme.surface;border.color:Theme.categorySeparator;radius:3}
        onClosed:{if(owner)owner.confirmSelectedLayerRemoval(0);owner=null}
        ColumnLayout {
            width:parent.width;spacing:10
            Label {text:"如何处理选中的图层组？";color:Theme.text;Layout.fillWidth:true}
            Label {text:"仅删除组会保留内部图层，移除组的属性及蒙版。";color:Theme.muted;wrapMode:Text.WordWrap;Layout.fillWidth:true;font.pixelSize:10}
            Button {objectName:"deleteGroupKeepChildren";text:"仅删除组，保留内部图层";Layout.fillWidth:true;onClicked:{groupDeleteDialog.owner.confirmSelectedLayerRemoval(1);groupDeleteDialog.close()}}
            Button {objectName:"deleteGroupSubtree";text:"删除组及全部图层";Layout.fillWidth:true;onClicked:{groupDeleteDialog.owner.confirmSelectedLayerRemoval(2);groupDeleteDialog.close()}}
            Button {objectName:"deleteGroupCancel";text:"取消";Layout.fillWidth:true;onClicked:groupDeleteDialog.close()}
        }
    }
    Dialog {
        id: newDialog;objectName:"newDocumentDialog"
        title: "新建画布"; modal: true; anchors.centerIn: parent; width: 390
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: root.requestAction("new")
        ColumnLayout {
            width: parent.width; spacing: 14
            Label { text: "创建独立文档，当前绘画保持打开。大画布按需分配瓦片。"; wrapMode: Text.WordWrap; Layout.fillWidth: true; color: Theme.muted }
            RowLayout { Label { text: "宽度" } CompactSpinBox { id: newWidth; from: 1; to: 1000000; value: 960; editable: true } }
            RowLayout { Label { text: "高度" } CompactSpinBox { id: newHeight; from: 1; to: 1000000; value: 640; editable: true } }
            CheckBox {id:transparentBackground;objectName:"transparentBackground";text:"透明背景";checked:false}
            Label { text: "默认白色背景 · 线性 sRGB · RGBA 32F"; color: Theme.muted; font.pixelSize: 11 }
        }
    }
    Dialog {
        id: panelDialog; objectName:"customPanelDialog"
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
        id: closeDialog;objectName:"unsavedDocumentDialog"
        title: "保存当前绘画？"; modal: true; anchors.centerIn: parent; width: 390
        standardButtons: Dialog.Save | Dialog.Discard | Dialog.Cancel
        onAccepted: { root.savingBeforeAction=true; root.saveCurrent() }
        onDiscarded: root.executePending()
        onRejected: { root.pendingAction=""; root.savingBeforeAction=false;root.exiting=false }
        Label { text: "当前绘画尚未保存。OpenRaster 保存图层，工作区布局会保留。"; width: parent.width; wrapMode: Text.WordWrap; color: Theme.muted }
    }
    Dialog {
        id: busyCloseDialog;objectName:"busyDocumentDialog"
        title: "文件操作尚未完成"; modal: true; anchors.centerIn: parent; width: 390
        standardButtons: Dialog.Discard | Dialog.Cancel
        onDiscarded: root.executePending()
        onRejected: {root.pendingAction="";root.exiting=false}
        Label { text: "关闭会取消未完成的文件任务，并丢弃尚未保存的绘画。"; width: parent.width; wrapMode: Text.WordWrap; color: Theme.muted }
    }
    FileDialogs.FileDialog {id:placeDialog;title:"置入图片";nameFilters:["图片 (*.png *.jpg *.jpeg *.webp)"];onAccepted:PaintClient.placeImage(selectedFile)}
    FileDialogs.FileDialog {
        id: openDialog; objectName:"openDocumentDialog"; title: "打开绘画或图片"
        nameFilters: ["绘画与图片 (*.ora *.png *.jpg *.jpeg *.webp)"]
        onAccepted: { root.pendingOpen=selectedFile; Qt.callLater(()=>root.requestAction("open")) }
    }
    FileDialogs.FileDialog {
        id: saveDialog; title: "保存图层 · OpenRaster（8 位 sRGB）"
        fileMode: FileDialogs.FileDialog.SaveFile; defaultSuffix: "ora"
        nameFilters: ["OpenRaster (*.ora)"]
        onAccepted: { if (!PaintClient.saveDocument(selectedFile)) {root.savingBeforeAction=false;root.pendingAction="";root.exiting=false} }
        onRejected: { root.savingBeforeAction=false; root.pendingAction="";root.exiting=false }
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
            root.fileNotice=success ? "" : "文件操作未完成"
            if (root.savingBeforeAction) {
                root.savingBeforeAction=false
                if (success && !PaintClient.modified) root.executePending()
                else {
                    root.pendingAction="";root.exiting=false
                    if (success) root.fileNotice="保存期间又有修改，请再次保存后继续"
                }
            }
        }
    }
}
