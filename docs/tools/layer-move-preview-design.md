# 图层移动实时预览

## 目标与契约

移动工具和 Ctrl 临时拾取移动在按下时确定目标，拖动使用相对起点的绝对整数位移；更新不写历史，松手仅提交一次现有移动命令，Esc、失去输入抓取或窗口停用取消。空白拾取不移动先前活动层。保持位置锁、组/蒙版共同移动、画布外像素保留及原有撤销语义。

核心提供 begin/update/commit/cancel 会话，只捕获受影响层的原始外观元数据，避免为每次拖动复制所有图层瓦片表；预览偏移从不可变基线计算，全部目标校验成功才发布。取消恢复原始外观；提交先恢复再调用现有 move_layer，避免累积误差和重复历史。会话期间其他修改拒绝，保存也不能捕获临时状态。

## 共用呈现与输入

普通平面 Normal 图层栈复用液化的原图/下层/上层常驻纹理和 x/y/u/v 网格包，移动只需 2×2 网格，不读取或重采样像素。源纹理包含画布外瓦片和透明边界。组、蒙版、裁剪、特殊混合、置入图片和软件渲染采用 CPU 像素预览，保持功能一致。交互结束后保留最后预览，待最终像素帧接管。

ABI 1.63 增加移动会话操作、会话标记和通用 resident preview 查询；保留 ABI 1.62 液化入口和包格式。Actor 只合并相邻的绝对移动更新，提交携带最终位移并保留队列关闭屏障。Qt 在一帧中合并最新输入，快速按下/松开也能有序关闭。鼠标和平板使用同一链路。

共用源切分一次构造原图/上下层快照，避免三次克隆整个图层栈。后端能力读取当前窗口 `rendererInterface()->graphicsApi()`，窗口绑定与场景图初始化后通知 QML；不能把全局 `QQuickWindow::graphicsApi()` 配置当成软件场景图的实际后端。

GPU 用于纹理变换与合成，整数位移本身无需 wgpu 计算算子。CPU 与 GPU 共用文档偏移/历史语义；不将 GPU 输出回读用于交互。

## 最小验收

- 核心：更新可见而历史不变；提交单历史、撤销/重做正确；取消恢复；锁定拒绝；组/蒙版语义；画布外往返；四顶点常驻来源复用。
- ABI/Actor：有序 begin/update/commit/cancel，绝对更新合并，最终位移和容量查询，旧液化回归。
- Qt：松手前真实像素/常驻预览随动，松手单历史、Esc 恢复、Ctrl 拾取及软件回退；Windows Release 构建。
- 不以自动化运行时间代替实体鼠标/平板的持续手感结论，不运行无关全量测试。

## 代码入口与使用

- `core/crates/paint-core/src/document/tools/layer_move.rs`：基线、原子外观更新、提交和取消；`liquify_preview.rs::resident_source`：共用不可变源切分。`ResidentPreview` / `ResidentPreviewSource` 与旧液化类型保持兼容。
- `core/crates/paint-ffi/src/engine/session.rs`：移动操作、绝对更新尾合并、关闭屏障、共用常驻资产缓存和渲染调度；`document/layer_api.rs` / `liquify_api.rs`：ABI 会话/共用包查询。
- `ui/src/engine/PaintCoreClient.cpp`：16ms 最新输入槽、开始确认/最终关闭、共用资产与网格取帧；`ui/src/canvas/CanvasItem.cpp`：鼠标/平板、Esc/失焦取消；`LiquifyPreviewNode.cpp` 的材质供两个工具复用，未另建 shader。

使用 V 移动工具，或按住 Ctrl 点击可见内容后拖动。无需 GPU 计算开关，显示加速跟随 Qt 图形后端；设置中的 CPU 选项只控制 wgpu 算子，整数移动不派发计算算子。软件后端和复杂图层栈自动回退。

静态预览纹理在会话首次取帧时构建，最长边2048，之后同源复用；不是跨所有会话的永久缓存。大图预览可能较最终显示柔和，结束后普通视口帧接管。所有像素与置入图片原始模型保持；未把普通通用合成器改成 GPU 后端。

实际平台检查与限制见[验收](../validations/tools/layer-move-preview-validation.md)。
