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

1. **尺寸发布未传到 UI（已缩小范围）**。新增的会话级回归 `transform_command_publishes_the_new_canvas_size`（`paint-ffi` 的 `actor_tests`）证明：通过 `Session::submit(Operation::Transform(..))` 提交后，**发布确实会带上新尺寸**（(32,16) → (16,32)，逆时针再回到 (32,16)），worker 保持 `Running`，不会 panic。同时 Qt 侧插桩显示：命令**已提交**（真实序列号）、worker 侧**确实执行了变换**，但客户端此后**没有收到任何新发布**，`paint_session_info` 也仍返回旧尺寸。
   因此问题**不在内核、不在 ABI、不在 worker 发布循环**，而在 Qt 客户端这一段（`metadataPending` 门闩、`publish()` 的 `paint_session_info(publication)` 调用、或 UI 事件循环与 worker 的交互）。
   本轮曾怀疑是 `catch_unwind` 捕获 panic 后线程退出（那会导致"提交后再无发布"），但上面的会话测试排除了它。
   临时插桩（`qWarning`/`eprintln!`）已全部移除，工作区无调试残留。
2. **取色器取样在测试环境下取到全黑**。`CanvasItem::pickColorAt` 读的是已渲染帧（`m_image` + `m_imageRegion`），测试里帧为 71×71、区域 64×64，但取到 `#000000`。计划改为走 ABI 只读取样：`Document::sample_pixel` 已在 `paint-core` 落地（只读、不进历史、不标脏），只差 `paint_session_sample_pixel` 导出与客户端接线。

两者的 Qt 用例都**没有提交**（失败的测试不入库）：`canvasRotateAndFlipGoThroughTheMenuActionsAndUndoInOneStep` 与 `altEyedropperSamplesWithoutHistoryAndBracketKeysResize` 已从 `ui/tests/ui_tests.cpp` 与 CTest 列表中移除，待问题收敛后连同修复一起补回。

## 下一步

1. 在 Qt 客户端这一段继续查：`publish()` 里对 `paint_session_info(publication)` 的调用是否会因 `PAINT_BUSY` 提前 return（那会把整次发布丢弃，且 `m_publication` 不推进，之后 `info.publication != m_publication` 才会再次尝试——但若 `metadataPending` 一直是 true 就永远不再尝试）。这是当前最可疑的一处。
2. 取色器改为 ABI 只读取样，避免依赖渲染帧；随后补回对应用例。
3. 之后再回到队列：油漆桶、涂抹、高斯模糊、Ctrl+T、结构性调整、液化、.vbr 笔刷、PS 色相/饱和度·曲线·色阶。

> 附：用户确认"移动图层有时会报错"的机制是——创建剪贴蒙版后把底层（基底）移走，剪贴层失去基底就会报错。这正是 `docs/layer-selection-design.md` 与 `clipping.rs` 里"基底消失时级联释放剪贴层"那条规则的来源，已实现并推送（`139dee3`）。
