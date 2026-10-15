# ABR形状抖动转换

2026-10-15。将useTipDynamics、szVr/angleDynamics/roundnessDynamics.jitter、minimumDiameter、minimumRoundness接入现有ShapeJitter，不另建PS执行器。采用原生seed0和逐实际印章随机序列，尺寸/圆度缩小与±180°角度抖动明确Approximate：PS随机分布及压感叠加尚未逐像素核对。minimumRoundness从源绝对圆度转换为相对基础圆度，低于内核1%下限时有界裁剪并报告近似。使用原生压力尺寸后再施加随机缩小，不能声称与PS同一最低直径公式。

开启配置允许缺省某个动态对象，缺省幅度为零；只有实际非零抖动生成新形状配置。完整关闭配置需三个动态对象和两项最低比例齐全，否则保留原始字段及未执行报告。完整关闭非默认配置保留，默认关闭不升级。bVTy=0为无设备调制，原生已执行的尺寸笔压独立保持；角度/圆度设备和渐隐控制继续未执行。工具覆盖或笔姿导致尺寸随机源不明确时，尺寸随机量不执行且报告，不通过丢失已支持的压力行为伪装兼容。

converter6保留显式v5转换；旧五代来源/报告/基准不重算。新来源baseline要求shapeJitter，独立可选pressure/scatter/color，DTO不得相互夹带。交接JSON5由Qt复制既有state9形状对象及完整resetState；VBR1.20承载新来源，旧版本夹带明确拒绝，Schema/样例同步。无新C ABI、state或行为版本。

Windows最小验收：程序/图片、三种随机量和最低比例、已支持压力/散布/颜色组合真实两笔与历史/alpha；关闭/缺省/非法范围单位类型/未知控制报告，旧五代来源保持；六代混合最高minor与降级拒绝/原记录字节/新库像素；公开FFI准备、Qt导入编辑重置偏好重开与新目录VBR、Release/生成头/C ABI。跨平台后置；完整ABR/16位、纯度/设备控制、湿混双色/橡皮、收尖/非分隔等仍待后续。

字段布局依据[ag-psd abr.ts](https://github.com/Agamnentzar/ag-psd/blob/master/src/abr.ts)，非PS笔触等效证明。
