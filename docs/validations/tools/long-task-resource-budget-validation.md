# 长任务资源预算验收（2026-10-21）

## 共享结果历史扩展

已将变换/整图翻转/栅格化的LayerEdit及桶Stroke接入同一分页结果引用；选区计账包含真实遮罩内存。核心结果六项、历史五项、直接跳转三项、进度一项、公开变换七项、原生三项与两后端Qt各九行通过，四ABI/严格Clippy/格式/生成头/Release通过。75MiB固定热样本物化275.930→0.129ms，仅核心物化，非端到端。完整证据及冷页失败/置入源/选区/预算边界见[历史存储验收](../document/history-storage-validation.md#2026-10-21长任务共享结果历史扩展)。ABI1.75/state45保持；会话字段与其他命令、完整8K/16K及实体持续时延仍待。

对应 [设计](../../../tools/long-task-resource-budget-design.md)。

## 本轮完成

- `paint-ops::batching::plan_batch`：批次折半，顺序/覆盖保持，单元仍超限返回流式后备标记；空输入不调用规划器。
- `RemapOp::try_pixel_with`：按需源读取与既有 CPU 算子共享双线性/三次采样、原像素/选区混合、预览步长公式。
- 液化：带符号极值/原像素支撑范围、自适应暂存批次；单瓦片超过 2048 源瓦片时使用最多 64 个源瓦片读取 pin 的 CPU 后备；缓存逐个淘汰，避免清空重载。最终和软件预览输出使用文档逻辑瓦片配额，取消独立的 8192 输出瓦片限制。

## 验证

- `cargo test -p paint-ops --locked`：9 单元、5 operators、10 SDF 通过。
- 核心 adaptive_tests 3 项通过：既有网格细化与画布外历史；2304 个逻辑源瓦片触发真实 2048 暂存门槛，单瓦片后备与独立 CPU 双线性/三次/选区/步长采样完全相同，真实 remap 输出一致；较大普通批次拆小；8K/16K 分别规划 16384/65536 个输出瓦片，文档配额不足仍明确失败。
- 核心公开 liquify：11 通过，1 手动性能项未运行；既有像素摘要、画布外恢复、取消、历史、低资源失败保持。
- 原生 `liquify_abi_validation_preview_cancel_and_single_history`：1 项通过。
- 软件 Qt `maximumLiquifySizePublishesPixelsAndKeepsLocalAndDocumentHistory` 通过；D3D11 同项与常驻网格预览 2 项通过，含 15000px 笔径。约 50ms 的回执测量包含测试轮询，不等同真实输入时延。
- 四 ABI 顺序通过；严格 Clippy 与格式通过；完整 Windows Release 成功，输出 `build/qt/bin/Release/drawverse.exe`，ABI 1.74/笔刷 state44/格式保持。

## 边界与未完成

8K/16K 本轮验证的是范围规划，不是整图物化耗时或实体设备连续交互。2304 源瓦片回归共享测试像素，检验暂存与采样正确性，不是实际 2304 块独立纹理的峰值内存基准。文档配额、驻留页预算、磁盘容量、位移场/输入工作量等现有保护仍存在；不能称所有大图路径都已流式化。

未完成：笔刷切换卡顿根因和修复；热历史保留/冷历史后台恢复的进一步优化；其他长任务对本轮规划器的迁移；任意 OS OOM、全量、跨平台和 16K 实体持续性能。调查失败的试验不计入验收。

## 2026-10-21 已完成液化的分页结果历史

- 大结果通过 StoredCommand::prepare_result_in 保留受当前 PagePool 管理的 Arc 瓦片。引用元数据和历史步数有预算，像素没有额外 pin，仍受文档驻留/暂存限制。其他命令保留原编码行为。
- 新核心三项：真实液化在低历史预算下生成 Paged 记录，反复直接历史跳转/撤销/重做保留原 before/after 身份与 redo 分支；驻留仅 1 瓦片时冷读取和结果保持、驻留回到预算；损坏冷页后原子保留瓦片映射、修订、dirty 与历史栈。
- 新存储两项：未托管/其他文档瓦片或元数据超预算回退编码，写失败明确报错；元数据预算淘汰最旧记录并释放身份。原液化编码写失败用例改验“不需要第二份编码文件仍可成功”，取消及最终像素精确 undo/redo 保持。
- 最终 Release 定向回归：液化结果 4 项、历史存储单元 10 项、公开液化 11 项、公开 history_storage 3 项及 paging 3 项通过。公开液化的既有手动性能项未运行。
- 另显式运行手工 Release 比较：600 块变化、before+after 共 75MiB 逻辑像素、64MiB 历史预算、4096 驻留瓦片。旧编码准备 358.227ms，引用准备 0.065ms；各五次物化中位数 581.150ms 与 0.113ms。验证引用身份保持。此为热数据核心物化，不包含 UI 渲染、完整 undo/redo 或磁盘冷页时延，不是 PS 性能数字。
- 原生 liquify_abi_validation_preview_cancel_and_single_history 一项通过。
- 最终 Qt：软件液化发布/局部与全局历史 1 项、直接历史选择 1 项通过；D3D11 同两项及常驻网格显示 1 项通过（含 15000px 笔径）。这些共享界面用例不是 75MiB Paged 历史 UI 时延基准。四 ABI 顺序通过；完整 Windows Release/生成头通过，产物 build/qt/bin/Release/drawverse.exe，ABI1.74/state44/VBR 不变。
- 格式及生产 paint-ffi lib 严格 Clippy 通过。扩大到 paint-core --tests 的严格 Clippy 被未修改的四个既有问题阻断：transform_preview 两个 unnecessary_cast、history_travel 的 clone_on_copy、liquify_operators 的 items_after_test_module。初次宽范围 debug 单元测试在旧液化编码断言失败后停止；更新为分页结果契约后定向项通过，未完成全量套件验收。

尚未覆盖：完整 8K/16K 多层物化和历史、任意 OS OOM、实体连续输入、跨平台；液化会话字段历史及其他长任务结果类型未迁移。暂存不足自动释放旧历史后重试仍待，实际物理存储仍可能成为限制。


## 2026-10-21 严格检查阻塞修复

已修复上述四处既有Clippy阻塞：变换预览测试只移除实际i32坐标的冗余转换（另一处u32转i32保留）；DocumentOptions按Copy直接传值；将液化adaptive_tests移至文件末尾。核对液化生产代码和测试模块内容均保持，仅调整模块位置，不新增allow或关闭检查。

cargo fmt --all -- --check 与 cargo clippy --workspace --all-targets --locked -- -D warnings 均通过，扩大检查覆盖测试、示例等全部目标。实际Rust模块路径的定向回归：document::transform::preview 9项、document::history_travel 3项、document::liquify::operators::adaptive_tests 3项通过，无忽略。最初按文件名过滤的两条命令命中0项，不计验收；修正为模块路径后重跑。

本项仅调整测试，运行时算法/ABI1.74/state44/格式保持，不重复四ABI或重建Release；本轮前两项Qt修复已完成Windows Release、两后端各九项回归，产物 build/qt/bin/Release/drawverse.exe。未宣称全量测试运行通过，8K/16K仍仅包含规划回归，首次加载与其他长任务迁移等剩余边界不变。
