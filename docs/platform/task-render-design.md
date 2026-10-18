# Rust 任务与 CPU 视口渲染

## 方案与边界（实现前设计）

| 方案 | 收益 | 代价 | 本轮决定 |
|---|---|---|---|
| Tokio | 异步网络/进程生态 | CPU 作业仍需专用执行器与背压 | 插件/IO 阶段再评估 |
| Rayon | CPU 工作窃取 | 文档 FIFO、显式优先级、取消需另外实现 | 后续性能数据需要时再评估 |
| 标准库有界线程池 + 串行执行器 | 无第三方运行时，顺序/取消/资源边界可测试 | 需维护调度实现 | 本轮采用 |

paint-task 提供 CancellationToken、有界优先级 CPU 池、可查询结果与独立串行执行器。任务 panic 被隔离为失败，串行状态 panic 后拒绝新命令，不能继续使用可能受损文档。退出取消未执行作业、协作取消执行中作业并 join；阻塞销毁只允许后台线程。取消不是强杀线程，笔刷单段仍受内核预算约束。

paint-render 的 Renderer trait 接收 DocumentSnapshot、RenderRequest 和取消令牌。CpuRenderer 在只读稀疏瓦片上做线性预乘正常合成，再按全局对齐的 2^LOD 方块取平均。先合成再降采样，不能分别平均图层后混合。只枚举有内容且与视口相关的瓦片；大 LOD 汇总稀疏瓦片，不扫描巨大空白画布。输出 sRGB 预乘 RGBA8 显示帧（及 Rust 线性结果测试接口），输出与工作内存有独立上限，逐瓦片/输出行检查取消。此 CPU 实现不是 GPU 或 ICC 色彩模块。

## ABI 1.1 增量扩展

保留 ABI 1.0 DTO 布局/函数/行为，minor 升至 1，添加异步 session 与 CPU 视口能力位。Session 是独立拥有的文档 actor，不与旧 PaintDocument 共享可变模型；旧同步调用继续可用。

- paint_session_create/destroy：core 拥有 session；未销毁 session 时 core_destroy=BUSY。销毁清空输入、取消任务、等待 worker 退出，因此只在后台调用。
- paint_session_submit：复制版本化命令/UTF-8 输入到最多 4096 项 FIFO，返回 sequence。接受不代表执行成功；完成前缀、最新执行错误由 session_info/error_message 查询。
- 绘画队列过载移除当前笔触连续 Move 尾部并预留 Cancel，回滚整笔；该笔后续输入明确返回 CANCELLED，直到下一次 Begin。控制命令不被越序执行。
- session_info/layer_info/layer_name：读取已发布的不可变元数据，不等文档计算锁；publication 不匹配返回 BUSY，避免图层枚举混用两个状态。
- set_viewport：4 个有界显示槽，输出至多 4,194,304 像素；输入为文档区域、输出物理像素、document_generation。更新请求号并使旧视图帧失效，不产生历史。
- frame_info/read_frame：只读最新完整帧，request_id + frame_id 精确匹配；缓冲规则沿用 PaintTile，新增 RGBA8 格式，旧 read_tile 仍只接受线性 RGBA32F。

generation 隔离新文档；request_id 隔离平移/缩放；revision 防止显示倒退。同一视口允许显示正在绘画的有序较早帧以避免持续输入造成渲染饥饿；撤销/取消/图层变化设置 revision 下限，拒绝操作前的迟到帧。每槽只保留最新帧和一个作业；新视口取消旧作业，导航作业优先级低于主画布。

## UI 接入

PaintCoreClient 在 GUI 线程只复制小命令/视口参数到非计算队列；Qt worker 负责 session 生命周期、读取元数据和帧字节。不再用 C++ 绘画命令队列、CPU 合成或整张文档显示缓存。CanvasItem 请求可见文档区域与对应物理像素，导航另用完整文档的有界缩略图；跨 DPI/窗口映射仍用 Qt 逻辑坐标。新建放开到内核的 1,000,000 边长上限，显示内存取决于视口。

## 验收

Rust：优先级/FIFO、队列上限、排队/运行取消、panic 后可控退出、关闭；CPU 正常合成与 reference 对照、可见性/透明/层不透明度、LOD 先合成平均、巨大稀疏画布、资源拒绝、取消、快照隔离；异步 ABI 顺序、图层一致枚举、代际/视口旧帧拒绝、过载回滚、错误消息、句柄生命周期、像素缓冲与旧 ABI 回归。

Qt Test：保留现有用例，新增超大画布内存/视口移动、导航、绘画/撤销迟到帧、异步关闭与 DPI；从真实应用输出截图。统一 python tools/check.py 不跳过失败项。真实硬件、GPU、ICC、文件 IO 和 Python 插件仍按后续模块验收。
