# ABR颜色动态转换

2026-10-15。将useColorDynamics、clVr.jitter、H   /Strt/Brgh百分比、colorDynamicsPerTip接入已有ColorDynamics/linear-hsv-v1。前景/背景是起笔捕获的工具两色，预设只保存混合幅度；0..100严格类型/单位和舍入前范围。随机序列、线性RGB/HSV与PS颜色空间未逐像素核对，非零颜色变换明确Approximate。clVr设备/渐隐控制、purity纯度尚未执行，逐字段保留报告；不通过改变颜色空间伪装PS完整等效。

逐笔/逐实际印章使用已有原生频率。开启且perTip缺失按逐笔解释并报告缺省近似；完整关闭配置需要明确所有已映射字段（含频率）才进入原生状态，否则保留来源报告。关闭原生默认不升级旧数据；purity不会映射为其他颜色幅度，仅有纯度时不会伪造着色效果。完整关闭非默认配置保存/重置/交换。

新增颜色配置使用converter5，原record1；保留显式v4转换，旧四代基准/报告/记录不重算。来源v5要求colorDynamics，可组合独立pressure/scatter DTO，其余动态继续拒绝；防止颜色数据混入pressure DTO或给新颜色独占预设强加scatter对象。ABR交接JSON4防旧Qt丢字段，Qt接受1..4复制完整已有state10配置与resetState。VBR1.19容纳v5来源，最高需求/降级拒绝/Schema和样例同步；不引入新C ABI、state或执行行为版本。

Windows最小相关验收：关闭/逐笔/逐印章、两色与HSV、程序/图片、压力/散布真实组合两笔与alpha上限/历史；非法范围/单位/类型、控制未执行报告、关闭默认和旧四代报告不变；来源完整基准、五代混合/最高minor/降级/原字节及新库像素。原生ABR读取/准备、Qt导入/编辑/重置/偏好重开/新目录VBR两色绘画，Release与ABI。跨平台后置；纯度/设备控制、完整ABR/16位、湿混双色/橡皮、收尖等继续后续。

字段参考[ag-psd abr.ts](https://github.com/Agamnentzar/ag-psd/blob/master/src/abr.ts)，仅字段布局依据，不是PS渲染一致性证明。
