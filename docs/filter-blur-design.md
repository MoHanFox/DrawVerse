# 滤镜：高斯模糊（2026-10-10）

用户要求实现"滤镜 → 模糊 → 高斯模糊"。本文件先定实现方案与验收，再编码（本轮交付设计与菜单接线点，未改内核）。

## 现状（已核对的接入点）

- 图层像素存放在稀疏 `Tile`（64×64 的 `Arc<Tile>`）里，历史用 `Command::Stroke { layer, changes }`：每条 `TileChange` 携带 `before/after` 两个 `Arc<Tile>` 快照。`undo`/`redo` 直接把快照换回去，**不重新计算**。
- 因此"滤镜"最自然的实现是：**改瓦片 + 记录 before/after 快照 + 一条历史**，与笔画完全同一条路径，天然支持撤销/重做、逐条历史回放、磁盘暂存编码。
- `Document::commit_with_action`、`self.pool.attach/trim`、`dirty_tile/dirty_all` 都是现成的；`LayerAppearance` 的 `offset_x/offset_y` 决定"层局部坐标 ↔ 画布坐标"，未平移的图层可以直接按画布坐标写瓦片，已平移的图层必须走偏移换算（现有笔画代码里有这层分支，滤镜必须照抄，否则会把像素写到错位置）。
- 菜单项已存在：`滤镜 → 高斯模糊…`（`objectName:"filterGaussianBlurAction"`，当前禁用标注"待支持"）。

## 内核方案

1. **新命令类型** `Command::Filter { layer, changes }`，与 `Command::Stroke` 同样保存 `TileChange` 快照；`apply_history` 增加对应分支（forward/backward 都是换瓦片）。新增 `HistoryAction::GaussianBlur`（显示"高斯模糊"）。
2. **`Document::gaussian_blur(layer, radius)`**：
   - 校验：`radius` 取 0.1–250 的有限值；活动图层必须是像素图层（组、蒙版、被完全锁定的图层返回明确错误，不静默失败）；笔画进行中返回 BUSY。
   - 采样：对本图层所有非空瓦片的**并集包围盒**做一次带 padding 的读取（padding = ceil(3σ)），用三层 box blur 逼近高斯（σ = radius/3，误差 < 1%），在**线性预乘 RGBA** 空间做，且**只模糊颜色、不模糊 alpha 之外的东西**——即对预乘分量做加权平均即可保证边界不出现黑边（预乘空间线性平均天然正确）。
   - 边界：图层外视为透明（预乘 0），不做边界复制，避免把画布外的黑边拉进来。
   - 写回：把结果按 64×64 切片写回瓦片；空瓦片丢弃，保持稀疏；`pool.attach` + `trim`。
   - 记录：对每个发生变化的瓦片收集 `before/after`（`Arc` 指针相等则跳过），`commit_with_action(Command::Filter{..}, HistoryAction::GaussianBlur)`；无变化时返回 `false`，**不产生历史记录**（与"无变化不生成假记录"一致）。
   - 失败回滚：任何一步失败都把已写入的瓦片按 before 快照还原，不留半成品（照抄 `begin_stroke` 的 `restore_stroke` 模式）。
3. **ABI 1.13**：`paint_session_gaussian_blur(core, session, layer_id: u64, radius: f32, out_sequence: u64*)`，只新增符号，不改既有 DTO；`radius` 非法、图层不存在、图层被锁分别返回 INVALID/NOT_FOUND/BUSY。

## Qt / QML

- `PaintCoreClient::gaussianBlur(quint64 layer, qreal radius)`，走既有 `submit` 队列与世代校验，完成后由帧刷新带动画布与导航器。
- 菜单 `滤镜 → 高斯模糊…` 打开一个紧凑对话框（半径滑块 0.1–250，实时数值，预览按钮可选），确定后调用；对话框打开期间禁用绘画，取消不产生任何修改。
- 菜单项启用条件：`PaintClient.ready && !drawing && !layerEditBusy && !fileBusy`，且活动图层是像素图层。

## 验收

- Rust：
  - 均匀色块模糊后仍为同色（能量守恒、无边界暗边）；
  - 单点脉冲模糊后呈径向对称且总能量近似不变（预乘 alpha 下和值守恒）；
  - 半径 0.1 近似不变、半径增大更平滑（方差单调增）；
  - 稀疏图层：只写回受影响瓦片，空白瓦片不被实体化（`tile_count` 断言）；
  - 撤销/重做逐像素还原为原图；无变化的重复调用不新增历史；
  - 组/蒙版/锁定图层拒绝、笔画中拒绝、非法半径拒绝，且失败后文档与历史不变。
- FFI：通过 C11/C++20 静态与共享调用各一次，检查非法参数与成功路径的序列号推进。
- Qt：菜单项触发对话框→确定后画布像素变化、`modified` 置位、Ctrl+Z 一步还原、导航器同步；取消不产生修改与历史。
- 完整 `python tools/check.py`（含两倍 DPI）后构建部署。

## 分步实施

1. 内核 `Command::Filter` + `HistoryAction::GaussianBlur` + `gaussian_blur` + Rust 回归；
2. ABI 1.13 与生成头、C/C++ 调用回归；
3. `PaintCoreClient` + 高斯模糊对话框 + 菜单接线；
4. Qt 回归、完整检查、部署。
