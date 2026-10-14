# 颜色动态保存与交换

state10 / VBR1.8 / behavior7 / color-dynamics。新增独立colorDynamics对象：model固定linear-hsv-v1、enabled/perTip严格布尔，foregroundBackground/hue/saturation/value比例0–1，seed固定16位小写十六进制字符串。f64输入检查原范围后转f32，未知模型/字段、null、坏类型/范围/版本/能力拒绝。关闭保留非默认值；全部默认省略，不升级旧文件。

模型明确使用线性RGB混合与线性RGB的HSV，前景alpha保持；不声称PS感知HSV兼容。前景与背景属于工具颜色，不进入预设/VBR。压力/设备/散布/形状/颜色分别保存，恢复后组成完整配置。converter1来源基线保持冻结，拒绝新动态对象；图片与完整原ABR字节保持。

本阶段只接Rust IO，ABI仍1.40。Qt仅接受state9/behavior6，库package入口临时拒绝colorDynamics对象：读required=0且caller/资源目录不变，写不改目标。原生背景传入与Qt下一阶段接入后解除边界。

验收：完整种子/关闭值/整笔与逐印章频率、同工具颜色下state10和VBR1.8恢复的真实连续两笔逐像素一致，全部模块共存与单独颜色配置、旧状态原字节保持；所有坏值与轻微越界、版本/模型/能力拒绝；图片和原始来源保留、旧入口不丢字段及不污染目标。运行时随机序号不入格式，重开不承诺接续旧随机序列。完整系统其余范围继续后续。
