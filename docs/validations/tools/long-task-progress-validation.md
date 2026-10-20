# 长任务真实进度验收（2026-10-20）

对应[设计](../../tools/long-task-progress-design.md)。范围为液化完成、自由变换完成、整图水平/垂直翻转。ABI1.74 / bit62，state44、behavior36、VBR1.0与历史磁盘格式保持。

- 核心 `task_accounting_keeps_pixels_history_and_resource_failure_atomic` 1项通过，覆盖三类任务有/无进度记录的真实像素与历史对照、undo/redo，以及低瓦片预算下 ResourceLimit 原像素/历史/修订和编辑会话保持。
- 原生 `live_task_progress_is_readable_before_publication_and_retains_completion` 1项通过：计算尚未完成时另一线程取得真实中间瓦片计数，阶段内单调/不超总数，结束时间冻结；失败状态、无变换会话、指针/短输出/无效句柄、72字节布局与能力检查通过。actor已有 resident_preview_keeps_render_floor_but_controls_advance_it 1项通过。
- 软件与D3D11下各2项真实主窗口Qt测试通过：4096×2160填充后翻转，10000ms延迟不显示、0ms能观察运行状态与实际中间计数、GUI计时器继续运行、完成隐藏；设置界面修改/双文档即时同步/重启保存，浮动文档实际变换和液化确认使用自身进度，其他文档不显示。
- 设置值写入改为GUI线程串行处理，避免不同文档worker的旧写入覆盖新值；仅设置修改时同步小偏好，不在绘画或计算循环写盘。
- 四C/C++静态/动态ABI消费者顺序通过，严格Clippy、Rust格式、生成头与Release构建通过。输出 `build/qt/bin/Release/drawverse.exe`。

百分比是当前阶段，准备和写撤销记录等总量未知阶段显示不定进度；阶段切换可重新计数。使用既有64×64候选瓦片、分页缓存与GPU批次，不将整幅中间结果强制放入内存。低预算测试不等同于任意操作系统OOM、磁盘断开或自适应重试验收；没有新增取消长任务入口，也没有跑全量、跨平台或实体4K长时间连续性能。
