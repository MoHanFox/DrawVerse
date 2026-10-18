# 湿混状态与VBR交换验收

2026-10-15，Windows/MSVC/Rust与Qt6.8.3 Release。state17、VBR1.15、drawverse.brush/behavior14和wet-mix能力已实现于Rust IO；ABI仍1.49，旧Qt尚无湿混字段/绘画入口。对应设计见brush-wet-io-design.md。未运行完整tools/check.py。

- brush_wet_exchange四项通过，日志artifacts/brush-wet-io-test.log。开关×自动清洗×自动装载八组合，带PCHIP大小/不透明度压感与颜色动态；三种定义（原始/state恢复/VBR恢复）在真实分页文档上各执行两笔，跨笔复用返回载体，像素与载体采样精确一致，真实两次undo/redo保持。
- 图片/纹理共享源、灰度双路径、相对固定宽度、关闭双色配置、time-grid平滑与散布组合，state/VBR实际时间输入两笔及返回载体精确一致；原始ABR字节和sourceResource保持。converter v1基线不改写成湿混，当前修改与原记录分离。
- 六比例在转f32前拒绝正负越界，完整字段/缺失/null/未知/错误布尔/模型、profile/behavior/能力、state16与manifest1.14降级拒绝。取消读写返回错误，旧procedural-only容器拒绝湿混，失效包及未支持有效双色湿混组合保存失败不覆盖原文件。合法/非法公开示例由Rust解析，两个合法示例执行真实绘画；最终示例回归见brush-wet-io-fixtures.log。
- brush_wet_settings两项通过：state17保留关闭非默认参数，默认省略与旧状态规范字节往返保持；旧版本/null/舍入前越界拒绝；ABR来源单项/批次基线继续拒绝非默认湿混。日志brush-wet-io-test.log。
- old_native_entries_reject_wet_without_publishing_or_writing通过，启用/关闭非默认state准备返回Unsupported且库不变，公开库包读写/旧无库写入准确拒绝，required为0，已有文件字节不变。拒绝在资源导入前执行。日志brush-wet-io-native.log；这不是已支持原生湿混入口。
- 相关旧brush_smoothing_exchange三项、brush_dual_color_exchange三项、vbr四项、vbr_image四项、vbr_source八项通过，日志brush-wet-io-regression.log。本阶段去重29项相关Rust用例通过。
- 新Schema contracts/vbr-wet-v1.15.schema.json：沿用完整1.14子集，新增严格11字段wetMix、行为14/能力对应、最高17能力及有效双色湿混拒绝。公开合法、关闭双色合法、非法越界示例位于contracts/fixtures。使用临时工具目录中的jsonschema4.23.0 Draft202012Validator完成Schema自检、两个合法样例与36非法变体验证，日志brush-wet-io-schema.log。可用同版本python -m jsonschema -i contracts/fixtures/vbr-wet-valid.json contracts/vbr-wet-v1.15.schema.json复核合法示例；非法ratio示例必须失败。工具依赖未加入产品。
- workspace/all-targets cargo check、paint-io/paint-ffi全目标clippy -D warnings、Release构建及构建内生成头--check、静态/共享C/C++四项ABI smoke均通过，日志brush-wet-io-{check,clippy,build,abi}.log。既有Qt颜色设置/捕获/实际预览/VBR/保存重开/像素及历史用例通过，日志brush-wet-io-qt.txt；未宣称Qt湿混可用。Qt部署工具仍提示SDK缺translations/catalogs.json（--no-translations构建成功）。

默认WetSettings不增加对象或提高格式版本；关闭非默认保留。实际前景/背景色、载体网格/路径和工具载色不进入预设或VBR。state17与bundle解析/编译完整对象，执行使用既有Rust配置湿混入口。

后续：完整原生准备/执行桥与Qt分类参数、实际有色底预览、持久resetState基准及工具载体寿命/手动清洗装载；接入后再解除本阶段原生临时拒绝。双色湿混、收尖、非分隔绘制、高级ABR和完整设备/跨平台/性能验收仍未完成，不视为整个湿混或笔刷系统完成。
