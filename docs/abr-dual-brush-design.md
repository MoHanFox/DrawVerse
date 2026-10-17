# ABR计算双笔刷导入设计

2026-10-18。ABR `dualBrush` 的计算笔刷形状现在可导入统一双笔刷系统：读取 `Brsh` 的直径、角度、圆度和硬度，`Spcn` 转为第二笔尖独立路径间距，`BlnM` 映射到乘、变暗、变亮、减去、差值或滤色贡献模式。

支持 `computedBrush` 与 `sampledBrush` 双笔刷。sampled 形状通过 ABR sample 身份关联第二图片，导入时编码为独立资源哈希并写入第二笔尖 mask；同一图片在主/第二笔尖间按身份共享。`useScatter`、`Cnt`、`bothAxes`、`scatterDynamics.jitter`、`countDynamics.jitter` 及后续散布设备控制已映射到第二笔尖的散布/数量配置。`Flip` 按 Photoshop 的语义逐印章随机翻转第二笔刷形状，形状自身的 `flipX/flipY` 作为固定镜像同时保存。

ABR 转换器提升到 11，handoff 提升到 10；源记录新增 version3，携带 `dualBrush`，converter11要求 version3；VBR source converter11要求 manifest minor41。Qt 接受 handoff v10，将 `dualBrush` 恢复到 state12 的双笔刷行；已有双笔刷 VBR/state 格式不变。
