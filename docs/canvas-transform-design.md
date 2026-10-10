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

1. **尺寸发布未传到 UI**。直接在 Qt 里调用 `transformCanvas(0)` 返回成功，内核侧已用插桩确认操作真的执行了（`DBG before (32, 16) 0` → 第二次 `(16, 32)`），但 `PaintCoreClient::documentWidth()` 始终停留在旧值（32）。核心测试证明 `Document::dimensions()` 会更新，因此问题在**会话发布/客户端轮询**这一段（`paint_session_info` 的 width/height 或 `PaintCoreClient` 对 `metadata` 的处理），不在变换本身。因为发布不更新，用户此时看到的画布尺寸也不会变，所以这项功能**还不能算可用**。
2. **取色器取样在测试环境下取到全黑**。`CanvasItem::pickColorAt` 读的是已渲染帧（`m_image` + `m_imageRegion`），测试里帧为 71×71、区域 64×64，但取到 `#000000`。需要确认离屏/软件后端下帧是否真的填充，或改为走 ABI 取样（`paint_session_sample_pixel` 之类的只读接口，不会进历史）。

两者的 Qt 用例都**没有提交**（失败的测试不入库）：`canvasRotateAndFlipGoThroughTheMenuActionsAndUndoInOneStep` 与 `altEyedropperSamplesWithoutHistoryAndBracketKeysResize` 已从 `ui/tests/ui_tests.cpp` 与 CTest 列表中移除，待问题收敛后连同修复一起补回。

## 下一步

1. 先解决"发布未更新"：在 `paint_session_info` 返回路径与 `PaintCoreClient` 的 `metadata` 槽上各加一条断言级验证，定位是后端没发布还是前端没采纳；修好后把旋转用例补回。
2. 取色器改为 ABI 只读取样，避免依赖渲染帧；随后补回对应用例。
3. 之后再回到队列：油漆桶、涂抹、高斯模糊、Ctrl+T、结构性调整、液化、.vbr 笔刷、PS 色相/饱和度·曲线·色阶。
