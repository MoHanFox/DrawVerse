# 双色双笔刷保存与交换验收

2026-10-15，state16/VBR1.14/behavior13/dual-color-stroke；ABI仍1.47。嵌套color严格保存separated-stroke-v1、三个开关、前后顺序和两个27种模式名称，实际工具RGBA不入预设。默认省略、旧字节及版本不升级。原生/Qt尚未接色彩配置，准备/包入口暂拒非默认双色配置以防丢失。

- brush_dual_color_exchange 3项：双笔刷开关×双色开关×配对/独立路径×前后/溢出共16组，组合相对固定宽度、time-grid平滑、PCHIP大小和不透明度压感；恢复后在有色半透明底上执行两笔不同前景/背景色的真实Multiply工具绘画，state/VBR逐像素一致、三条历史（底色+两笔）及撤销重做保持。state15/manifest1.13拒绝，缺失能力及behavior错误拒绝。
- 严格完整字段/unknown/null/类型/组穿透及未实现per-dab模型拒绝；Schema模型/版本和27个主/第二模式名称与Rust表逐项一致。未运行完整JSON Schema解释器验证。
- brush_dual_color_boundary 2项：双模块开关四组合、全部27模式确切枚举往返；默认目录编码/恢复字节保持旧版本。
- vbr_source的dual_color_current_image_preserves_shared_secondary_and_original_provenance通过：当前图片共享给第二笔尖，与平滑及双色共同保存；原ABR字节/记录/sourceResource保持、图片身份一致、真实时间笔触像素一致。原始converter基线不改写成双色定义。
- native_dual_color_gate_preserves_current_catalog_and_file通过：关闭/启用双色的state准备与图片包接入返回Unsupported，当前库保持；公开库包写入返回Unsupported，已有文件不变。
- 相关brush_dual_size_exchange 3项、原生相对尺寸准确准备/捕获/库包1项通过；合计11项唯一对应回归。workspace all-targets cargo check、paint-io/paint-ffi all-targets Clippy -D warnings通过。证据brush-dual-color-io-tests/new/source/native/related-native/check/clippy.log。
- Release Qt应用/Rust FFI与生成头一致性构建通过（brush-dual-color-io-build.log）；应用未运行时更新开发目录exe/DLL并核对DLL SHA256。

未运行全仓tools/check.py或Qt交互测试。下一阶段先优化双色重算范围，再接原生准备/Qt入口、真实预览/完整重置/关闭重开、自动描边预设与复合光标；非分隔模式、收尖、湿混、高级ABR与完整设备/跨平台验收仍未完成。
