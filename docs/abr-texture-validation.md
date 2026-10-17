# ABR 纹理导入验收

日期：2026-10-18，Windows 优先。

已接通 ABR 描述符中的 `useTexture`、`Txtr/Ptrn/Idnt`、`textureDepth`、`textureScale`、`textureBrightness`、`textureContrast`、`TxtC`、`InvT` 与 `textureBlendMode`。`patt` 图案先执行尺寸、通道像素和总预算预检，再解码为灰度覆盖；图案以 PNG 内容哈希作为资源身份，并随 ABR 导入批次、托管资源目录、state11/21 handoff 和 Qt 预设状态保存。

当前映射：

- 文档固定或笔尖局部纹理由 `TxtC` 决定。
- `textureDepth`、`textureScale` 和 `InvT` 转换到已有 `TextureSettings`。
- `textureBrightness`、`textureContrast` 转换到 0..1 采样后的覆盖率近似调整；非零值要求 state21/VBR1.26。
- `Nrml`、`Mltp`、`Drkn`、`Lghn`、`Sbtr`、`Dfrn`、`Scrn`、`CBrn` 映射到 `normal`、`multiply`、`darken`、`lighten`、`subtract`、`difference`、`screen`、`colorBurn`。
- 未识别的模式保留图案和参数，但纹理不启用，并报告 `notExecuted`；不会静默换成普通模式。
- 灰度、索引色和 RGB 图案按已核对的 8 位解码路径转换；RGB 使用 Rec.601 亮度。

已验证：

- 新增生成夹具覆盖描述符纹理参数、`patt` 图案、资源哈希、纹理 handoff 版本 7、state11/21 绑定和真实两笔像素历史。
- `cargo test -p paint-io --test abr_import`：7 项通过，其中纹理用例验证图案覆盖、multiply/colorBurn 模式、亮度和对比度及撤销重做。
- `cargo test -p paint-io --test abr_conversion`：28 项通过，既有压力、散布、颜色、形状、设备转换及报告边界保持。
- `cargo test -p paint-io --test brush_texture_exchange`：7 项通过，包含 `colorBurn` 实际两笔像素、VBR1.25/VBR1.26 输出、state21 恢复及降级拒绝；`vbr_image` 12 项分组/资源/版本回归保持。
- `cargo test -p paint-ffi --test abr_import`：9 项通过，包含带颜色加深和亮度/对比度的纹理 ABR 经公开 `abr_read`、资源写入、state21 准备和来源记录，以及 148 个真实 ABR 预设和旧 v1/v2/v3/v4/v5/v6/v7 来源入口。
- Windows Qt Release 构建通过；含颜色加深 handoff 的 `abrTextureHandoffImportsMappedModeIntoState11` 和现有 ABR 用户导入回归通过。
- `paint-io` 与 `paint-ffi` 的 all-target 严格 Clippy 通过。

边界：

- 八种已有纹理覆盖公式及亮度/对比度调整进入执行。其余尚未定义的 Photoshop 模式、`height` 特殊模式、`minimumDepth` 和 `textureDepthDynamics` 仍保留来源并报告未执行；`colorBurn` 是覆盖率标量近似，需 VBR1.25，亮度/对比度需 VBR1.26。
- 16 位图案、其他 PSD 图像模式、完整 PS 逐像素对照和纹理角度尚未纳入本阶段。
- ABR 来源记录已接受 converterVersion8，但纹理参数尚未写入来源基线的独立字段；Qt 导入快照本身保留纹理配置。
- 未运行全量 `paint-io` 通过声明；本次运行中既有 `masks` 的 ORA 用例因缺少预期“背景”图层名称失败，与纹理路径无关。未运行跨平台和设备手感验收。
