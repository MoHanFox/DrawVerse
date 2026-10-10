# 油漆桶工具（2026-10-11）

## 目标

在画布上点击即用当前前景色填充与落点颜色相近的**连通区域**，参数为容差、连续区域开关、不透明度、以及"填充选区/整层"的取舍。与魔棒共用同一套洪水填充算法，不允许两套实现。

## 现状（已核对的接入点）

- **连通区域已经实现**：`paint-core/src/wand.rs` 的 `wand_shape(document, seed_x, seed_y, tolerance)` 做扫描线洪水填充，按通道容差比较（premultiplied alpha 归一化），产出 `SelectionShape { kind: Mask|Rectangle, mask: Option<SelectionMask> }`。
- **按覆盖率写像素的机制已经实现**：`Document::dab_with_selection::<SELECTED>()` 在目标矩形内逐像素算 `coverage`，并对每个瓦片记录 `before/after` 快照，最终由 `end_stroke` 汇总成一条 `Command::Stroke` 历史（`begin_stroke` → `dab` → `end_stroke`）。
- 因此"填充 = 用一个覆盖率掩码驱动一次笔触"是**复用既有机制**，不需要新的像素写入路径。

## 设计

### 1. 内核：临时填充选择，不动用户选区

关键约束：**填充不能改变用户当前的选区**（那是魔棒的语义，不是油漆桶的）。但 `dab_with_selection` 读的是 `self.selection`。

方案：新增 `Document::fill_region(&self, seed: (i64,i64), tolerance: f32, color, opacity, contiguous: bool) -> Result<bool>`：

1. `idle()`、校验落点在文档内、容差 0–255、颜色有限、图层不是组/被完全锁定；
2. 用一个**局部** `Selection`（`wand_shape` 产物）驱动一次内部笔触：
   - 在 `Document` 内加一个 `fill_selection: Option<Selection>` 覆盖字段，`dab` 侧改为 `self.fill_selection.as_ref().unwrap_or(&self.selection)`；该字段只在 `fill_region` 执行期间存在，结束（含失败路径）必定清空——用 guard 结构保证，避免"填充把用户选区改掉/清掉"这类不可逆副作用；
   - 笔触用一个"矩形笔刷"覆盖 `wand_shape` 的包围盒，覆盖率由掩码给出（掩码外 0，掩码内取 `opacity`），
   - 非连续模式下掩码取整层同类像素（`wand_shape` 的 `contiguous=false` 变体），实现时给 `wand_shape` 增加一个 `contiguous: bool` 参数；
3. 走 `begin_stroke`/`dab`/`end_stroke` 的既有生命周期，因此：一次历史记录（新增 `HistoryAction::Fill`，显示"油漆桶"）、可撤销、失败原子回滚、稀疏瓦片保持；
4. 空区域（没有任何像素被改变）返回 `Ok(false)`，**不产生历史**（与"无变化不生成假记录"一致）。

`wand_shape` 增加 `contiguous` 参数属于行为扩展，需要同步更新 `set_selection_magic` 与 ABI（见下）。

### 2. C ABI 1.13

只新增符号：

- `paint_session_fill_region(core, session, x: i64, y: i64, tolerance: u32, contiguous: u32, opacity: f32, out_sequence: u64*)`：颜色取当前画笔色（与绘画一致，不额外传色，避免两处颜色状态不一致）；
- `PAINT_HISTORY_FILL = 25`。

失败必须无副作用：落点在文档外、容差越界、图层是组/被锁定、笔画进行中分别返回 INVALID / BUSY。

### 3. Qt / QML

- `PaintCoreClient::fillRegion(x, y, tolerance, contiguous, opacity)`；工具栏新增油漆桶槽位（图标 + 快捷键 `G`），与套索/魔棒共用"选区工具槽位 + 右键形态面板"的既有交互（点击选择、右下角三角形提示）。
- 点击画布即填充（`CanvasItem` 在 `selectionTool()` 为油漆桶时把落点转文档坐标并调用），**不进入拖拽状态**；`Alt` 仍是取色器，`Shift`/`Alt` 的"加选/减选"对填充无意义，因此填充时忽略它们（文档里写明）。
- 状态栏与画笔详情栏显示"油漆桶 · 容差 N · 连续/非连续"。
- 参数：容差（0–255）、连续区域开关、不透明度（1–100%），放在画笔设置面板的"油漆桶"分组里，与画笔参数分开，避免污染笔刷预设。

## 验收

- Rust：
  - 三色带文档中，点击中间区域只填该带；容差增大后跨越相邻带；`contiguous=false` 时整层同类像素都被填；
  - 填充**不改变用户选区**（填充前后 `selection().to_text()` 相等）；
  - 一次历史、可撤销、重做；空区域不产生历史；
  - 掩码外像素逐像素不变；图层偏移（`offset_x/offset_y`）下填充位置正确；
  - 组/蒙版/锁定图层、越界落点、越界容差、笔画中均被拒绝且文档不变。
- FFI：非法参数、未初始化会话、失败无半成品。
- Qt：点击即填充且生成一条历史；`Alt` 仍取色不填充；容差/不连续切换后填充范围变化；撤销一步还原；填充前后用户选区不变。
- 全量 `python tools/check.py`（含两倍 DPI）后构建部署。

## 分步实施

1. 本文档；
2. 内核 `wand_shape(contiguous)` + `fill_region` + `HistoryAction::Fill` + Rust 回归；
3. ABI 1.13、生成头、C/C++ 调用回归；
4. Qt 客户端 + 工具槽位/图标/快捷键 + 画笔设置面板分组；
5. Qt 回归、全量检查、部署。

## 风险

- `dab` 的覆盖率路径是为"圆形笔刷 + 选区乘算"优化的；填充需要"任意掩码覆盖率"，实现时要确认每像素只做一次乘算、不引入额外几何判断，否则大区域填充会明显变慢。基准沿用 `docs/painting-performance-*` 的固定输入。
