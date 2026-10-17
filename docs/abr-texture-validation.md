# ABR 纹理导入验收

日期：2026-10-18，Windows 优先。

已接通 ABR 描述符中的 `useTexture`、`Txtr/Ptrn/Idnt`、`textureDepth`、`textureScale`、`textureBrightness`、`textureContrast`、`minimumDepth`、`textureDepthDynamics`、`TxtC`、`InvT` 与 `textureBlendMode`。`patt` 图案先执行尺寸、通道像素和总预算预检，再解码为灰度覆盖；图案以 PNG 内容哈希作为资源身份，并随 ABR 导入批次、托管资源目录、state11/21 handoff 和 Qt 预设状态保存。

当前映射：

- 文档固定或笔尖局部纹理由 `TxtC` 决定。
- `textureDepth`、`textureScale` 和 `InvT` 转换到已有 `TextureSettings`。
- `textureBrightness`、`textureContrast` 转换到 0..1 采样后的覆盖率近似调整；非零值要求 state21/VBR1.26。
- `minimumDepth`、`textureDepthDynamics` 的笔压深度和逐印章抖动已接入；最低深度作为深度下限，非默认值要求 state21/VBR1.27。
- `Nrml`、`Mltp`、`Drkn`、`Lghn`、`Sbtr`、`Dfrn`、`Scrn`、`CBrn` 映射到 `normal`、`multiply`、`darken`、`lighten`、`subtract`、`difference`、`screen`、`colorBurn`。
- 未识别的模式保留图案和参数，但纹理不启用，并报告 `notExecuted`；不会静默换成普通模式。
- 灰度、索引色和 RGB 图案由本地有界解码器处理 8/16 位 raw/RLE；16 位按四舍五入缩放，RGB 使用 Rec.601 亮度。
- 仅解码被预设引用的图案 ID；未引用图案保留在原始 ABR 中但不分配覆盖资源，缺失引用仍整包拒绝。
- 带非默认纹理的 ABR 来源记录使用 record version2，保存完整 `TextureParameters` 与资源 ID；旧纹理来源保持 version1。包含 version2 来源的 VBR 最低 minor27。

已验证：

- 新增生成夹具覆盖描述符纹理参数、`patt` 图案、资源哈希、纹理 handoff 版本 7、state11/21 绑定和真实两笔像素历史。
- `cargo test -p paint-io --test abr_import`：8 项通过，其中纹理用例验证图案覆盖、multiply/colorBurn 模式、亮度和对比度、深度动态及撤销重做。
- 新增 16 位 raw 灰度图案夹具验证 `0/65535` 量化到 8 位覆盖；真实 `sample-and-pattern.abr` 的 8 位 RLE 图案也通过同一本地解码器。
- `cargo test -p paint-io --test abr_conversion`：28 项通过，既有压力、散布、颜色、形状、设备转换及报告边界保持。
- `cargo test -p paint-io --test brush_texture_exchange`：9 项通过，包含 `colorBurn`、深度动态实际两笔像素、VBR1.25/VBR1.26/VBR1.27 输出、state21 恢复及降级拒绝；`vbr_image` 12 项分组/资源/版本回归保持。
- `cargo test -p paint-io --test vbr_source`：15 项通过，新增纹理来源 version2/资源引用/VBR1.27 往返和 minor26 降级拒绝。
- `cargo test -p paint-ffi --test abr_import`：9 项通过，包含带颜色加深、亮度/对比度和深度动态的纹理 ABR 经公开 `abr_read`、资源写入、state21 准备和来源记录，以及 148 个真实 ABR 预设和旧 v1/v2/v3/v4/v5/v6/v7 来源入口。
- Windows Qt Release 构建通过；含颜色加深、亮度/对比度和深度动态 handoff 的 `abrTextureHandoffImportsMappedModeIntoState11` 和现有 ABR 用户导入回归通过。
- “来源与兼容”页新增“恢复到来源基线”，可恢复导入时的笔尖、纹理、模式、亮度/对比度和深度动态；预设身份、分组和当前来源引用保持不变。
- `paint-io` 与 `paint-ffi` 的 all-target 严格 Clippy 通过。

边界：

- 八种已有纹理覆盖公式、亮度/对比度及深度动态进入执行。其余尚未定义的 Photoshop 模式和 `height` 特殊模式仍保留来源并报告未执行；`colorBurn` 是覆盖率标量近似，需 VBR1.25，亮度/对比度需 VBR1.26，深度动态需 VBR1.27。
- 其他 PSD 图像模式、完整 PS 逐像素对照和纹理角度尚未纳入本阶段；16 位笔尖仍明确拒绝，16 位纹理图案已支持。
- ABR 来源记录已写完整纹理基线和资源身份；Qt 导入快照与 VBR 导出/导入保持该配置。
- 未运行全量 `paint-io` 通过声明；本次运行中既有 `masks` 的 ORA 用例因缺少预期“背景”图层名称失败，与纹理路径无关。未运行跨平台和设备手感验收。
