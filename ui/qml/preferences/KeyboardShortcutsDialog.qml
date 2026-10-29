import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import DrawVerse 1.0
import "../theme"

Dialog {
    id: root
    objectName: "keyboardShortcutsDialog"
    title: "键盘快捷键"
    modal: true
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(640, parent.width - 24)
    height: Math.min(520, parent.height - 24)
    property string editingId: ""
    property string message: ""
    readonly property var rows: ShortcutSettings.commands
    function stopRecording() { editingId = ""; ShortcutSettings.recording = false }
    function assignKey(id, key) {
        const error = ShortcutSettings.validate(id, key)
        if (error) { message = error; return }
        stopRecording()
        message = ShortcutSettings.assign(id, key)
    }
    onClosed: stopRecording()
    contentItem: ColumnLayout {
        spacing: 8
        Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.muted; font.pixelSize: 11; text: "点击快捷键后按下新组合，即时保存；Esc 取消录入。空格平移、Alt 吸色、Ctrl 拖动、缩放、确认/取消及方向键保持固定。" }
        RowLayout {
            ComboBox { id: category; model: ["全部", "文档", "编辑", "调整", "图层", "选择", "工具", "视图", "画布"]; onActivated: root.stopRecording() }
            TextField { id: search; Layout.fillWidth: true; placeholderText: "搜索命令"; onTextEdited: root.stopRecording() }
        }
        ListView {
            id: list
            objectName: "shortcutList"
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 4
            model: root.rows.filter(row => (category.currentIndex === 0 || row.category === category.currentText) && (row.label.indexOf(search.text) >= 0 || row.key.toLowerCase().indexOf(search.text.toLowerCase()) >= 0))
            ScrollBar.vertical: ScrollBar {}
            delegate: RowLayout {
                id: entry
                required property var modelData
                width: list.width
                height: 30
                Label { text: entry.modelData.category; color: Theme.muted; font.pixelSize: 10; Layout.preferredWidth: 36 }
                Label { text: entry.modelData.label; color: Theme.text; font.pixelSize: 11; Layout.fillWidth: true; elide: Text.ElideRight }
                Button {
                    id: capture
                    objectName: "shortcutKey:" + entry.modelData.id
                    Layout.preferredWidth: 160
                    text: root.editingId === entry.modelData.id ? "请按下快捷键…" : (entry.modelData.key || "未设置")
                    onClicked: { root.editingId = entry.modelData.id; root.message = ""; ShortcutSettings.recording = true; forceActiveFocus() }
                    Keys.priority: Keys.BeforeItem
                    Keys.onShortcutOverride: event => { if (root.editingId === entry.modelData.id) event.accepted = true }
                    Keys.onPressed: event => {
                        if (root.editingId !== entry.modelData.id) return
                        event.accepted = true
                        if (event.isAutoRepeat) return
                        if (event.key === Qt.Key_Escape) { root.stopRecording(); return }
                        const key = ShortcutSettings.fromKey(event.key, event.modifiers)
                        if (!key) return
                        root.assignKey(entry.modelData.id, key)
                    }
                }
                Button { text: "清除"; onClicked: { root.assignKey(entry.modelData.id, "") } }
            }
        }
        Label { Layout.fillWidth: true; visible: text.length > 0; wrapMode: Text.WordWrap; color: "#ee9966"; text: root.message; font.pixelSize: 11 }
        RowLayout {
            Layout.fillWidth: true
            Button { objectName: "resetKeyboardShortcuts"; text: "恢复全部默认"; onClicked: { root.stopRecording(); root.message = ShortcutSettings.reset() } }
            Item { Layout.fillWidth: true }
            Button { objectName: "closeKeyboardShortcuts"; text: "关闭"; onClicked: root.close() }
        }
    }
}
