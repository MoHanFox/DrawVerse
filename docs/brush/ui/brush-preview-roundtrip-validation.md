# 笔刷预览轮转与 ABR 间距验收

2026-10-18，Windows。

- 组合传递抖动、关闭状态的双笔刷、平滑、关闭状态的湿混和起收笔后，真实异步笔刷预览通过；预览生成后不再因 `f32` 往返造成的等价数值差异被丢弃。
- `abr_computed_dual_brush_maps_shape_mode_and_spacing` 使用外层 `Spcn=100`、内层 `Brsh.Spcn=30`，转换结果为 `0.3`，通过。
- Windows Release 构建 `qt_ui_tests` 通过；未运行全量测试、跨平台、实体数位板或 GPU 验收。
