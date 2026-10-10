import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs as FileDialogs
import "."

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
    onOpened: {
        const values = PaintClient.storageSettings
        storageDirectory.text = values.directory || ""
        storageMemory.currentIndex = Math.max(0, storageMemory.options.indexOf(Number(values.memoryMiB || 256)))
        storageScratch.value = Number(values.scratchGiB || 8)
        storageReserve.value = values.reserveMiB === undefined ? 512 : Number(values.reserveMiB)
        historyLimit.value = PaintClient.historyLimit
        PaintClient.inspectStorage()
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
                        model: ["性能", "历史", "界面"]
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
                            Label { text: "存储与缓存"; font.pixelSize: 10; color: Theme.accent }
                            GridLayout {
                                columns: 2; columnSpacing: 12; rowSpacing: 8; Layout.fillWidth: true
                                Label { text: "像素驻留缓存"; color: Theme.text; font.pixelSize: 9 }
                                ComboBox {
                                    id: storageMemory; objectName: "storageMemory"
                                    property var options: [64, 128, 256, 512, 1024]
                                    model: ["64 MiB", "128 MiB", "256 MiB", "512 MiB", "1024 MiB"]
                                    Layout.fillWidth: true; enabled: !PaintClient.storageBusy
                                }
                                Label { text: "单文档瓦片暂存上限"; color: Theme.text; font.pixelSize: 9 }
                                SpinBox { id: storageScratch; objectName: "storageScratch"; from: 1; to: 64; editable: true; Layout.fillWidth: true; enabled: !PaintClient.storageBusy }
                                Label { text: "保留磁盘空闲空间"; color: Theme.text; font.pixelSize: 9 }
                                SpinBox { id: storageReserve; objectName: "storageReserve"; from: 0; to: 16384; editable: true; stepSize: 256; Layout.fillWidth: true; enabled: !PaintClient.storageBusy }
                                Label { text: "暂存目录"; color: Theme.text; font.pixelSize: 9 }
                                TextField { id: storageDirectory; objectName: "storageDirectory"; placeholderText: "留空使用系统临时目录"; Layout.fillWidth: true; enabled: !PaintClient.storageBusy; selectByMouse: true }
                            }
                            RowLayout {
                                Layout.alignment: Qt.AlignRight
                                Button { text: "使用系统目录"; enabled: !PaintClient.storageBusy; onClicked: storageDirectory.text = "" }
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
                        // 3) Interface: nothing implemented yet, listed honestly.
                        ColumnLayout {
                            spacing: 8
                            Label { text: "界面"; font.pixelSize: 10; color: Theme.accent }
                            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }
                        }
                    }
                    // Pending preferences carry their own "待支持" badge and are never interactive.
                    Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }
                    Repeater {
                        model: ["GPU 与多屏合成", "内存上限（文档自动回收）", "记录条数按总字节上限（自动压缩）", "主题与强调色", "界面缩放与字体", "快捷键自定义", "数位板映射"]
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
                objectName: "saveStorageSettings"; text: "保存设置"; enabled: !PaintClient.storageBusy && !PaintClient.drawing
                onClicked: {
                    PaintClient.historyLimit = historyLimit.value
                    PaintClient.saveStorageSettings(storageDirectory.text, storageMemory.options[storageMemory.currentIndex], storageScratch.value, storageReserve.value)
                    PaintClient.applyHistoryLimit()
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
