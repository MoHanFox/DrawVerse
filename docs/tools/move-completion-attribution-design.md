# 主线 C2-A — Move Completion Attribution

依赖 C1 审计提交 `22a4183c8317ad0b9e636c56e4ec1fbd57cec2df`；产品基线仍为 `b948dccbc2fd767af3a999ee9a03e108455c44cb`。本轮只增加默认关闭的任务级观测，不改变算法、调度、取消、质量或 ABI。

## 观测和验收契约

Rust 内部记录器以环境变量 `DRAWVERSE_MOVE_COMPLETION_TRACE` 指定新输出文件。单进程最多 50000 条，记录本地 Instant 时间、线程、session、sequence、task、view、generation、request、revision、frame。结束并停止所有核心 worker 后冻结、一次导出；拒绝覆盖旧文件，溢出明确报告。默认关闭不逐次分配、加锁或写盘。

先观察命令入队/接纳、快照准备、任务入队/开始/执行结束、取消/丢弃、帧发布及 ABI 读取。以准确帧身份连接 Qt worker/GUI 的接收，不使用最近回调。Rust Instant 与 C++ steady_clock 没有共同绝对起点：分别报告阶段持续时间；跨语言未观测间隔只作总区间残差，不宣称已精确分配给线程。

仅当首轮证据表明执行占主要成本时，补少数 renderer 粗粒度阶段与已有缓存统计，不在像素循环逐样本埋点。

S2/S3 各三次独立原生可见 Release 进程，保留立即/后续两段；S1 一个轻量参照，一个代表场景诊断关闭对照。保留取消、错误、不完整记录，不替换失败。复用像素、终值、单历史、撤销重做和取消断言。最多提出一项后续修复，本轮不实施。

## 探针后的有限细分

首轮 S2 任务探针的两个正常视口任务执行约385/391ms，排队0.01–0.02ms，成功收尾约409/416ms，支持继续细分执行。增加 `render_diagnosed`：与正式 Renderer 共用同一私有 render 函数，只在显式诊断调用测缓存锁等待、准备、整体合成/LOD/blit；缓存命中/未命中取现有计数差。PagePool 读写增量是共享池在该任务区间的计数，可能含并发任务，不能作为该任务独占 I/O 时间。没有逐瓦片计时或改变分支、参数、质量。

跨语言时间线可选有界对时：同一sequence的Rust command.submit发生在C++ move.submit同步scope内；同一五元组的Rust frame.copied发生在C++ copyFrame scope内。两者各给出offset区间，求交并报告区间宽度；若缺少精确身份或交集为空则标未测/不一致。假定单次收尾区间内两种单调时钟速率稳定，第二个独立包含关系用于一致性核对，不宣称绝对时钟同源或物理显示延迟。Rust阶段持续时间不依赖该对时。

记录器是单次进程采集，首次核心销毁后冻结导出；不支持同进程重新start，多核心场景在首次销毁时结束全局采集。本轮宿主只有一个PaintCore；session身份仍区别该Core的文档/临时会话。任务ID为诊断关联ID，同时记录真实TaskTicket ID，未改变任务队列。
