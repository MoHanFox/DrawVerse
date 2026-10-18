# 笔刷渐隐动态验收

2026-10-18，Windows，对应 [设计](../../../brush/dynamics/brush-fade-design.md)。只运行相关最小回归，未执行全量 `tools/check.py`。

- `paint-core --test brush_fade` 3项通过：4步尺寸渐隐在真实64×64笔触中缩小末端覆盖，关闭渐隐与参考像素完全一致，第二笔尖拒绝自身渐隐配置。
- `paint-io --test brush_fade_exchange` 2项通过：state34/VBR1.40往返、manifest minor40、旧state33拒绝及schema字段检查。
- `paint-io --test abr_conversion abr_fade_control` 通过：`bVTy=1`、`fStp=4`映射converter10渐隐，尺寸最低比例为0，其他通道保持平坦。
- `brush_source` converter10基线测试通过；converter9携带渐隐明确拒绝。
- Windows Release Qt `fadeDynamicsPresetsPreviewAndReopen`、`fadeDynamicsQmlToggleStepsAndCurve`、`abrFadeHandoffImportsStampStepsIntoState34` 通过：真实预览、步数控件、曲线、state34保存重开和handoff9恢复。
- 严格 Clippy、格式检查、全目标 `cargo check` 通过。

未验证散射/双笔尖的PS步数等价、随机抖动、实体设备手感或多平台。
