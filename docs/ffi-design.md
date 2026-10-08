# 第 3 阶段：C ABI 设计与验收

实现前设计，2026-10-08。正式源为 `core/crates/paint-ffi/src/api.rs`；cbindgen 固定版本生成 `ui/include/paint_api.h`。原 draft.h 留作设计历史，不可用于调用。

## 方案比较与本轮范围

| 决策 | 备选与问题 | 采用方案 |
|---|---|---|
| 句柄 | 解引用 Box 指针简单但失效/跨 core 检查会访问释放内存；编码整数为指针省内存但平台语义需额外假设 | 实际分配的小型不透明 token + 注册表；永不解引用调用者句柄；存活对象独立拥有，释放时移除对象 |
| 防止地址复用 | 仅注册表不能拒绝释放后恰好复用的地址 | token 在进程内保留，最多 65,536 个已创建句柄，达到上限返回 LIMIT_EXCEEDED；仅数 MB 元数据，文档像素正常释放 |
| 字符串 | NUL 字符串/借用 Rust 内存易产生生命周期问题 | UTF-8 + uint64_t 长度，名称/错误复制到调用者缓冲；两次查询；无 NUL 终止 |
| 像素 | 草案直接展示 RGBA8 需要显示色彩转换提前耦合 FFI | 本轮 `paint_core_read_tile` 读取线性预乘 RGBA32F；RGBA8/正式 Renderer API 后续添加，不发布假实现 |
| 生成头 | build.rs 自动覆盖已提交头可能掩盖漂移 | 独立 workspace 生成器 `paint-api-gen`；--check 比较字节，不修改已提交头；固定 cbindgen + Cargo.lock |
| 线程 | 全局锁执行绘制会让各文档互相阻塞 | 注册表短锁 + 每文档 Mutex；本轮所有同步函数仅由客户端工作线程调用，下一轮 paint-task 实施非阻塞提交 |

ABI 1.0.0，支持 64 位 Windows/macOS/Linux（含 ARM64）；应用版本仍为 0.1.0。所有公开函数使用 `extern "C"`，返回 int32_t。公开 DTO 为 repr(C)，只使用固定宽度数字、C 指针；无 Rust/Qt/STL 容器。内部不透明 marker 不具有 C 布局，生成头只给 forward declaration。

## 发布 API

- 版本与能力查询、core/document 创建和销毁、文档信息。
- 图层增删、枚举信息、名称复制、活动层、不透明度/可见性。
- 笔触 begin/to/end/cancel、undo/redo。
- 有界同步像素读取（最大 256×256），调用者缓冲与 stride，输出 revision；普通混合参考采样来自 paint-core，只读快照在文档锁外转换字节。
- 订阅/撤订阅文档变更/状态事件；消息是借用 DTO，UI 应复制进队列。状态变化包括活动层、笔触状态和历史深度，避免没有新增 dab 的 stroke_end 漏掉历史面板更新。
- 最近错误的长度查询与 UTF-8 复制。

本轮不导出草案的 load/save/task/render/GPU 函数，也不宣称任何相关 capability。下一轮新接口通过加法扩展引入；改变本轮接口布局、含义、生命周期或线程规则必须升 ABI 主版本。

## 生命周期和线程

core 有存活 doc/subscription 时销毁返回 BUSY。销毁接受 pointer-to-handle，成功置 NULL；已有 NULL 可幂等销毁（doc/subscription 仍要求有效 core）。旧 token 或其他类型 token 返回 INVALID_HANDLE；不同 core 的 doc/subscription 同样拒绝。调用者保证销毁不与正在使用该句柄的调用并发；即使注册表内部安全，也不可并发读写同一个 pointer-to-handle 输出内存。

每文档锁串行写，调用者负责跨线程 stroke 输入的因果顺序。错误 TLS 不跨线程共享。文档锁因 panic 被 poison 后不能再读写，返回 INTERNAL_ERROR，仍可以销毁；不进行盲目 poison 恢复。注册表 panic 同样失败关闭。OOM/进程 abort 不可由 catch_unwind 恢复。

回调可在任意调用线程执行，无注册表/文档/订阅 gate 锁。撤订阅先禁用，等待在途回调，然后移除；回调期间不得修改/销毁/订阅/撤订阅，相关操作返回 BUSY，避免自等待与递归事件。只读查询可重入。关闭 GUI 先在工作线程撤订阅，再停止客户端线程与销毁文档/core。回调不能跨语言抛异常或 panic；C++ 注册包装必须捕获异常并 noexcept。本轮 C++ 测试提供 noexcept 回调，实际 Qt PaintCoreClient 在 UI 模块加入。

## 指针与输出

所有参数内存须可读/写、正确对齐、初始化，且互不重叠；像素缓冲本身以字节写入无需 f32 对齐，需按原生字节序解释。库能检查 NULL/显然不对齐/已登记句柄，不能检测任意野指针、坏容量、悬空 user_data 或数据竞争。

DTO 的 struct_size 至少为本版本 sizeof，读取/写入已知前缀，不接触扩展尾部；过小/未初始化 header 拒绝。reserved 字段必须为零；未知 format/mode/tool 返回 UNSUPPORTED，非法数值返回 INVALID_ARGUMENT。输出结构须先置零并写 struct_size；像素输出还须初始化 format/data/capacity/stride。失败初始化可写输出的已知元数据，创建失败写 NULL，层 ID 失败为零；过小结构不覆盖调用者内存。

缓冲不足仅返回 required，不写部分名称或像素。像素 `required=(h-1)*stride+w*16`，不要求最后一行尾部 padding；ROI、尺寸、stride、乘加与地址长度限制在写入前核验。padding 保持不变，失败不触碰像素。空 ROI 不接受。copy 全部完成才发布 width/height/revision。

## 验收

Rust：DTO 布局快照、空/错/旧/跨 core 句柄、所有权/幂等释放、struct_size/扩展尾部、UTF-8/非法字符串、笔触与历史、RGBA32F/stride/容量/溢出/ROI、真实 capability、TLS/panic/poison、回调锁外重入与撤订阅屏障。

C/C++：从真实生成头编译 C11/C++20，明确布局 sizeof/offsetof 断言，链接 Rust staticlib 和 cdylib，各运行创建/绘制/像素读取/undo/redo/事件/释放；必须不依赖 Qt。统一 check 运行 Rust、头漂移与本地 ABI CTest。Qt Test/pytest 等各自模块实施后加入。
