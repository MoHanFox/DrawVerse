# 搜索与快捷键最小验证

2026-10-12，Release 构建成功，生成 ABI 头一致性检查通过；无核心或 ABI 行为修改。

Windows 原生 Qt、100% 缩放、software 后端，仅执行以下两个用例：

- `layerContextClipboardFocusShortcutsAndLowerEdgeDrop`：名称大小写与首尾空白过滤、无结果提示、折叠组祖先和组名匹配子树、清空恢复折叠状态；画笔名称过滤及搜索结束；搜索框 Delete/字母留给文本，Esc/Enter/点击工具恢复画布焦点；画布焦点下 Ctrl+C/V，面板 Ctrl+J/E/G、Delete；移除拷贝粘贴菜单与液化工具栏入口，保留复制操作及拖放回归。
- `liquifyCanvasTransactionOptionsAndDocumentLock`：滤镜进入、回车确认单条历史、Esc 取消、浮动画布详情栏及原有参数、鼠标/数位板和文档锁定回归。

两个用例通过，无 QML 警告。日志：`artifacts/search-tools-native-1.txt`。未运行全量 `tools/check.py` 或核心测试。

重现：配置 Qt 插件路径后，设置 `QT_QPA_PLATFORM=windows`、`QT_QUICK_BACKEND=software`、`QT_SCALE_FACTOR=1`，运行 `qt_ui_tests.exe layerContextClipboardFocusShortcutsAndLowerEdgeDrop liquifyCanvasTransactionOptionsAndDocumentLock`。
