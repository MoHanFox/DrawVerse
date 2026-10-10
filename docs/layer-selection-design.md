# 图层多选与操作（2026-10-10）

用户要求：图层支持快捷键复制粘贴；Shift 点选范围（含中间的所有图层与文件夹）、Ctrl 多选；右键菜单提供"向下合并（未多选）/合并图层（多选）、创建剪贴蒙版、复制图层、删除图层、从图层创建图层组"。

## 现状

- 内核只有**单选**活动图层：`Document::active_layer` / `set_active_layer`，`PaintCoreClient.activeLayer` 与 `LayersPanel` 的 `row.active` 都基于它。
- 已有能力：`add_layer`、`remove_layer`、`move_layer`、`set_layer_properties`、`set_layer_appearance`、分组/解组/重挂（FFI `paint_session_group`、`paint_session_drop_layer`、`paint_session_set_layer_clipping`）。
- **缺失**：图层复制/粘贴、向下合并、多选合并、从多选创建组。

## 设计

### 1. 多选（UI 层，不改文档语义）

多选属于"面板选择状态"而不是文档状态，因此保存在 `PaintCoreClient`（不写入历史、不写盘）：

- 新增 `selectedLayers: QVariantList`（quint64 列表）与 `Q_INVOKABLE selectLayers(id, modifiers)`；
  - 普通点击：选中集合 = {id}，活动图层 = id；
  - **Ctrl/Cmd 点击**：切换 id 在集合中的存在性，活动图层随最后一次点击；
  - **Shift 点击**：从当前活动图层到 id 的**可见顺序区间**全部选中（含跨越的所有图层与文件夹），活动图层 = id；
  - 选择集合只包含仍然存在的图层（删除/结构变化时由 `layersChanged` 清理）。
- `LayersPanel` 行高亮改为"在选中集合中"，活动图层行加更亮的边框，用户能区分"选中"与"活动"。
- Shift 区间使用 `PaintClient.layers` 的当前可见顺序（折叠的组内成员不参与，和 PS 一致）。

### 2. 复制 / 粘贴 / 复制图层

- `Ctrl+C`：把选中图层（含组与蒙版）序列化为应用内剪贴数据（像素以既有文档快照机制导出，不写系统剪贴板的位图）。
- `Ctrl+V`：把剪贴数据作为新图层插入到活动图层上方；名称加"副本"，多次粘贴递增。
- `Ctrl+J`：复制当前选中图层（等价于复制＋立即粘贴到自身之上）。
- 这三者都通过新的核心/ABI 接口完成，走历史（一次操作一条记录，可用 Ctrl+Z 撤销），命名"复制图层"。

### 3. 合并

- **向下合并**（未多选时可用）：把活动图层与其下方同级图层按现有合成规则合并为一个图层；若活动图层是剪贴蒙版层或蒙版，拒绝并给出原因（不静默失败）。
- **合并图层**（多选 ≥2 时可用）：把选中集合合并为一个图层，插入到集合中最上方图层的位置。
- 合并必须复用既有合成路径（含混合模式、不透明度/填充、剪贴、组隔离、蒙版），不能自己写一套权重近似：实现为"把待合并图层临时组成一个内部组，用 `groups::composite_row` 渲染到目标图层，再删除被合并项"，一次历史记录。
- 合并后的图层名沿用最上方（或最下方，向下合并时）图层名，`HistoryAction` 新增 `MergeLayers`（显示"合并图层"）。

### 4. 右键菜单

`LayersPanel` 已有的行右键/菜单按钮弹出 `groupOperationsMenu`，按选中数量切换：

| 条目 | 启用条件 |
|---|---|
| 向下合并 | 未多选（选中数 = 1）且下方存在同级图层 |
| 合并图层 | 选中数 ≥ 2 |
| 创建剪贴蒙版 / 释放剪贴蒙版 | 选中 1 个且存在上方同级图层 |
| 复制图层 | 选中 ≥ 1 |
| 删除图层 | 选中 ≥ 1 且不会删空文档 |
| 从图层创建图层组 | 选中 ≥ 1 |

未实现的能力（如"转换为智能对象"）不放进菜单，避免点了没反应的假项。

## C ABI 1.13

只新增符号，不改既有 DTO 布局：

- `paint_session_duplicate_layers(core, session, ids: *const u64, count, insert_above: u64, out_ids, out_count)`：复制给定图层（含组子树与蒙版），返回新图层 id；
- `paint_session_merge_layers(core, session, ids: *const u64, count, mode: PAINT_MERGE_DOWN|PAINT_MERGE_SELECTION, out_layer)`：合并并返回结果图层 id；
- `paint_session_clipboard_copy(core, session, ids, count)` / `paint_session_clipboard_paste(core, session, insert_above, out_ids, out_count)`：应用内图层剪贴板。

## 验收

- Rust：多选区间计算（含文件夹、跨组边界、折叠组）、复制图层（含组子树与蒙版、命名递增）、向下合并与合并选区的像素结果与"临时组渲染"一致、拒绝合并蒙版/剪贴层、历史各生成一条记录且可撤销、复制粘贴后图层 id 唯一。
- FFI：新 DTO 校验（空集合、未知 id、越界 count、重复 id），失败不产生半成品状态。
- Qt：Ctrl 点击多选、Shift 范围包含中间文件夹、选中集合在删除/撤销后自洽；右键菜单按选中数量切换条目与启用状态；Ctrl+J 复制、Ctrl+C/V 粘贴、Ctrl+E 向下合并各生成一条历史且可撤销。

## 分步实施

1. 客户端多选状态与 `LayersPanel` 高亮（纯 UI，可先交付并验证交互）；
2. 右键菜单结构与启用条件（合并类先接内核）；
3. 内核：复制图层（含组/蒙版）、向下合并、多选合并、图层剪贴板 + ABI 1.13；
4. 快捷键接线与 ORA/历史回归；
5. 完整 `python tools/check.py`、两倍 DPI、构建部署。
