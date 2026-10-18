# 双笔刷覆盖模式扩展

2026-10-18。ABR 双笔刷的 `BlnM` 除原有六种模式外，新增颜色加深、线性加深、颜色减淡、线性减淡、叠加、柔光、强光、排除、实色混合、鲜明光、线性光、点光和划分，共 19 种灰度覆盖组合。

核心 `DualMode::combine` 直接复用纹理覆盖模式的精确公式，保留原有 `Subtract` 的 `primary × (1-secondary)` 语义，以及主覆盖为零时不贡献第二笔尖的裁切规则。ABR 映射同时接受短码（`CBrn`、`CDdg`、`Ovrl`、`SftL`、`HrdL`、`Xclu`、`HrdM`、`Vvlt`、`LinL`、`PntL`、`Dvde` 等）和名称形式；未知模式仍明确报告未执行。

新模式要求 state36、VBR1.43、behavior28 和能力 `dual-cover-extended`。旧 state/VBR 遇到新模式明确拒绝，旧六种模式字节与版本保持。Qt 双笔刷模式下拉显示新增模式；ABR handoff 继续使用现有 version11，状态写入新版本。颜色模式 Hue/Saturation/Color/Luminosity 仍不冒充灰度覆盖模式，保留为后续需要颜色规范和样本的项目。

验收：核心 19 模式与纹理公式逐值对照；真实像素/历史；state36/VBR1.43 的 Overlay 往返与旧版本拒绝；ABR `Ovrl` 导入；Qt 状态、行为和能力导出。
