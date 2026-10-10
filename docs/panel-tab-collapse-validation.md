# 面板标签双击收回验收（2026-10-10）

设计见 [panel-tab-collapse-design.md](panel-tab-collapse-design.md)。本轮修复用户第 6 条残留缺口，并补齐第 11、25、27 条此前实现但缺少断言的共享行为；Qt/QML、CanvasItem 的空格平移状态查询与 Qt Test 有改动，Rust、C ABI 1.10、布局 v6 结构、设置格式与历史语义不变。

## 修复

`ui/qml/PanelGroup.qml` 的标签 `panelTabGrip` 双击原先写 `setColumnCollapsed(id,!root.flyout)`：在侧边展开窗（flyout）里等于“展开来源图标列”。现改为 `flyout ? dismissRequested() : setColumnCollapsed(id,true)`，与页头拖动栏双击一致——只收回当前呈现，绝不展开来源列。停靠/浮动列双击标签仍折叠为图标。正文继续共用 `PanelContent`，页头/标签/正文容器继续共用 `PanelGroup`，未新增第二套渲染路径（全仓只有 DockTile 一处 `PanelGroup`、PanelGroup 一处 `PanelContent` 实例化）。

## CanvasItem 改动

`spaceHeld` 是应用级“按住空格”状态，原先只有测试不可见的可写变量。为让“文本输入时按空格不误平移”可被真实断言：

- `ui/src/CanvasItem.h` 增加只读 `spacePanning`（`m_space || spaceHeld`）与 `spacePanningChanged`，取值实现保持在 .cpp，应用级 `spaceHeld` 仍为文件内部符号；
- 所有写入统一经 `setSpacePanning(bool)`（全局态）与 `setKeySpace(bool)`（聚焦画布的按键态），两者都按合并后的 `spacePanning` 值判断变化并发射信号，满足 Q_PROPERTY 通知契约；`WindowDeactivate` 现在也清空空格平移状态（此前会残留）；
- 按键、失活、释放、自动重复抑制的既有语义不变。

## 新增回归（ui/tests/ui_tests.cpp）

- `panelTabsRetractWithoutExpandingRailsAndKeepSharedPresentation`
  - 默认右侧同类别列折叠为图标列；点击图标打开类别展开窗；展开窗顶层标签栏 alpha≈0.75（第 11 条顶部浮动栏 75% 不透明），停靠栏 alpha≈1。
  - 同时打开颜色类别展开窗，双击导航展开窗页头只收回该窗口：整列仍为图标态、图标列可见、另一个展开窗不受影响（设计中的“其它已打开展开窗不受影响”）。
  - 再次打开后双击面板标签 `panelTab:layers`：窗口消失、整列仍为图标态——本次修复的行为。
  - 关闭另一个展开窗后颜色列仍为折叠状态；停靠列恢复展开后双击标签仍折叠整列，保持既有语义。
  - 红/绿证据：把该行临时还原为 `!root.flyout` 后重编译，本用例在“双击标签后整列仍为图标态”断言处失败（`ui_tests.cpp:2226`，`icons` 为 false：来源列被重新展开）；恢复修复后通过。日志 artifacts/panel-tab-collapse-red.txt。
- `textFieldsKeepSpaceAndHistoryReplayStaysStepped`
  - 第 25 条：真实聚焦的 `TextField` 按空格只输入空格（“78” → “78 ” → 退格回 “78”），同一时刻画布 `spacePanning` 为 false、撤销深度不变；用后删除该临时控件。
  - 第 27 条：以 4 笔真实历史点击深度 0 条目，定时器 `interval == 20`，点击后按 5ms 采样深度序列必须依次经过 4→…→0（`seen.size()>=3`，首尾为 4 与 0），即逐条回放而非瞬间跳转；随后定时器自行停止。该断言在“瞬间跳转”实现下会因看到 4 与 0 两段而失败。

## 独立复核

verifier 队友核对了完整 diff、单渲染路径、生成头/构建输出未污染、两级 DPI 重复运行无抖动，并指出三处需要加强：`spacePanning` 通知不完整、设计文档未写明 CanvasItem 改动、逐条回放断言无法区分瞬间跳转。三处均已按上面内容修正后重新验证。

## 验证结果

- 改动文件：`ui/qml/PanelGroup.qml`、`ui/qml/PanelContent.qml`（仅给历史回放 Timer 增加 objectName）、`ui/src/CanvasItem.h/.cpp`、`ui/tests/ui_tests.cpp`、`ui/CMakeLists.txt`（两倍 DPI 列表加入两个新用例）、`docs/*`。
- Qt Test：普通 DPI 68 项、两倍 DPI 39 项全部通过（0 失败、0 跳过，无 QML 警告）。日志 build/verify-ui/qt-ui-tests.txt、qt-ui-highdpi.txt。两个新用例再加 `-repeat 6` 共 24 项通过，无抖动。
- 完整 `python tools/check.py` 通过：Python 工具与真实 Git 检出、Rust fmt/clippy/tests/doc、release、cbindgen 严格字节检查、C11/C++20 static/shared 共 6 项 CTest 全通过。日志 artifacts/panel-tab-collapse-final-check.log。
- 部署：`cmake --build build/qt --config Release --target deploy_qt` 更新 build/qt/bin/Release/drawverse.exe，日志 artifacts/panel-tab-collapse-deploy.log。

环境：Windows 11 / Qt 6.8.3 / MSVC 2022。Qt 6.5–6.7 未在本机运行验证，macOS/Linux 原生行为仍需各自实机验证。
