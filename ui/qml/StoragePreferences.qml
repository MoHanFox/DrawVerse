import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs as FileDialogs
import "."

Dialog {
    id: dialog
    title: "性能与暂存盘"
    modal: true
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: 620
    onOpened: {
        const values = PaintClient.storageSettings
        directory.text = values.directory || ""
        memory.currentIndex = Math.max(0, memory.options.indexOf(Number(values.memoryMiB || 256)))
        scratch.value = Number(values.scratchGiB || 8)
        reserve.value = values.reserveMiB === undefined ? 512 : Number(values.reserveMiB)
        PaintClient.inspectStorage()
    }
    FileDialogs.FolderDialog {
        id: folder
        title: "选择暂存目录"
        onAccepted: directory.text = PaintClient.localFolderPath(selectedFolder)
    }
    contentItem: ColumnLayout {
        spacing: 16
        Label { text: "画布存储"; font.pixelSize: 18; color: Theme.accent }
        Label {
            text: "冷瓦片与大型撤销记录保存在暂存盘。设置保存后，下次启动生效。"
            wrapMode: Text.WordWrap; Layout.fillWidth: true; color: Theme.muted
        }
        Label { text: "下次启动配置"; color: Theme.text }
        GridLayout {
            columns: 2; columnSpacing: 16; rowSpacing: 12; Layout.fillWidth: true
            Label { text: "像素驻留缓存" }
            ComboBox {
                id: memory; objectName: "storageMemory"
                palette.dark: Theme.muted
                property var options: [64, 128, 256, 512, 1024]
                model: ["64 MiB", "128 MiB", "256 MiB", "512 MiB", "1024 MiB"]
                Layout.fillWidth: true; enabled: !PaintClient.storageBusy
            }
            Label { text: "单文档瓦片暂存上限" }
            SpinBox { id: scratch; objectName: "storageScratch"; from: 1; to: 64; editable: true; Layout.fillWidth: true; enabled: !PaintClient.storageBusy }
            Label { text: "保留磁盘空闲空间" }
            SpinBox { id: reserve; objectName: "storageReserve"; from: 0; to: 16384; editable: true; stepSize: 256; Layout.fillWidth: true; enabled: !PaintClient.storageBusy }
            Label { text: "暂存目录" }
            TextField { id: directory; objectName: "storageDirectory"; placeholderText: "留空使用系统临时目录"; placeholderTextColor: Theme.muted; Layout.fillWidth: true; enabled: !PaintClient.storageBusy; selectByMouse: true }
        }
        RowLayout {
            Layout.alignment: Qt.AlignRight
            Button { text: "使用系统目录"; enabled: !PaintClient.storageBusy; onClicked: directory.text = "" }
            Button { text: "选择目录…"; enabled: !PaintClient.storageBusy; onClicked: folder.open() }
        }
        Label { text: "暂存上限单位 GiB；空闲空间单位 MiB。缓存仅统计瓦片像素，撤销与显示缓存另计。"; Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.muted }
        Label {
            Layout.fillWidth: true; wrapMode: Text.WrapAnywhere; color: Theme.muted
            text: PaintClient.activeStorageSettings.memoryMiB === undefined ? "本次运行：暂存盘未就绪，请修正设置并重启。" : "本次运行：" + (PaintClient.activeStorageSettings.directory || PaintClient.storageInfo.systemDirectory || "系统临时目录")
                + "\n像素缓存：" + (PaintClient.activeStorageSettings.memoryMiB || 256) + " MiB"
                + " · 瓦片暂存：" + (PaintClient.activeStorageSettings.scratchGiB || 8) + " GiB"
                + " · 保留空闲：" + (PaintClient.activeStorageSettings.reserveMiB === undefined ? 512 : PaintClient.activeStorageSettings.reserveMiB) + " MiB"
        }
        Label {
            Layout.fillWidth: true; wrapMode: Text.WordWrap
            text: PaintClient.storageBusy ? "正在检查暂存盘…" : PaintClient.storageMessage.length > 0 ? PaintClient.storageMessage
                : PaintClient.storageInfo.availableGiB === undefined ? "" : "目录所在盘可用 " + Number(PaintClient.storageInfo.availableGiB).toFixed(2)
                + " GiB；本次检查清理 " + Number(PaintClient.storageInfo.removedRuns || 0) + " 个残留目录。"
        }
        RowLayout {
            Layout.alignment: Qt.AlignRight
            Button { text: "关闭"; onClicked: dialog.close() }
            Button {
                text: "保存设置"; objectName: "saveStorageSettings"; enabled: !PaintClient.storageBusy
                onClicked: PaintClient.saveStorageSettings(directory.text, memory.options[memory.currentIndex], scratch.value, reserve.value)
            }
        }
    }
}
