# 双色绘制删除与模式收窄验收

2026-10-18，Windows。对应[设计](brush-mode-cleanup-design.md)。

- `paint-core --lib texture_modes_match_ps_style_coverage_combinations` 通过，包含 `height`/`linearHeight` 的 Krita 高度公式对照。
- `paint-core --test brush_texture` 7项、`brush_dual` 11项通过。
- `paint-io --test brush_texture_exchange` 15项、`brush_scatter_io` 9项通过；state/VBR 读写、旧版本门控、扩展模式和真实像素回归保持。
- Windows Qt 最小 8项通过（含初始化/清理）：纹理十模式下拉、双重笔刷八模式下拉、线性高度恢复、旧双色配置剥离和单色湿混回归。
- 双色绘制、自动描边和复合光标双色溢出的旧验收用例已标记跳过；灰度双重笔刷和复合光标的底层代码保留。
- Windows Release 构建、生成头检查和 C/C++ ABI 链接通过。

未运行全量 `tools/check.py`，未做跨平台、实体设备、GPU 或所有旧模式像素等价验收。
