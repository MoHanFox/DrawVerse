# ABR计算双笔刷导入设计

2026-10-18。ABR `dualBrush` 的计算笔刷形状现在可导入统一双笔刷系统：读取 `Brsh` 的直径、角度、圆度和硬度，`Spcn` 转为第二笔尖独立路径间距，`BlnM` 映射到乘、变暗、变亮、减去、差值或滤色贡献模式。

仅支持 `computedBrush` 双笔刷。`sampledBrush` 双笔刷需要第二图片资源的关联与来源记录，本阶段明确报告 NotExecuted 并保持原字段。`useScatter`、`Cnt`、`bothAxes`、`scatterDynamics.jitter` 和 `countDynamics.jitter` 已映射到第二笔尖的散布/数量配置；`Flip` 和散布设备控制仍保留并报告未执行。

ABR 转换器提升到 11，handoff 提升到 10；源记录新增 version3，携带 `dualBrush`，converter11要求 version3；VBR source converter11要求 manifest minor41。Qt 接受 handoff v10，将 `dualBrush` 恢复到 state12 的双笔刷行；已有双笔刷 VBR/state 格式不变。
