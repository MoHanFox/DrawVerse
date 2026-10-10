import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

// Read-only list of the shortcuts the application really binds today. Editing them is not
// implemented, so the footer says so instead of offering controls that would do nothing.
Dialog {
    id: root
    objectName: "keyboardShortcutsDialog"
    title: "键盘快捷键"
    modal: true
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: 520
    implicitHeight: 460
    // Only the key column carries an objectName; the labels are plain text next to it.
    readonly property var rows: [
        ["文档", "新建画布", "Ctrl+N"],
        ["文档", "打开…", "Ctrl+O"],
        ["文档", "保存", "Ctrl+S"],
        ["文档", "另存为 OpenRaster…", "Ctrl+Shift+S"],
        ["编辑", "撤销", "Ctrl+Z"],
        ["编辑", "重做", "Ctrl+Shift+Z"],
        ["编辑", "首选项…", "Ctrl+K"],
        ["编辑", "创建／释放剪贴蒙版", "Ctrl+Alt+G"],
        ["选择", "矩形选框", "M"],
        ["选择", "椭圆选框", "Shift+M"],
        ["选择", "全选", "Ctrl+A"],
        ["选择", "取消选择", "Ctrl+D"],
        ["选择", "反向选择", "Ctrl+Shift+I"],
        ["工具", "移动工具", "V"],
        ["工具", "画笔", "B"],
        ["工具", "橡皮擦", "E"],
        ["视图", "适合窗口", "F"],
        ["视图", "取消当前笔触", "Esc"],
        ["画布", "平移视图", "按住空格拖动"],
        ["画布", "临时切换缩小／放大", "Alt＋滚轮"],
        ["画布", "调整笔刷大小", "Ctrl＋拖动"],
        ["数位板", "画笔轮廓／精确十字", "CapsLock"]
    ]
    contentItem: ColumnLayout {
        spacing: 10
        Label {
            Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.muted; font.pixelSize: 9
            text: "以下为当前实际生效的快捷键；与菜单里显示的快捷键一致。自定义快捷键尚未实现。"
        }
        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }
        ListView {
            id: list
            objectName: "shortcutList"
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true; spacing: 5
            model: root.rows
            ScrollBar.vertical: ScrollBar { width: 6 }
            delegate: Item {
                id: entry
                required property var modelData
                width: list.width; height: 22
                Text { anchors.left: parent.left; anchors.verticalCenter: parent.verticalCenter; width: 36; text: entry.modelData[0]; color: Theme.muted; font.pixelSize: 8; elide: Text.ElideRight }
                Text { anchors.left: parent.left; anchors.leftMargin: 44; anchors.right: keyBox.left; anchors.rightMargin: 10; anchors.verticalCenter: parent.verticalCenter; text: entry.modelData[1]; color: Theme.text; font.pixelSize: 9; elide: Text.ElideRight }
                Rectangle {
                    id: keyBox
                    anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter
                    width: Math.max(46, keyText.implicitWidth + 12); height: 18
                    color: Theme.input; border.color: Theme.line; radius: 2
                    Text { id: keyText; objectName: "shortcutKey"; anchors.centerIn: parent; text: entry.modelData[2]; color: Theme.accent; font.pixelSize: 9 }
                }
            }
        }
        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }
        RowLayout {
            Layout.fillWidth: true
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.disabled; font.pixelSize: 8; text: "自定义快捷键（待支持）" }
            Button { objectName: "closeKeyboardShortcuts"; text: "关闭"; onClicked: root.close() }
        }
    }
}
