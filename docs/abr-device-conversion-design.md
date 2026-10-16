# ABR设备形状转换

2026-10-16。复用现有TipDynamics执行路径：angleDynamics.bVTy的6/8/3分别映射方向、笔旋转、倾斜角度，roundnessDynamics.bVTy=3映射倾斜圆度。字段编号依据[ag-psd abr.ts](https://github.com/Agamnentzar/ag-psd/blob/master/src/abr.ts)；不是Photoshop笔触等效证明。使用原生设备能力回退、基础角度叠加、余弦倾斜圆度与随机叠加，实际映射标Approximate。minimumRoundness绝对百分比除基础圆度后裁剪至原生1%–100%；原值仍保留。各设备控制独立执行，未知控制/渐隐/初始方向或旋转继续逐字段未执行。

useTipDynamics必须为true。现有TipDynamics没有关闭位，关闭的设备控制保留原始来源和未执行报告，不把关闭配置变成活动设备控制。useBrushPose或对应toolOptions覆盖存在时不猜测设备输入；其他已支持压力、随机、颜色和散布保持独立。jitter/Mnm/fStp沿用严格单位、范围与类型校验；本阶段不把Mnm当作设备圆度下限，使用明确的minimumRoundness。

converter7保留显式v6转换及旧六代不可变来源/报告。来源record1要求tipDynamics，其他动态可选；DTO不得夹带其他通道。交接JSON6接既有state7设备对象及重置基准，VBR1.21承载新来源并拒绝降级夹带；旧Schema保持，新Schema与样例覆盖新来源。无C ABI、state或行为版本变化。

验收条件：支持的控制与缺失设备能力、关闭/未知/笔姿和工具覆盖、非法范围类型单位、独立随机/压力组合；程序与图片真实设备两笔像素及历史；旧六代记录保持、七代最高版本与降级拒绝；公开原生准备、Qt导入/编辑重置/偏好恢复/新目录VBR实际像素。Windows最小相关测试、Release/生成头及ABI检查。跨平台后置；完整ABR/16位、湿混双色/橡皮、收尖/非分隔等仍待后续。
