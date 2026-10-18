# 整条笔触描边保存交换验收

2026-10-19 Windows。对应[保存交换设计](../../../brush/core/brush-stroke-outline-io-design.md)。

state41、VBR 1.0、behavior33和`stroke-outline`能力已接通。默认BrushCatalog仍写state2且不出现`strokeOutline`；启用与关闭的非默认对象都提升state41，state40携带该对象明确拒绝。纯文字VBR明确拒绝描边，ImageBundle保存自身、已有笔刷和纹理来源，manifest写到1.0，并保持完整关闭值与资源引用。

`cargo test -p paint-io --test brush_stroke_outline_exchange --locked`三项通过：自身来源在启用/关闭、state/VBR与真实像素一致；已有图片笔尖的间距、散布、随机翻转及纹理来源在state、VBR和全新资源目录重开后像素一致；f64舍入前的宽度范围、未知字段、未知模型、null和非法组合均拒绝。普通`begin_brush_stroke`直接执行预设描边，不依赖显式参数入口。

相关旧回归38项通过：平滑3、来源8、state6、起收笔8、ImageBundle13。覆盖默认旧字节、state1/2迁移、旧minor、资源身份和取消保存。`cargo test -p paint-io --tests --no-run --locked`编译全部paint-io测试目标；`cargo clippy -p paint-io --lib --test brush_stroke_outline_exchange --locked -- -D warnings`和`cargo fmt --all -- --check`通过。

未运行全量`tools/check.py`、ABI/Qt、跨平台或设备验收。ABI与Qt仍待后续步骤，本阶段不宣称整项描边完成。

