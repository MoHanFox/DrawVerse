# 组穿透（2026-10-12）

用户明确要求新增组专用混合选项“穿透”，不改变现有组默认正常/隔离行为。BlendMode::PassThrough=27，旧 0–26 编号不变；仅组可选，像素层/蒙版及预览非法使用必须拒绝。组混合下拉附加穿透项；悬停预览共享快照，提交为既有 Appearance 原子历史并可撤销。

穿透组子节点对传入父级当前 backdrop 自下而上合成，嵌套穿透继续传递，嵌套普通组仍隔离。组整体影响使用 premult RGBA 的 before→after 插值（opacity×fill×蒙版显示覆盖），空组不改变背景，隐藏/零权重不读取子节点冷页。组内剪贴链保留既有 alpha 约束；穿透组作为剪贴基底时，后续剪贴效果使用组自身覆盖限制，不借用组外底层 alpha；被剪贴的穿透组对链基底进行有界影响并保留基底 alpha。

只对含穿透的文档启用有界逐行递归，深度仍最大 16，源瓦片不可变，不复制整文档。普通组保留原合成路径，渲染缓存已有 appearance mode 身份随值 27 失效。取色、导出、视口统一读取相同核心结果。

ABI 1.19，新增 feature bit 65536 与 PAINT_BLEND_PASS_THROUGH=27；已有 DTO/函数不变。ORA stack isolation=auto，dv:blend=27 保存精确模式；加载接受外部 auto+src-over+opacity=1 的穿透，其他需隔离的标准组合按规范隔离；DrawVerse 扩展保留组透明度下的穿透语义。isolation 未指定沿用既有隔离解释，未知值拒绝，非组值27拒绝。旧 dv 文档不自动迁移。

最小验收：实际颜色减淡对组外下层、嵌套隔离/穿透、opacity/fill/蒙版/隐藏、剪贴、逐行/单像素、历史和非法使用；ORA 回环和外部 auto、真实渲染缓存变化；Qt 组专属选项、悬停预览无历史、提交/撤销/实际帧。只运行相关定向测试、fmt/clippy、头生成严格检查和 Release 构建。

规范参考：[Adobe 组混合模式](https://helpx.adobe.com/photoshop/desktop/create-manage-layers/apply-layer-effects/set-layer-opacity-and-blending-modes.html) / [OpenRaster stack isolation](https://www.openraster.org/baseline/layer-stack-spec.html)。
