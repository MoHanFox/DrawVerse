# 形状抖动状态与VBR

state9 / VBR1.7 / behavior6 / shape-jitter。新增独立shapeJitter对象：enabled严格布尔，size/minimumSize/angle/roundness比例0–1，minimumRoundness比例0.01–1，seed为固定16位小写十六进制字符串。完整u64不能经过JSON双精度数字。所有字段必需，未知字段、null、类型/范围/版本/能力不匹配拒绝。关闭保留非默认配置，全部默认可省略，不升级旧文件。

压力、设备姿态、散布及形状抖动分别序列化，准备时组合为完整执行配置。旧pressure对象继续拒绝非压力模块。behavior与能力依据对象存在声明，包含关闭模块。程序和图片、完整ABR来源资源并集保持；converter1基线冻结，拒绝shapeJitter对象。

本阶段完成Rust IO与执行还原，ABI仍1.39。Qt当前仅接受state8/behavior5，原生package入口暂拒绝shapeJitter对象，读required=0且caller与资源目录不变，写不改目标。下一阶段原生能力与Qt接入后解除此边界。

验收：完整种子/关闭值/state9和VBR1.7还原后连续两笔真实像素一致，压力/设备/散布组合，图片和ABR原字节保持；坏对象/版本/能力与来源基线拒绝；旧默认状态和资源包不升级，旧入口不丢字段、不污染目标。运行时随机笔触序号不入交换格式，不承诺工程重开接续旧随机序列。此阶段不是完整系统验收。
