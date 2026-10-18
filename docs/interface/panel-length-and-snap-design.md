# 展开窗长度、外轮廓层序与最近颜色（2026-10-10）

本轮处理用户三条反馈：展开窗自动补长、外轮廓渲染优先级过高、色轮下方色板应为"最近使用的颜色"。均为 Qt/QML 与 UI 偏好层改动，Rust 与 C ABI 1.10 不变。

## 1. 展开窗不再自动补长

根因两处叠加：

- `ui/qml/BrushSettingsPanel.qml` 在参数与预览之间有一个 `Layout.fillHeight` 撑高项，把预览推到窗口底部，中间留出空白；
- `ui/qml/PanelFlyout.qml` 把展开窗高度写死 `440`（画笔 `540`）。

改法：参数区改为内容定高（内容块高度 = `content.implicitHeight`），预览紧跟"圆形笔尖"说明，撑高项移到内容块之后只吸收窗口多余高度；展开窗按面板类型给出所需高度（画笔设置 260、画笔 420、导航 300、历史 380、其余 360），并限制在可用屏幕高度内。停靠列正文保持顶部对齐，多余高度留在底部。窗口仍可拖动调整，用户改过的宽高继续由布局 v6 的 `panelViews` 记忆。

## 2. 外轮廓不再压住面板/菜单

`ui/qml/WindowOutline.qml` 用 `Qt.ToolTip` 标志创建主窗外轮廓窗。`Qt.ToolTip` 在 Windows 上是弹出式窗口，会持续被抬到普通窗口之上；它的透明内部覆盖整个主窗区域，于是面板展开窗、菜单等窗口在该区域内被"优先渲染"的外轮廓窗遮住。

改法：改用 `Qt.Tool|Qt.FramelessWindowHint|Qt.WindowTransparentForInput|Qt.WindowDoesNotAcceptFocus`——仍是不可交互、不抢焦点、不占正文的边框窗，但不再是弹出式层序。外描边颜色 `#63666B`、50% 不透明、向外 1px、最大化隐藏等既有行为不变（`WindowOutline.qml` 只改 flags）。

## 3. 色板改为"最近使用的颜色"

色轮下方的色板原先是硬编码的 12 个固定颜色。现改为最近使用颜色列表，由真实用色驱动、按最近使用排序、最多 12 个、重复只前移不重复出现：

- `ui/src/BrushLibrary.h/.cpp` 增加 `recentColors` 属性、`useColor()`（校验颜色、去重前移、截断 12）与 `recentColorsChanged`；`snapshot()` 写入可选 `recentColors` 数组，`restore()` 缺少该字段时保留内置初始色，非法颜色或超过 12 个判为损坏并拒绝；
- `ui/src/PaintCoreClient.h/.cpp` 暴露 `recentColors` 并把 `recentColorsChanged` 转发给 QML；设置前景色（色轮取色、点击色板、交换前景/第二色）都会调用 `useColor()`，因此点击色板本身也会把这个颜色提到最前；
- `ui/qml/PanelContent.qml` 的颜色面板用 `PaintClient.recentColors` 渲染色板（`objectName:"recentColorGrid"`），空列表时不显示任何色块。

新配置首次使用显示内置初始色，之后由用户实际用色累积；偏好沿用既有画笔偏好 JSON 与后台保存路径（`brushes/state`），不新增设置文件。

## 验收

- 定点 Qt 用例（offscreen + 软件渲染）：`colorClicksAndCapsLockControlBrushOutline`、`brushPresetsKeepIndependentSettingsAndRejectStalePreviews`、`brushLibrarySettingsAndEnginePreviewsShareSelectionAcrossWindows`、`panelPresentationRemembersFlyoutsAndPaintsActualHeaderAlpha`、`categoryFlyoutsShareRowsTabsAndRetractWithoutExpanding`、`compactLayerSliderAndHsvMarkersUseSharedStyles` 全部通过（8 项含初始化/清理）；外轮廓几何与描边断言 `collapsedExpandButtonDragsWithoutExpandingAndPixelsStayInPlace` 覆盖 `WindowOutline` 的 1px 外扩几何与 `#63666b`/50% 描边。
- 未运行完整 `python tools/check.py`（按当前约定只跑与改动直接相关的最小用例）。
- `cmake --build build/qt --config Release --target deploy_qt` 通过，`build/qt/bin/Release/drawverse.exe` 已更新并启动冒烟。

## 已知未完成

"收起一个并列面板后其余面板不被拉长"仍未实现：根因在 `WorkspaceManager::layoutItems` 的纵向分配按分割比例铺满可用长度。需要为纵向普通面板链改成"每个面板占用并保存自己的长度、剩余留白"，会影响分割线拖动、工具列整列高度与浮窗正文像素断言，按新模块先设计再实施。
