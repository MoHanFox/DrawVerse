# 套索与魔棒工具（2026-10-10）

用户要求补齐 Photoshop 式的套索与魔棒：套索是自由路径选区，魔棒是按颜色相似度扩散的选区。两者都进入现有的"选区"模型，因此绘画/擦除/蒙版约束、历史、撤销重做、ORA 保存自动生效。

## 核心问题与方案

现有选区是**几何函数**：`SelectionShape` 只有矩形/椭圆，`coverage(x,y)` 逐像素解析求值、不分配内存、可无损文本序列化。套索与魔棒不是这两者：套索是多边形、魔棒要读文档像素做连通扩散。

方案：给 `SelectionShape` 增加**可选像素掩码** `mask: Option<SelectionMask>`（`SelectionMask { width, height, coverage: Arc<[u8]> }`），并把 `SelectionKind` 扩展为 `Rectangle | Ellipse | Polygon | Mask`：

- 矩形/椭圆保持原来的解析 `coverage`，`mask=None`，行为与性能不变；
- **Polygon**（套索）：在创建时用奇偶规则把闭合路径扫描线栅格化成掩码，之后与矩形/椭圆走同一套合并逻辑；
- **Mask**（魔棒）：在创建时对文档合成像素做洪水填充（容差比较），把连通区域固化成掩码。

这样 `coverage()` 仍是纯函数（掩码查表），`Arc` 让步骤克隆零拷贝，`Selection` 的合并/反转/步骤预算等既有语义不变。代价是内存：掩码限制 `宽*高 <= 16Mi` 且 `<= 文档像素数`，超限返回资源错误（UI 明确提示，不静默失败）。

## 序列化

文本编码升级为 v2：`2|<enabled>;<op>,<kind>,<aa>,<x>,<y>,<w>,<h>[,<mask>]`，其中掩码用行内 RLE：`<行>,<起始>,<长度>,…` 以 `|` 结尾，覆盖值只有 0/1（掩码按像素中心判断，天然二值）。旧 v1 文本继续可读（ORA 旧文件）；新写入 v2，ORA 版本 v4 → v5。v1 前缀 `1|` 与 v2 前缀 `2|` 互斥，`from_text` 两者都接受。

## C ABI 1.12

只新增符号，不改任何既有 DTO 布局：

- `paint_session_edit_selection_path(core, session, request: *const PaintSelectionPath, out_sequence)`：`PaintSelectionPath { struct_size, edit_kind, operation, antialias, point_count, tolerance, reserved, points: *const PaintSelectionPoint }`，`edit_kind = POLYGON | MAGIC`；POLYGON 要求 `3..=4096` 个点、MAGIC 恰好 1 个种子点（容差 `0..=255`，取 0-255 的通道距离阈值）。点坐标必须是有限值且在文档外扩 4096 范围内。
- `paint_core_selection_mask(core, session, publication, x, y, width, height, out_bytes, out_len)`：按需读取当前选区的掩码窗口（供 Qt 画套索/魔棒轮廓，避免把整张掩码拉到 QML）。越界或 `publication` 不匹配返回 BUSY/INVALID，不返回部分数据。
- `paint_session_selection_step` 的 `shape` 字段扩展取值：`2 = Polygon`、`3 = Mask`；几何仍返回掩码包围盒，旧调用方读到 0/1 行为不变。

## Qt / QML

- 工具形态：选区槽位现在有四个形态——矩形选框、椭圆选框、套索、魔棒；全部可用，`ToolVariantsPanel` 里不再有"待支持"项。
- `PaintCoreClient::selectionTool`：`0` 无、`1` 矩形、`2` 椭圆、`3` 套索、`4` 魔棒（原有 1/2 语义不变，只需扩展范围）。`M`/`Shift+M` 保持矩形/椭圆，新增 `L`（套索）与 `W`（魔棒）。套索/魔棒也进状态栏工具名与画笔详情栏。
- 套索交互：画布上按下开始采集点（按屏幕距离抽稀，最多 4096 点），移动画实时预览轮廓，松开即提交路径（闭合），随后用现有选区轮廓机制绘制；右键/Esc 取消不提交。
- 魔棒交互：画布上单击以落点为种子提交扩散；`Shift` 加选、`Alt` 减选，与现有矩形/椭圆的修饰键语义一致。
- 实时轮廓：`CanvasItem` 在套索采集中绘制当前路径（现有 `SelectionOverlay` 之外的新绘制项），提交后走既有选区步骤获取。

## 历史与保存

- 套索使用 `HistoryAction::Selection`，魔棒使用新的 `HistoryAction::MagicSelection`（名称"魔棒"、图标复用选区图标），两者都走已有 `set_selection_with_action`，因此撤销/重做、逐条历史、裁剪边界语义不变。
- ORA 写入 v5（选区文本 v2），读取兼容 v1–v4。

## 选区轮廓（蚂蚁线）

`ui/src/SelectionOverlay.cpp` 把静态轮廓改成 Photoshop 式的"蚂蚁线"：黑色 2px 底层 + 白色 1px 虚线（cosmetic 笔，屏幕空间），虚线相位每 70ms 前进一个文档单位，因此在任何缩放级别下都保持同样的观感，用户能一眼确认选区边界。约束：

- 只在所属窗口可见且未最小化时走动（`visibleChanged`/`visibilityChanged`/`ItemSceneChange` 都会重新判定），窗口关闭、隐藏或最小化即停止定时器，不在后台空转重绘；
- 相位变化只触发重绘，不重建 `QPainterPath`，也不重复发 `geometryChanged`（几何只在步骤/缩放/文档矩形变化时重建）；
- 预览拖拽（矩形/椭圆选区拉框）同样使用蚂蚁线。

栅格化选区（套索/魔棒）目前用其包围盒画轮廓；精确到像素边界的轮廓需要内核导出掩码边界段，属于后续工作。

## 验收

- Rust：多边形栅格化（凹多边形、自交、越界、点数上限校验）、魔棒容差与连通性（相邻不同色不连通、透明区域、容差 0/大值）、掩码与矩形的合并/反转、v2 文本往返与 v1 兼容、掩码预算拒绝。
- FFI：新 DTO 校验（点数、容差、非法 kind、非有限坐标）、`read_sized` 尺寸检查、掩码窗口读取的越界/旧 publication BUSY、步骤 kind 扩展。
- Qt：工具形态面板出现四个形态且都可用；套索拖动后产生选区且可用 Ctrl+Z 撤销；魔棒点击产生选区、Shift 加选、重复点击不产生假记录；状态栏与画笔详情栏显示新工具名。
- Qt 轮廓：`selectionOutlineMarchesWhileVisibleAndStopsOtherwise` 断言无选区时相位不动；启用选区后相位持续推进、`geometryChanged` 不重复发射；窗口关闭后相位停止（定时器不再空转）。
- 全新 `python tools/check.py` 与两倍 DPI 通过后构建部署。
