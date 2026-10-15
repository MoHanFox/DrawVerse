# ABR形状抖动转换验收

2026-10-15，Windows优先，跨平台验收后置。converter6 / 来源record1 / ABR交接JSON5 / VBR1.20；复用state9/behavior6、Qt形状动态页面与真实Rust引擎，无C ABI变化。

useTipDynamics、szVr/angleDynamics/roundnessDynamics.jitter和两项最低比例已转换。开启时缺失的动态对象不添加随机量，无非零随机行为时保持旧报告/版本；完整关闭非默认参数保留，不完整关闭/原生默认关闭保持旧行为。已支持尺寸笔压独立执行，重复报告只更新同一字段；未知设备/渐隐控制继续报告未执行，工具覆盖或笔姿下不擅自转换尺寸随机。随机分布、压感后随机下限叠加以及最低圆度转换标近似；圆度由源绝对比例换成相对基础值，内核范围裁剪即使关闭也明确报告，原值仍在原始来源。

v6基准要求shapeJitter，pressure/scatter/color是独立可选对象；压力DTO剥离形状字段，纯形状配置不强加其他模块。旧五代转换入口、来源基准和原记录字节保持，VBR按最高来源需求20导出，降级拒绝。Qt交接1..5，旧交接夹带shapeJitter原子拒绝；导入基准、编辑重置、偏好保存与VBR新库资源交接使用既有统一路径。

最小相关验证：

- paint-io相关51项通过：abr_conversion24、abr_import5、abr_real_corpus1、brush_source8、vbr_source13（artifacts/abr-shape-core.log）。新增程序/孔洞图片、开启/关闭、压力/散布/两色组合真实两笔/alpha上限/历史；三个通道独立/缺省、工具覆盖/设备控制报告、非法单位类型/舍入前范围/渐隐、圆度换算/裁剪、关闭默认旧报告、六代来源最高minor/降级/原记录字节与像素。
- 公开FFI读取/准备/来源7项通过（artifacts/abr-shape-ffi.log），新增开启/关闭及独立/组合配置，明确核对纯形状基准没有被强加其他DTO；真实148支笔刷整包保持通过。
- paint-io / paint-ffi all-targets Clippy -D warnings通过（artifacts/abr-shape-clippy.log）。
- Draft202012Validator通过新Schema，六代合法来源、三个可选模块分别省略/全部省略、未来版本/降级/缺失形状/非法圆度下限（artifacts/abr-shape-schema.log）。旧Schema和原样例未改写。
- Release及生成头字节检查通过（artifacts/abr-shape-build.log）；C/C++ static/shared四项通过（artifacts/abr-shape-abi.log）。
- Windows Qt6.8.3 windows平台/software Quick后端3项功能通过，含初始化/清理5 passed、0 failed（artifacts/abr-shape-qt.txt）。形状新用例开启/关闭、完整参数与编辑重置、低压力两色绘画/alpha上限/undo/redo、旧交接拒绝且库不变、VBR带来源新目录逐像素一致及偏好重开；旧颜色动态三种配置和真实148预设整包保持通过。
- 显式外部ABR基础笔尖/压力/散布诊断1项通过（artifacts/abr-shape-external.log）；该外部样本不作为PS随机形状效果的逐像素参考，不分发其资产。

未运行无关全量和其他操作系统验收。完整ABR/16位/纯度/设备形状控制、湿混双色与橡皮、收尖/非分隔及VBR分组等继续后续，完整笔刷系统尚未完成。
