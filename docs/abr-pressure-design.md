# ABR 笔压动态导入

2026-10-15。沿用已有统一BrushDynamics，接通ABR useTipDynamics/szVr 与 usePaintDynamics/prVr/opVr 的笔压控制（bVTy=2），线性响应及最低比例。size优先使用独立minimumDiameter，否则使用szVr.Mnm；不同的szVr.Mnm作为未执行字段报告，不冒称二者等价。仅在模块开启且无该通道工具覆盖时映射；渐隐/倾斜等其他控制、随机抖动和工具覆盖继续逐字段保留与报告。非零抖动不妨碍已支持的笔压分量执行，但仍明确部分支持。已知启用的笔压对象严格检查类型、单位与舍入前范围，错误不能静默落到默认。

仅执行新笔压行为的记录用drawverse.abr.basic converterVersion3，来源记录结构仍version1。旧转换v1/v2入口与来源字节、报告、基准不重算。v3只放宽压力基准，不允许其他动态被伪装为已转换。

ABR交接JSON在出现新笔压记录时使用version2，Qt接受1/2并复制完整pressureDynamics和独立尺寸/流量开关，升到既有state5/6；旧Qt明确拒绝version2。VBR1.17容纳converterVersion3来源，按全部来源最高需求导出，降级包拒绝。无新笔刷行为、state或C ABI布局变化。公开Schema与合法/非法示例同时更新。

验收：程序/图片三通道独立与组合压力、最低比例/开关/不同字段/未知控制/抖动报告、无工具覆盖误映射、非法边界/取消与旧v1/v2基准；真实笔触压力与历史、来源reset/VBR最高minor/降级拒绝/新旧混合及字节保持；原生ABR交接与Qt导入、重置、持久化和交换后真实笔触。以最小相关Rust/FFI/Qt测试和Release构建验证，不替代授权PS实机对照或声称完整ABR。

字段参考[ag-psd ABR读取器](https://github.com/Agamnentzar/ag-psd/blob/master/src/abr.ts)，该实现提供描述符字段与control枚举；不作为PS逐像素等价证据。16位、完整ABR动态、湿混双色/橡皮、收尖与非分隔等仍属后续阶段。
