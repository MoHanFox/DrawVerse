# ABR颜色动态转换验收

2026-10-15，Windows优先，跨平台后置。converter5 / 来源record1 / ABR交接JSON4 / VBR1.19。复用既有ColorDynamics、state10/behavior7、Qt颜色动态页面与完整重置基准，无C ABI变化。

useColorDynamics、clVr.jitter、H   /Strt/Brgh和colorDynamicsPerTip接入原生前景背景混色、HSV幅度和逐笔/逐实际印章频率。百分比0..100在转f32前严格检查，完整关闭非默认配置保留，完整关闭默认和不完整关闭配置保持v4报告和版本。前景/背景来自起笔工具颜色而非ABR内固定颜色。开启且频率缺省采用逐笔并标近似。非零随机/颜色变化明确近似，不声称PS颜色空间、随机序列或整笔视觉完全一致；纯度/设备及渐隐颜色控制仍逐字段报告未执行。

v5来源要求colorDynamics，pressure与scatter为独立可选对象；纯颜色不强加scatter，压力序列化剥离颜色。旧四代转换入口、来源基准、原字节和报告保留。VBR按最高来源需求选择19，18以下夹带v5拒绝。Qt仅在交接JSON4接收颜色，旧交接夹带拒绝且库不变；现有重置、偏好保存和资源交接完整保留颜色配置。

最小相关验证：

- paint-io共46项通过：abr_conversion20、abr_import5、abr_real_corpus1、brush_source8、vbr_source12（artifacts/abr-color-core.log）。新增关闭/逐笔/逐印章及程序/图片、压力散布组合真实两笔、工具两色、历史；范围/单位/类型与设备控制报告、缺省频率、关闭默认报告、来源Schema样例、五代来源混合/降级/原记录字节和像素。
- 公开FFI读取/准备/来源6项通过（artifacts/abr-color-ffi.log），包含独立颜色与压力/散布组合、开启/关闭以及真实148预设整包。当前状态与原生准确准备结果一致，原ABR删除后来源仍可读。
- paint-io / paint-ffi all-targets Clippy -D warnings通过（artifacts/abr-color-clippy.log）。
- Draft202012Validator新Schema和旧/新来源、独立压力散布省略、非法未来版本/降级/缺失颜色/越界检查通过（artifacts/abr-color-schema.log）；旧Schema和旧fixture未改写。
- Release及生成头字节检查通过（artifacts/abr-color-build.log）；C/C++ static/shared四项通过（artifacts/abr-color-abi.log）。
- Windows Qt6.8.3 windows平台/software Quick后端三项功能用例通过，含初始化/清理5 passed、0 failed（artifacts/abr-color-qt.txt）。新颜色用例覆盖关闭/逐笔/逐印章三种配置、真实工具两色/低压力整笔alpha上限与undo/redo、编辑重置、旧JSON拒绝、带来源VBR新目录逐像素一致及偏好重开；旧散布及真实148预设整包保持通过。
- 显式外部ABR基础笔尖/压力/散布诊断1项通过（artifacts/abr-color-external.log）。外部样本不是新增颜色视觉参考，不将其基础PNG当作PS颜色动态一致性证明。

未运行无关全量或其他操作系统验收。当前Windows阶段已通过；完整ABR/16位/设备控制/纯度、湿混双色与橡皮、收尖/非分隔和VBR分组等仍待后续，完整笔刷目标保持进行中。
