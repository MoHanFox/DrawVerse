# 整条笔触描边原生入口

2026-10-19，ABI1.60。对应[保存交换](brush-stroke-outline-io-design.md)，本阶段接明确原生准备和资源包读写，Qt镜像随后接入。

新增`PAINT_FEATURE_STROKE_OUTLINE`（bit55）：

- `paint_brush_library_prepare_state_outline`接受完整state41，走既有异步准备、准确票据、当前/在途目录、来源票据和冷重建管线。
- `paint_brush_library_package_read_outline` / `paint_brush_library_package_write_outline`读写VBR 1.0，保留自身、已有笔刷和纹理来源以及全部内容ID资源。
- 新请求可携带所有既有完整参数；关闭但非默认的描边对象仍被镜像和执行，不以enabled决定是否保存。

普通、wet、taper和wet-dual入口继续拒绝描边对象，防止旧客户端静默丢失字段。Groups准备与分组包入口属于完整参数入口，因此也接受描边；Qt仍必须同时要求`BRUSH_GROUP_*`与`STROKE_OUTLINE`能力，不能只凭旧ABI或一个能力位推断。

包写入从库资源目录收集主笔尖、纹理、双笔刷、来源和描边已有笔刷/纹理身份；容量探测不安装资源，正式读取先安装完整束再返回JSON。准备票据保持不可变BrushSettings，已开始的笔触继续使用起笔捕获，后续目录替换或库销毁不改变在当前修订上已执行的描边。

本阶段无DTO布局变化，ABI minor升到1.60；state41/VBR 1.0/behavior33独立演进。Qt设置、真实预览和保存重开仍待后续。

