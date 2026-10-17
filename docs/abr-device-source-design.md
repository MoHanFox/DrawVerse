# ABR设备来源映射设计

2026-10-18。ABR `brushDynamics.bVTy` 的已知设备控制类型现在可以进入统一动态系统：`2` 保持笔压，`3` 映射倾斜幅度，`6` 映射笔触方向，`8` 映射笔旋转。尺寸、流量和整笔不透明度分别根据各自的 `szVr`、`prVr`、`opVr` 对象选择来源。

## 执行语义

ABR 控制类型为笔压时，沿用原有 `pressure_size`/`pressure_flow`/`pressure_opacity` 路径。控制类型为倾斜、方向或旋转时，清除对应通道的笔压标记，写入 `TiltDynamics`、`DirectionDynamics` 或 `RotationDynamics` 的线性响应，并把来源矩阵的对应目标设为该来源。多个通道可以选择不同来源；同一来源的多个通道共享一个动态对象。

`Mnm` 继续作为最低比例，`minimumDiameter` 仍优先作为尺寸最低比例。所选来源的曲线在通用动态中为恒等曲线时，矩阵执行使用该来源的归一化输入，因此 ABR 的线性设备响应不会被误判为“无效果”；未启用的来源仍为 1。矩阵默认 `All` 的旧相乘语义不变。

## 报告与版本

`bVTy`、`Mnm`、开启标志和已映射的最低值报告为 Mapped；非零 `jitter`、`fStp` 渐隐步数、未知控制类型和工具覆盖继续保留为未执行或原字段报告。转换器版本提升到 9，ABR handoff 版本为 8。源记录允许 converter 1..9；converter 9 的基线必须带 `sourceMatrix` 和至少一个 tilt/rotation/direction 对象，低版本携带这些对象明确拒绝。VBR source converter 9 要求 manifest minor38。

Qt 导入接受 handoff v8，并恢复 `tiltDynamics`、`rotationDynamics`、`directionDynamics` 和 `sourceMatrix`；保存/重开继续使用 state32。ABR 的未知控制类型、渐隐、随机抖动、效率像素等价和 16 位笔尖仍不在本阶段范围。
