# 时间平滑IO验收

2026-10-14，state14/VBR1.12/behavior11，当前ABI仍1.45；设计见brush-smoothing-io-design.md。完整time-grid-v1参数以f64严格校验，默认不升级旧字节/版本，关闭非默认保留。无图片的新模块也进入统一bundle格式，执行由ImageBundle.definition/BrushSettings.definition复制原内核设置；主/第二共用同一过滤路径。

对应最小证据：
- brush_smoothing_boundary 1项：启用/关闭非默认state14完整还原，旧state13明确拒绝；默认编码字节往返相同。
- brush_smoothing_exchange 3项：平滑与PCHIP压感/不透明度压感/独立双流组合的两条真实时间笔触，原始/state/VBR逐像素一致且撤销重做一致；关闭参数保持并与未启用像素一致，启用与未平滑像素不同；未知模型/null/缺失/类型/舍入前范围及行为/能力/manifest降级拒绝，Schema版本/范围断言。
- vbr_source 7项：新平滑图片笔触恢复、原始ABR字节与原记录逐字节/来源ID保持；既有完整来源/复杂模块/原包与当前基线资源及1024边界回归通过。
- brush_pressure 7、brush_dual_exchange 3项通过；原7项时间平滑内核回归通过，含输入拆分、设备/独立过滤、预算拒绝与取消、四类写入/选区/蒙版/偏移/分页及真实历史。
- legacy_native_bridge_rejects_smoothing_without_publishing_or_writing通过：开关两种配置在准确准备阶段返回Unsupported，旧目录保持；库导入/两种公开写包入口暂拒并保留原文件。
- paint-io/paint-ffi全targets Clippy -D warnings通过，workspace全targets cargo check通过，生成C头字节检查通过。

证据文件：artifacts/brush-smoothing-io-tests.log、brush-smoothing-io-combined.log、brush-smoothing-io-native.log、brush-smoothing-io-core.log、brush-smoothing-io-clippy.log、brush-smoothing-io-header.log。未运行tools/check.py全仓；本轮没有Qt行为改动，不报告Qt界面完成。当前原生桥/Qt最高state13不能安全保留新字段，显式拒绝是临时兼容保护，后续完整桥/界面接入后解除并发布能力。收尖/双色自动描边/湿混/ABR高级映射与完整系统继续后续。
