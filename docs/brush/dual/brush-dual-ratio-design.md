# 双笔刷百分比尺寸

2026-10-18，对应[反馈顺序](../../planning/brush-feedback-execution-design.md)步骤7。

增加独立模型`relative-ratio-v1`，包含mode(independent/relative)、diameterRatio（0.0005–2000倍，界面0.05–200000%）、widthRule。相对名义半径为主半径×比例，钳制0.5–1000。follow-pressure沿用第二大小动态；fixed-width使用主实际半径×比例并忽略第二大小动态，与旧像素偏移规则保持独立。独立模式仍保留关闭的比例。

旧`relative-size-v1`和diameterOffset继续读取/执行/写回，不能把像素偏移重新解释成百分比。UI切换为新比例模式时按当前实际名义尺寸换算，旧模型额外保留像素偏移选项；不在无编辑时迁移源文件。

新比例写state38、VBR1.0/behavior30及dual-size-ratio能力，关闭值亦保留。ABI布局不变，准确准备入口按实际字段读取。ABR按读取并钳制后的主副直径计算比例，保持初次导入名义尺寸；原来源基线保留旧语义。

实施前验收：多比例/压感/双路径与显式独立半径像素相同、旧偏移回归；state/VBR关闭与开启往返、降级与非法组合拒绝；Qt比例显隐、真实预览/绘画/新目录交换/重开；ABR比例与实际基线、来源恢复回归。结果写入[分步记录](../../validations/planning/brush-feedback-execution-validation.md)。

ABR新增转换器14/交接13：按执行直径自动生成比例，完整来源仍保存原ABR与导入基线。来源校验要求比例与转换器14一致，阻止降级丢值；Qt交接低于13拒绝比例对象，重置采用原导入比例。传递抖动可与比例共同保留。
