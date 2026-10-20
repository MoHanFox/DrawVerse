# 长任务资源预算验收（2026-10-21）

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
