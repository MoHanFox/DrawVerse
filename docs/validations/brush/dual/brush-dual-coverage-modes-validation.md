# 双笔刷覆盖模式扩展验收

2026-10-18，Windows/MSVC/Qt 6.8.3 Release。对应[设计](../../../brush/dual/brush-dual-coverage-modes-design.md)。只运行相关最小回归，未执行完整 `tools/check.py` 或跨平台/硬件验收。

- `paint-core --test brush_dual` 11项通过：新增 13 种扩展模式与 `TextureMode` 公式逐值一致；原有六种模式、主零覆盖裁切、图片孔洞、涂抹、历史、预算和散布回归保持。
- `paint-io --test brush_scatter_io` 9项通过：state36/VBR1.43 保存 `Overlay` 与真实两笔像素，VBR1.42 及旧 state 拒绝；主/第二散布控制和旧版本往返保持，VBR1.43 模式 Schema 校验通过。
- `paint-io --test abr_conversion` 34项通过：ABR `Ovrl` 导入为 `DualMode::Overlay`，双笔刷散布控制及既有 ABR 动态/形状/颜色/纹理回归保持。
- Windows Qt `scatterSettingsPersistExchangePreviewAndCapturedPixels`、`abrScatterClosedAndActiveImportResetPersistAndExchangeRealPixels`、`scatterDeviceControlsPersistAndExportNativeContract` 共 5 passed（含初始化和清理）：新增模式状态36、behavior28、`dual-cover-extended`、VBR 包导入及 Qt 双笔刷页写入。
- 新增 `core/crates/paint-io/tests/fixtures/schema-history/dual/vbr-dual-modes-v1.43.schema.json`；VBR1.43/state36门控和旧版本拒绝已覆盖。Windows Release、workspace all-target check、严格 Clippy 和格式检查通过。

未完成：Hue/Saturation/Color/Luminosity 的颜色语义、ABR 随机抖动、16 位笔尖、实体设备和跨平台验收；这些不能由本阶段结果替代。
