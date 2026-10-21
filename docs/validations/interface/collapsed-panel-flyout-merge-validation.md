# 折叠列弹出面板合并验收

日期：2026-10-21。Windows 11、Qt 6.8.3，Release；[设计](../../interface/collapsed-panel-flyout-merge-design.md)。

## 结果

- `collapsedFlyoutMergesMimeDropsAndKeepsTabOrder`：Qt MIME 单面板拖入内容区追加；新旧侧栏图标复用同一窗口，标签换序同步图标，保存/恢复保持同组、顺序与折叠状态。
- `collapsedFlyoutMergesNativeWindowDrag` 的 single-panel、one-group、whole-column 三行：真实窗口拖动命中浮面板标签插入点，单面板/整组/多组整列按来源顺序合并；Ctrl 禁用吸附，Esc 恢复来源，最终保持折叠。单面板拖入不携带同组其他面板。
- 上述四行，加欢迎页准备/文档生命周期两行、资源列表快速滚轮一行，在软件与 D3D11 分别通过：各 7 个功能用例，含初始化清理各 9 passed。欢迎生命周期断言实际硬件后端；已检查 D3D11 整列合并截图，标签顺序为画笔、画笔设置、颜色。
- Release 应用及工作流测试构建通过；之前本轮完整 Qt/四 ABI 构建通过。无 Rust、ABI、state、VBR 或工作区格式变更。产物：`build/qt/bin/Release/drawverse.exe`。

## 记录与边界

日志为本地 artifacts/flyout-merge-software.txt、flyout-merge-d3d11.txt；截图为相同前缀的单面板/整组/整列 PNG。移除了排查期间的临时日志。

独立复核 `panelTabsRetractWithoutExpandingRailsAndKeepSharedPresentation` 仍失败：交互断言完成，但最终要求零 QML 警告，实际收到 LayerLock 的 Connections 信号警告与 LayersPanel 的 selected.mask 未定义警告（7 条）。日志为 artifacts/flyout-shared-panel-software.txt。未做同提交基线对照，不能判定为既有问题；欢迎页验收中另三项扩展 UI 失败仍未解决。

欢迎生命周期退出仍报告两个异步创建对象；D3D11 原生拖动测试报告交换链 alphaBufferSize 提示。没有屏蔽这些日志。本轮不声称全量 UI、跨平台或实体持续拖动验收通过。
