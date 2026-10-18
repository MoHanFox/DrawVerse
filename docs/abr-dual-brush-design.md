# ABR计算双笔刷导入设计

2026-10-18。ABR `dualBrush` 的计算笔刷形状现在可导入统一双笔刷系统：读取 `Brsh` 的直径、角度、圆度和硬度，`Spcn` 转为第二笔尖独立路径间距，`BlnM` 映射到乘、变暗、变亮、减去、差值或滤色等灰度覆盖模式；后续阶段已将覆盖组合扩展到19种。

支持 `computedBrush` 与 `sampledBrush` 双笔刷。sampled 形状通过 ABR sample 身份关联第二图片，导入时编码为独立资源哈希并写入第二笔尖 mask；同一图片在主/第二笔尖间按身份共享。`useScatter`、`Cnt`、`bothAxes`、`scatterDynamics.jitter`、`countDynamics.jitter` 及后续散布设备控制已映射到第二笔尖的散布/数量配置。`BlnM` 现支持19种灰度覆盖模式（含叠加、柔光、颜色加深/减淡、划分等）；`Flip` 按 Photoshop 的语义逐印章随机翻转第二笔刷形状，形状自身的 `flipX/flipY` 作为固定镜像同时保存。

基础双笔刷使用 converter11/handoff10、源记录 version3、VBR1.41/state12；扩展覆盖模式使用 converter12/handoff11、VBR1.43/state36和`dual-cover-extended`能力。Qt 接受 handoff v10/v11，并将 `dualBrush` 恢复到对应双笔刷行；旧六种模式字节保持。
