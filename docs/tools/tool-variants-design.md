# 工具形态切换（2026-10-10）

用户要求像 Photoshop 一样把同一工具的不同形态合并在一个槽位：图标右下角画小三角提示"可切换"，右键图标弹出悬浮小面板列出该工具的全部形态，**点击面板里的形态才切换**，点工具栏图标只激活当前形态（不再循环切换）。

## 机制

- `ui/qml/IconButton.qml` 新增 `hasVariants`，为真时在右下角向外画一个 6×6 三角（`Shape` + `variantMark:<objectName>`，不覆盖图标本体）；控件本身仍是原按钮，停靠列、浮动工具条、图标列共用同一个组件。
- `ui/qml/ToolVariantsPanel.qml`：`Popup`，标题 + 形态列表，每项显示图标、名称、状态（当前形态显示 ✓）。条目数据用 `{id,label,icon,pending,selected,apply}` 描述；`pending:true` 的形态灰显、`enabled:false`、右侧标注"待支持"，点击无效果。面板按索引读取条目（`at(index)`），不依赖 JS 数组与 `QVariantList` 的隐式转换。
- `ui/qml/ToolStrip.qml` 的选区槽位：左键只做"激活当前形态"（`selectionTool<1 → 1`，已激活则不变），不再在矩形/椭圆之间循环；右键（`TapHandler acceptedButtons: Qt.RightButton`）打开面板，点选矩形/椭圆才切换（`PaintClient.selectionTool=1/2`）。`M`、`Shift+M` 及其它既有快捷键不变。

## 形态清单

| 形态 | 状态 |
|---|---|
| 矩形选框 | 可用（内核 `SelectionKind::Rectangle`） |
| 椭圆选框 | 可用（内核 `SelectionKind::Ellipse`） |
| 套索 | **待支持**：`paint-core` 目前只有矩形/椭圆两种 `SelectionKind`（`selection.rs`），自由套索需要新的多边形选择类型、FFI DTO/ABI 变更与 Qt 路径采集，本轮未实现，因此如实灰显标注，不做点了没反应的假项 |

新增 `lassoSelection` 图标路径（虚线套索轮廓）用于占位显示。

## 验收

`selectionToolVariantsUseCornerMarkAndRightClickPanel`（offscreen + 软件渲染）：

- 选区按钮存在 `variantMark:selectionTool` 且可见（右下角三角就是"可切换"的提示）；
- 面板初始不可见；左键点击按钮后 `selectionTool` 仍为 1（只激活、不切换形态）；
- 右键点击后 `toolVariantPanel` 打开，`variantEntry:rectangle / ellipse / lasso` 三项都在，矩形/椭圆 `enabled`、套索 `enabled==false` 且 `toolVariantState:lasso` 文本为"待支持"；
- 点击 `variantTrigger:ellipse` 后 `selectionTool==2`、面板自动关闭；再次右键打开时 `toolVariantState:ellipse` 显示 ✓。

配套回归：`compactStatusReflectsDocumentAndZoomControlsPreserveCenter`（状态栏工具名）、`selectionsDragCombineUndoConstrainAndPersist`（选区组合/撤销）、`colorClicksAndCapsLockControlBrushOutline`、`qmlMouseTabletAndFloatingWindows`（浮动工具条上点选区图标仍激活矩形）全部通过；新用例同时加入两倍 DPI 列表。

## 已知未完成

自由套索本体未实现（见上表）。它需要：`paint-core` 增加多边形选择类型与扫描线判定、`selection_api` 新增路径输入 DTO（ABI 1.12）、`CanvasItem` 采集拖动路径并显示实时轮廓、历史动作类型与 ORA 序列化同步——按新模块先写设计再实施。
