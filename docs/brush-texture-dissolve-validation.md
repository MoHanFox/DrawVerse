# 纹理溶解模式验收

2026-10-18，Windows，对应 [设计](brush-texture-dissolve-design.md)。只运行相关最小回归，未执行全量 `tools/check.py`。

- `paint-core dissolve_mode_dithers_coverage_deterministically` 通过：输出仅 0/1，10,000 个像素均值落在 0.45–0.55，同一坐标重复结果一致。
- `paint-io --test brush_texture_exchange` 15项通过：`dissolve`写入 state33/VBR1.39，manifest minor39，重开后模式保持；低版本38/state32明确拒绝；`vbr-texture-modes-v1.39` schema通过解析且包含 dissolve。
- Windows Release Qt 构建及 `advancedTextureModesPresetsAndReopen`、`advancedTextureModesQmlDropdown` 通过：Qt 下拉21项，state33保存重开恢复 dissolve。
- 严格 Clippy、格式检查、全目标 `cargo check` 通过。

验收仅证明覆盖率二值抖动近似，不证明 Photoshop 颜色 Dissolve、Hue/Saturation/Color/Luminosity 或完整像素等价。
