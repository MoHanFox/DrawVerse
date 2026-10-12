# 搜索与快捷键最小验证

## 2026-10-13：面板保持打开时恢复搜索输入

问题在图标展开的独立 `PanelFlyout` 原生窗口复现：`WindowDoesNotAcceptFocus` 使 `requestActivate()` 被 Qt 拒绝，键盘仍送到主窗口；控件局部焦点不能替代原生窗口激活。修复只移除可编辑面板的此标志，保留原有无边框窗口和停留行为。

仅新增并执行 Qt 用例 `layerSearchRefocusAfterToolsAndCanvasBlank`，Windows 原生 windows 平台、software、缩放 1：停靠面板及图标展开面板分别连续 3 轮“搜索→工具→搜索→画布外部空白→搜索”，真实鼠标点击与键盘输入；展开面板的按键发送到操作系统当前焦点窗口，检查原生焦点归属、查询保留、B 留给文本、原工具不变、面板持续可见和搜索控件实例不重建。

修复前该用例复现原生焦点归属错误和 Qt 拒绝激活的警告；修复后 1 项功能测试通过（含 init/cleanup 共 3 passed），0 failed、0 warnings。Release 构建和 `git diff --check` 通过；本轮无 Rust/ABI 改动，没有执行无关测试或全量检查。报告：`artifacts/search-refocus-native-1.txt`。

2026-10-12，Release 构建成功，生成 ABI 头一致性检查通过；无核心或 ABI 行为修改。

Windows 原生 Qt、100% 缩放、software 后端，仅执行以下两个用例：

- `layerContextClipboardFocusShortcutsAndLowerEdgeDrop`：名称大小写与首尾空白过滤、无结果提示、折叠组祖先和组名匹配子树、清空恢复折叠状态；画笔名称过滤及搜索结束；搜索框 Delete/字母留给文本，Esc/Enter/点击工具恢复画布焦点；画布焦点下 Ctrl+C/V，面板 Ctrl+J/E/G、Delete；移除拷贝粘贴菜单与液化工具栏入口，保留复制操作及拖放回归。
- `liquifyCanvasTransactionOptionsAndDocumentLock`：滤镜进入、回车确认单条历史、Esc 取消、浮动画布详情栏及原有参数、鼠标/数位板和文档锁定回归。

两个用例通过，无 QML 警告。日志：`artifacts/search-tools-native-1.txt`。未运行全量 `tools/check.py` 或核心测试。

重现：配置 Qt 插件路径后，设置 `QT_QPA_PLATFORM=windows`、`QT_QUICK_BACKEND=software`、`QT_SCALE_FACTOR=1`，运行 `qt_ui_tests.exe layerContextClipboardFocusShortcutsAndLowerEdgeDrop liquifyCanvasTransactionOptionsAndDocumentLock`。
