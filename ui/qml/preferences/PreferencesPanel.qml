import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs as FileDialogs
import "../theme"

Dialog {
    id: root
    objectName: "preferencesDialog"
    title: "首选项"
    modal: true
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: 640
    implicitHeight: 520
    property int section: 0
    property string selectedComputeDevice: "auto"
    function syncComputeSelection() {
        if (computeDevice)computeDevice.currentIndex = computeDevice.indexOfValue(root.selectedComputeDevice)
    }
    onSelectedComputeDeviceChanged: syncComputeSelection()
    onOpened: {
        renderMemory.value = PaintClient.renderSettings.memoryMiB
        renderGpu.value = PaintClient.renderSettings.gpuMiB
        const values = PaintClient.storageSettings
        storageDirectory.text = values.directory || ""
        storageMemory.value = Number(values.memoryMiB)
        selectedComputeDevice = PaintClient.computeDevice
        syncComputeSelection()
        highestQuality.checked = PaintClient.renderSettings.highestQuality
        storageScratch.value = Number(values.scratchGiB || 8)
        storageReserve.value = values.reserveMiB === undefined ? 512 : Number(values.reserveMiB)
        historyLimit.value = PaintClient.historyLimit
        taskProgressDelay.value = PaintClient.longTaskDelay
        PaintClient.inspectStorage()
        PaintClient.inspectCompute()
    }
    contentItem: ColumnLayout {
        spacing: 10
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 12
            // Left sidebar: one entry per preference category.
            Rectangle {
                Layout.preferredWidth: 132; Layout.fillHeight: true
                color: Theme.background; radius: 4; border.color: Theme.line
                ColumnLayout {
                    anchors.fill: parent; anchors.margins: 6; spacing: 4
                    Repeater {
                        model: ["性能", "历史", "界面", "偏好"]
                        Button {
                            required property int index
                            required property string modelData
                            objectName: "preferencesSection:" + modelData
                            Layout.fillWidth: true; implicitHeight: 26; padding: 4
                            text: modelData; font.pixelSize: 9
                            checkable: true; checked: root.section === index
                            palette.windowText: Theme.text
                            background: Rectangle { color: parent.checked || parent.hovered ? Theme.selected : "transparent"; border.color: Theme.line; radius: 2 }
                            contentItem: Text { text: parent.text; color: Theme.text; font.pixelSize: 9; verticalAlignment: Text.AlignVCenter; horizontalAlignment: Text.AlignLeft; leftPadding: 6 }
                            onClicked: root.section = index
                        }
                    }
                    Item { Layout.fillHeight: true }
                }
            }
            // Right pane: only the selected category's controls, grouped and divided by rules.
            Flickable {
                Layout.fillWidth: true; Layout.fillHeight: true
                clip: true; contentWidth: width; contentHeight: pane.implicitHeight
                ScrollBar.vertical: ScrollBar { width: 6 }
                ColumnLayout {
                    id: pane
                    width: parent.width; spacing: 10
                    StackLayout {
                        currentIndex: root.section
                        Layout.fillWidth: true
                        // 1) Performance: the working scratch-disk and cache settings plus stubs.
                        ColumnLayout {
                            spacing: 8
                            Label { text: "GPU硬件加速"; font.pixelSize: 10; color: Theme.accent }
                            ComboBox {
                                id: computeDevice; objectName: "computeDevice"
                                Layout.fillWidth: true
                                model: PaintClient.computeHardware
                                textRole: "name"; valueRole: "id"
                                enabled: !PaintClient.computeBusy && !PaintClient.drawing && !PaintClient.liquifyActive && !PaintClient.transformActive && !PaintClient.layerEditBusy
                                // The hardware model arrives asynchronously. indexOfValue()
                                // alone does not create a binding dependency on its rows.
                                currentIndex: -1
                                onModelChanged: Qt.callLater(root.syncComputeSelection)
                                onCountChanged: Qt.callLater(root.syncComputeSelection)
                                displayText: currentIndex >= 0 ? currentText : (count === 0 ? "正在读取计算硬件…" : "已保存的设备暂不可用")
                                onActivated: { root.selectedComputeDevice = currentValue; if(currentValue === "cpu")highestQuality.checked = false }
                            }
                            Label {
                                objectName: "computeStatus"
                                Layout.fillWidth: true; wrapMode: Text.WordWrap; font.pixelSize: 9; color: Theme.muted
                                text: "当前使用：" + PaintClient.computeStatus + "\n保存后在各文档空闲时生效，无需重启；原图和撤销记录保持。\n目前受支持的功能：液化计算、画布及图层渲染与合成"
                            }
                            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }
                            RowLayout {
                                Layout.fillWidth:true
                                Label {text:"长任务进度延迟";color:Theme.text;font.pixelSize:9;Layout.fillWidth:true}
                                SpinBox {id:taskProgressDelay;objectName:"taskProgressDelay";from:0;to:10000;stepSize:100;editable:true}
                                Label {text:"毫秒";color:Theme.muted;font.pixelSize:9}
                            }
                            Label {text:"计算超过此时间才显示进度；0 为立即显示。";color:Theme.muted;font.pixelSize:8;Layout.fillWidth:true}
                            CheckBox { id: highestQuality; objectName: "highestCanvasQuality"; text: "渲染画布最高质量"; checked: false
                                onToggled: if(checked && root.selectedComputeDevice === "cpu")root.selectedComputeDevice = "auto"
                            }
                            Label { text: "最高质量保留整张画布的原分辨率GPU合成，包括视口外；需要GPU加速，保存后生效。"; wrapMode: Text.WordWrap; Layout.fillWidth: true; color: Theme.muted; font.pixelSize: 9 }
                            Label { text: "存储与缓存（下次启动生效）"; font.pixelSize: 10; color: Theme.accent }
                            GridLayout {
                                columns: 2; columnSpacing: 12; rowSpacing: 8; Layout.fillWidth: true
                                Label { text: "内存使用量（MB）"; color: Theme.text; font.pixelSize: 9 }
                                SpinBox { id: renderMemory; from: 512; to: 1048576; stepSize: 256; editable: true; Layout.fillWidth: true }
                                Label { text: "GPU内存（MB，0=自动）"; color: Theme.text; font.pixelSize: 9 }
                                SpinBox { id: renderGpu; from: 0; to: 1048576; stepSize: 128; editable: true; Layout.fillWidth: true }
                                Label { text: "缓存图块大小（MB）"; color: Theme.text; font.pixelSize: 9 }
                                SpinBox { id: storageMemory; objectName: "storageMemory"; from: 64; to: 1048576; stepSize: 256; editable: true; Layout.fillWidth: true; enabled: !PaintClient.storageBusy }
                                Label { text: "单文档瓦片暂存上限"; color: Theme.text; font.pixelSize: 9 }
                                SpinBox { id: storageScratch; objectName: "storageScratch"; from: 1; to: 64; editable: true; Layout.fillWidth: true; enabled: !PaintClient.storageBusy }
                                Label { text: "保留磁盘空闲空间"; color: Theme.text; font.pixelSize: 9 }
                                SpinBox { id: storageReserve; objectName: "storageReserve"; from: 0; to: 16384; editable: true; stepSize: 256; Layout.fillWidth: true; enabled: !PaintClient.storageBusy }
                                Label { text: "暂存目录"; color: Theme.text; font.pixelSize: 9 }
                                TextField { id: storageDirectory; objectName: "storageDirectory"; placeholderText: "默认：" + (PaintClient.storageInfo.systemDirectory || "系统临时目录"); Layout.fillWidth: true; enabled: !PaintClient.storageBusy; selectByMouse: true }
                            }
                            RowLayout {
                                Layout.alignment: Qt.AlignRight
                                Button { objectName: "useSystemStorageDirectory"; text: "使用系统目录"; enabled: !PaintClient.storageBusy; onClicked: storageDirectory.text = "" }
                                Button { text: "选择目录…"; enabled: !PaintClient.storageBusy; onClicked: folder.open() }
                            }
                            Label {
                                Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.muted; font.pixelSize: 8
                                text: PaintClient.storageBusy ? "正在检查暂存盘…" : PaintClient.storageMessage.length > 0 ? PaintClient.storageMessage
                                    : PaintClient.storageInfo.availableGiB === undefined ? "存储设置保存后，下次启动生效。"
                                    : "目录所在盘可用 " + Number(PaintClient.storageInfo.availableGiB).toFixed(2) + " GiB；本次检查清理 " + Number(PaintClient.storageInfo.removedRuns || 0) + " 个残留目录。"
                            }
                            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }
                        }
                        // 2) History: the working record limit plus stubs.
                        ColumnLayout {
                            spacing: 8
                            Label { text: "记录"; font.pixelSize: 10; color: Theme.accent }
                            RowLayout {
                                Layout.fillWidth: true
                                Label { text: "最大记录"; color: Theme.text; font.pixelSize: 9; Layout.fillWidth: true }
                                SpinBox { id: historyLimit; objectName: "historyLimit"; from: 60; to: 1000; stepSize: 10; editable: true; implicitWidth: 110; enabled: !PaintClient.drawing }
                                Button {
                                    text: "应用"; enabled: !PaintClient.drawing
                                    onClicked: { PaintClient.historyLimit = historyLimit.value; PaintClient.applyHistoryLimit() }
                                }
                            }
                            Label {
                                Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.muted; font.pixelSize: 8
                                text: "已在打开的文档上生效：超出上限的最旧记录会被裁剪，裁剪后的边界不再显示为“初始状态”。"
                            }
                            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }
                        }
                        // 3) Interface.
                        ColumnLayout {
                            spacing: 8
                            Label { text: "界面"; font.pixelSize: 10; color: Theme.accent }
                            CheckBox {
                                objectName: "developerConsoleToggle"
                                text: "开发者控制台"
                                checked: Workspace.developerConsole
                                onToggled: Workspace.developerConsole = checked
                            }
                            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }
                        }
                        ColumnLayout {
                            spacing: 8
                            Label { text: "手绘板输入"; font.pixelSize: 10; color: Theme.accent }
                            ComboBox {
                                objectName: "tabletInputApi"; Layout.fillWidth: true
                                property var modes: ["auto", "windows-ink", "wintab"]
                                model: ["自动（优先 WinTab，未检测到驱动则使用 Windows Ink）", "Windows Ink", "WinTab（适用于驱动关闭 Windows Ink）"]
                                currentIndex: Math.max(0,modes.indexOf(PaintClient.tabletInputMode))
                                onActivated: PaintClient.tabletInputMode = modes[currentIndex]
                            }
                            Label {
                                objectName: "tabletInputStatus"; Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.muted; font.pixelSize: 8
                                text: "当前输入："+PaintClient.tabletInputBackend+"。更改后重启生效。WinTab 需要手绘板驱动提供支持。"
                            }
                            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }
                            Label { text: "笔刷系统"; font.pixelSize: 10; color: Theme.accent }
                            Label { text: "笔刷系统隔离"; font.pixelSize: 9; color: Theme.text }
                            ComboBox {
                                objectName: "brushToolIsolation"
                                Layout.fillWidth: true
                                model: ["不隔离（共用笔刷参数）", "仅大小、不透明度、流量", "隔离（三个工具各自设置）"]
                                currentIndex: PaintClient.brushLibrary.toolIsolation
                                enabled: !PaintClient.drawing && PaintClient.brushLibraryReady
                                onActivated: PaintClient.brushLibrary.toolIsolation = currentIndex
                            }
                            Label {
                                Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.muted; font.pixelSize: 8
                                text: "切换隔离级别时，以当前参数初始化三套设置。三个工具始终分别记住所选笔刷。笔刷名称、分组和资源库保持共用。设置自动保存。"
                            }
                        }
                    }
                    // Pending preferences carry their own "待支持" badge and are never interactive.
                    Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }
                    Repeater {
                        model: ["多屏显示", "内存上限（文档自动回收）", "记录条数按总字节上限（自动压缩）", "主题与强调色", "界面缩放与字体", "快捷键自定义", "数位板映射"]
                        component Pending: RowLayout {
                            id: pendingRow
                            required property string modelData
                            Layout.fillWidth: true; spacing: 8
                            Label { text: pendingRow.modelData; color: Theme.disabled; font.pixelSize: 9; Layout.fillWidth: true; elide: Text.ElideRight }
                            Label { text: "待支持"; color: Theme.disabled; font.pixelSize: 8 }
                        }
                    }
                }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            Label {
                Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.muted; font.pixelSize: 8
                text: "标注“待支持”的项目尚未实现，不能设置。"
            }
            Button {
                objectName: "saveStorageSettings"; text: "保存设置"; enabled: !PaintClient.storageBusy && !PaintClient.computeBusy && !PaintClient.drawing
                onClicked: {
                    if(highestQuality.checked && root.selectedComputeDevice === "cpu")root.selectedComputeDevice = "auto"
                    PaintClient.saveRenderSettings(renderMemory.value, renderGpu.value, highestQuality.checked)
                    PaintClient.historyLimit = historyLimit.value
                    PaintClient.longTaskDelay = taskProgressDelay.value
                    PaintClient.saveStorageSettings(storageDirectory.text, storageMemory.value, storageScratch.value, storageReserve.value)
                    PaintClient.applyHistoryLimit()
                    if (root.selectedComputeDevice !== PaintClient.computeDevice)
                        PaintClient.saveComputeDevice(root.selectedComputeDevice)
                }
            }
            Button { text: "关闭"; onClicked: root.close() }
        }
    }
    FileDialogs.FolderDialog {
        id: folder
        title: "选择暂存目录"
        onAccepted: storageDirectory.text = PaintClient.localFolderPath(selectedFolder)
    }
}
