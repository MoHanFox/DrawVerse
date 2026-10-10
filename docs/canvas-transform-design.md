# 画布旋转/翻转：实现与未决问题（2026-10-11）

## 已实现并验证

**内核 `paint-core`**

- `CanvasTransform { Rotate90Cw, Rotate90Ccw, Rotate180, FlipHorizontal, FlipVertical }`，`document/transform.rs`。
- 正交变换是**无损整数置换**：每个目标像素精确对应一个源像素，不需要重采样。只改画布尺寸、每个图层的局部范围与瓦片位置；90°/270° 交换宽高，翻转不换。
- 逐图层重写内容（含蒙版与嵌套组内图层），空图层与空组保持稀疏；偏移归零、局部范围改为新画布尺寸。
- 一条历史记录：新增 `Command::Transform { before, after, before_size, after_size }` 与 `HistoryAction::TransformCanvas`（24）；撤销同时还原**像素与画布尺寸**，重做同理。`history_storage` 复用结构命令的编码路径（变换后的像素就在栈里）。
- 失败原子性：先在副本上完成全部图层变换，任何一步失败都不改文档。

**ABI 1.12**

- `paint_session_transform_canvas(core, session, kind, out_sequence)`，`PAINT_CANVAS_TRANSFORM_*` 常量（0–4）；未知 kind 在提交前就拒绝。次版本升到 12，生成头已重新生成。

**Qt**

- `PaintCoreClient::transformCanvas(kind)`；图像菜单四个条目（顺时针 90°/逆时针 90°/垂直翻转/水平翻转）已从占位改为真实动作，顺时针带 `R` 快捷键。
- 注意：`transformCanvas` **不**设置 `m_pendingLayer`——会话对该调用是串行的，返回时画布已经变换完成，标记一个"在途编辑"会永久阻塞后续所有图层编辑（这正是本轮发现并修掉的 `m_pendingSelPath` 永不清零那个 bug 的同类问题；该字段已删除，套索/魔棒改为使用 worker 返回的真实序列号）。

**回归**：`core/crates/paint-core/tests/transform.rs` 五个用例——90°×4 回到原图且宽高交换、顺逆时针互逆、翻转两次为恒等与 180° 定位、一次撤销同时还原像素与尺寸、稀疏性与图层顺序保持。

## 未决问题（本轮未能收敛，需专门排查）

## 真因与修复（已收敛）

**根因：`PaintCoreClient::publish()` 的历史名称表没跟着 `HistoryAction` 扩展。**

`names` / `icons` 只覆盖 kind 0–19，而本轮先后新增了 `EllipseSelection`(20)、`LassoSelection`(21)、`MagicSelection`(22)、`ReleaseClipping`(23)、`TransformCanvas`(24)。旧代码遇到表外 kind 时直接

```cpp
if(entry.kind>=static_cast<uint32_t>(names.size())){emit failure(QStringLiteral("无法识别的历史操作"));return false;}
```

于是**任何新操作一进历史，整次发布就被丢弃**：画布尺寸、图层列表、选区、历史全部停在旧状态，`m_publication` 也不推进。这正好解释两件事——旋转提交成功且 worker 确实执行了、但 UI 永远收不到新尺寸；以及用户观感上的"移动图层有时会报错"（移走剪贴基底产生的 `ReleaseClipping` 落在表外，直接触发那句失败提示）。

**修复**

1. 名称与图标表补齐到 kind 24（套索选区、魔棒选区、释放剪贴蒙版、画布旋转）；
2. 表外 kind **不再中断发布**，改为显示"未知操作"与通用图标——任何一个未知历史类型都不应该冻结整个 UI；
3. 新增会话级回归 `transform_command_publishes_the_new_canvas_size`（`paint-ffi` 的 `actor_tests`）：提交变换后发布带上新尺寸、两次变换尺寸正确、worker 保持 `Running`。

**取色器**：`CanvasItem::pickColorAt` 不再读可能未渲染或已降采样的帧，改为走新 ABI `paint_session_sample_pixel(view, x, y, out_rgba)`（只读，不进历史、不标脏、不改 revision），离屏与软件后端下同样有效。注意核心 `Document::sample_pixel` 那条路用不了——发布结构里不保存文档指针，所以取样读的是"已渲染帧"这一路。

两个 Qt 用例 `canvasRotateAndFlipGoThroughTheMenuActionsAndUndoInOneStep` 与 `altEyedropperSamplesWithoutHistoryAndBracketKeysResize` **已补回测试套件**并通过（含 `R` 快捷键断言、Alt 取色不产生历史、`[`/`]` 调尺寸）。

## 已知缺陷：多瓦片图层旋转会丢内容（未修）

手工测试发现：960×640 的画布上画一笔（43,394 个着色像素），执行 90° 顺时针旋转后画布正确变成 640×960，但着色像素只剩 10,768 个——**大量内容被丢弃**，而且残余内容落在错误的位置（网格采样只在一个点上看到颜色）。

已确认的边界：

- 单元测试 `transform.rs` 的五个用例全部通过——它们都在 16×8 / 64×64 这类**单瓦片**范围内，覆盖不到多瓦片重排；
- 背景图层（`initialize_white_background`）旋转后是 150 个瓦片、614,400 个不透明像素，与 640×960 完全吻合，说明"瓦片数量与遍历范围"这一层是对的；
- 因此缺陷在 `transform_layer` 的多瓦片读写路径上（源瓦片索引 / 目标瓦片落位），而不是坐标映射本身——`CanvasTransform::map` 的逆映射在小文档上已被验证。

**结论：图像菜单的画布旋转/翻转在修好这条之前不应作为可用功能交付。** 用户真正要的是**旋转视图工具**（只转视图、不动像素），那条路径不碰图层瓦片，因此不受本缺陷影响，优先级更高。修复本缺陷需要一条**多瓦片**回归用例（例如 200×120 文档画一条跨四块瓦片的笔画，旋转后逐像素比对集合是否满足双射），先写用例再改实现。

## 验收

- Rust：`quarter_turns_swap_dimensions_and_rotate_content`、`counter_clockwise_is_the_inverse_of_clockwise`、`flips_mirror_without_changing_dimensions`、`transform_is_one_undoable_step_including_the_size`、`sparsity_and_layer_order_survive_a_transform`、`transform_command_publishes_the_new_canvas_size`。
- Qt：`canvasRotateAndFlipGoThroughTheMenuActionsAndUndoInOneStep`（菜单动作 + `R` + 一次撤销还原像素与尺寸 + 翻转保持尺寸）、`altEyedropperSamplesWithoutHistoryAndBracketKeysResize`（取样准确、无历史、无 revision 变化、括号键调尺寸）。

## 下一步

- 队列继续：油漆桶、涂抹、高斯模糊、Ctrl+T、结构性调整、液化、.vbr 笔刷、PS 色相/饱和度·曲线·色阶。

> 附：用户确认"移动图层有时会报错"的机制是——创建剪贴蒙版后把底层（基底）移走，剪贴层失去基底就会报错。这正是 `docs/layer-selection-design.md` 与 `clipping.rs` 里"基底消失时级联释放剪贴层"那条规则的来源，已实现并推送（`139dee3`）。
