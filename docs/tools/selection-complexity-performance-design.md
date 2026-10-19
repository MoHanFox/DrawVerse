# 复杂图像的套索、魔棒与油漆桶性能（2026-10-19）

目标是减少选区计算、轮廓构造、填充和无效画布合成，不改变选区数据、容差、AA、历史和文件格式。用户追加油漆桶 3840×2160 填充性能，与魔棒共用区域算子；参考同一提交的 `crates/engine/src/paint_cmds.rs::bucket` 对 `wand_region` 的复用。沿用 Rust 核心 / C ABI / Qt 分层，CPU 负责区域生长与颜色判断；显示交给 Qt，软件后端也必须工作。

## 参考与问题

核对 PhotoCraft 官方仓库 `storytold/photocraft` 的提交 `d95fdf3181c01ad41e95456091a394b105a9afb2`：`crates/algo/src/selection.rs::wand_region` 使用连续段扩散；`crates/ui-egui/src/channels_panel.rs` 的 `channel_thumbs_skip_selection_steps_and_reuse_textures` 和 `canvas.rs` 的 navigator 更新把选区变化与图像变化区分。参考这些思路独立实现，不复制其颜色距离或 AA 规则。

DrawVerse 当前魔棒逐像素读取/解锁瓦片、每个匹配点入队四邻居；Qt 把掩码拆为行矩形、构造 QRegion 后对大量矩形做 QPainterPath::simplified 和布尔裁切；纯选区操作还提高渲染屏障并重新合成画布。复杂边界、图层栈和大选区分别放大这些成本。

## 实施边界

- 魔棒使用有界的按瓦片分类缓存，读取同一行一批像素，保持原直通 sRGB/alpha 最大通道差及一色阶 AA。连续模式按水平段扩散，保留四连通和零权重阻断；全局模式顺序扫描。访问瓦片、掩码、编码和队列资源限制必须明确，失败保持选区及历史原状。
- 共用 `color_region::RegionDiscovery<W,F>`，魔棒保留 u8 权重，桶填充保留 f32 权重；连通判定与写入权重分离，避免精度变化。桶填充批量读取源与选区覆盖；对齐输出按瓦片批量写入，全覆盖不透明填充直接构造整块；偏移目标按行分区写入，保持蒙版/透明锁/源合成/选区边界和单历史原子提交。额外基准记录真实 3840×2160 填充和撤销重做。
- 掩码轮廓直接沿半覆盖像素的暴露边跟踪闭合环，合并共线段，包含孔洞和对角接触，避免把内部行矩形交给通用路径求交。完全位于画布内的路径跳过无意义裁切；路径几何缓存不随蚂蚁线相位变化失效。
- 纯 Selection/Wand 以及纯选区撤销重做复用已经完成或正在生成的相同像素帧。只重新标记发布修订，不复制像素载荷，不把像素编辑误判为选区操作；新文档、视口、实际绘画和复杂预览屏障沿用原约束。不修改 ABI 布局和文件格式。

## 验收条件

用固定复杂栅格/连续与全局模式/多图层源及密集套索测试记录前后 Release 耗时和结果摘要；小尺寸随机图与原四邻接标量参考逐覆盖比较，覆盖 AA、平均取样、偏移与现有组合/历史回归。Qt 验证碎片/孔洞/对角接触轮廓、组合与边界裁切，记录构造耗时；原生验证纯选区复用像素载荷、修订更新及下一次绘画仍刷新。执行最小 Rust/Qt 回归、ABI/生成头/格式检查和 Windows Release。记录未验证设备和上限，不以基准代替实际手感。

## 代码与预算

- `core/crates/paint-core/src/document/tools/color_region.rs`：`RegionDiscovery::new(width,height,read_row)` 接受返回 `(权重,是否可连通)` 的行读取闭包；`contiguous(seed)` 或 `global()` 发布同一瓦片结果。连续模式按需分类 64 像素行，位运算跨瓦片查找和标记连续段，避免为单个小色块计算整块 64×64 源图。
- 连续模式至多 4096 个访问瓦片；全局模式沿用 16Mi 像素扫描预算，只保留至多 4096 个匹配瓦片，并使用一个额外临时瓦片。长窄画布可以扫描超过 4096 个源瓦片，不因缓存增加新拒绝条件。待处理段栈最多 262144 项，约 2MiB；魔棒权重/位图载荷约 20MiB，桶权重/位图约 68MiB，另加有界索引。桶的输出 staging、文档分页与历史沿用原预算。魔棒包围掩码仍最多 16MiB，编码最多 1MiB。
- `bucket.rs` 的颜色比较仍复用原 `compare_color` / `match_weight`；`wand.rs` 只在既有位置量化至 u8，桶不量化为 u8。种子平均参考色仍逐行逐点累加，未引入色差近似或相邻颜色漂移。
- `ui/src/canvas/SelectionOverlay.cpp`：沿暴露边追踪，向右优先处理对角相接，奇偶填充保留孔洞；共线段合并后缓存几何。蚂蚁线相位仅重绘；画布内路径跳过全图求交。多个复杂选区组合与越界裁切仍走 Qt 路径布尔运算，不宣称所有组合都已消除这项成本。
- `core/crates/paint-ffi/src/engine/session.rs`：纯选区修改与选区撤销重做记录像素等价修订区间，`StampedFrame` 与新修订共享 `Arc<RenderFrame>`。请求 ID、代际、读帧水位和旧帧校验保持，实际绘画/填充退出等价区间。
- `ui/src/engine/PaintCoreClient.cpp`：接收新修订但像素相同的帧时保留旧 `QImage` 身份，`CanvasItem` 因 `cacheKey` 不变保留场景图纹理。仍有有界帧读取/比较开销，不能写成全链路零复制。

本轮未新增 wgpu 选色或 GPU 填充算子；CPU 区域判断与 Qt 显示各司其职。轮廓仍通过 `QQuickPaintedItem` 构建 CPU 线条并由 Qt 显示，未改为独立 GPU 蚂蚁线 shader。笔刷轮廓线宽现有初始化与旧文件缺省读取均已是 0（单条自适应细线），本轮复核默认与保存值保留，未重置用户已保存偏好。

实测与限制见[验收](../validations/tools/selection-complexity-performance-validation.md)。
