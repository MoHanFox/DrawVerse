# 起收笔相对长度与端部透明

2026-10-18，对应[反馈顺序](../../planning/brush-feedback-execution-design.md)步骤8。

每端保留独立像素长度，并可附带成对的lengthMode（independent/relative）与lengthPercent（0–10000）。缺少这对字段时保持旧像素行为；独立模式也保留百分比，切换不覆盖另一组值。相对长度为起笔时捕获的实际主笔刷直径×百分比/100，单端执行长度上限仍为4096px，尾部记录和输入预算不扩大。尺寸动态不改变已捕获的距离。

核心编译和尾部缓冲显式接收直径，防止调用者遗漏相对换算。state39保存新增字段，旧state降级拒绝；VBR维持1.0，behavior31和taper-relative-length由实际字段决定，关闭值也保留。ABI布局不变。

界面只显示当前模式的长度输入，统一保留两位小数；“端部透明”复用该端不透明度控制的minimum，关闭设为1、开启设为0，曲线保留。尺寸曲线独立生效。

实施前验收：相对/显式像素的实际笔触一致，改变直径后按比例变化，像素模式不变；边界和透明开关；state/VBR开关值往返与非法/降级拒绝；Qt显隐、真实预览与绘画、新目录交换和重开。结果记入[分步记录](../../validations/planning/brush-feedback-execution-validation.md)。
