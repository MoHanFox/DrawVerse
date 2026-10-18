# 曲线节点的直线与平滑模式

2026-10-18，补齐[反馈顺序](../../planning/brush-feedback-execution-design.md)步骤3b。

每个节点可保存一个nodeModes布尔值（true平滑、false直线），与曲线点一一对应，最多17项。无该字段的旧线性/PCHIP曲线保持原语义和输出字节；编辑新节点默认平滑。Ctrl点击只切换所点节点，不移动坐标或更改最低比例。

平滑节点使用既有PCHIP切线；直线节点在左右两段分别采用对应割线斜率，形成折角。两端都为直线的区间直接用原线性路径，两端都为平滑的区间沿用原PCHIP；混合区间使用同一个Hermite公式。平滑节点仍保持两侧导数连续，输出仍限制在相邻节点的值域。全平滑/全直线必须分别与旧模型逐样本一致。

nodeModes仅允许搭配显式平滑插值标记（普通动态smooth-pchip-v1、起收笔pchip）。state40、VBR1.0/behavior32/curve-node-modes根据实际字段保存，包括关闭控制；拒绝null、数量不符、非bool和state降级。ABI增加节点采样能力，编辑器图形和实际核心采用同一求值器。原有两个采样模型不变。

实施前验收：所有节点组合的有界性、全平滑/全直线旧结果一致、内部平滑导数与真实笔触差异；主副压力、设备动态与起收笔的state/VBR关闭值往返及非法数据拒绝；公开采样和Qt Ctrl点击/拖动/新增删除/最小值/保存重开、新目录真实像素一致。结果写入[分步记录](../../validations/planning/brush-feedback-execution-validation.md)。
