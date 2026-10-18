# 逐印章双笔刷双色验收

2026-10-18，Windows，对应 [设计](brush-dual-stamp-design.md)。本阶段完成 `interleaved-stamp-v1` 的核心、state22、VBR1.28、ABI1.56 和 Qt 设置入口；不宣称修复未定位的设备输入问题，也未运行跨平台验收或全量 `tools/check.py`。

## 核心

- `paint-core --test brush_dual_color` 13 项通过。新增 `interleaved_stamp_order_overlays_later_secondary_on_earlier_primary`：在同一位置构造前主印章与后第二印章交叉，逐印章像素为后一组的蓝色，整笔分隔对照为先前主印章的红色，证明两组模型的实际顺序不同。
- 同一测试集新增 `interleaved_model_is_paged_split_deterministic_and_undoable`：192×64 画布、`max_resident_tiles=1`、拆分输入与常驻像素一致，撤销回透明、重做恢复。
- `interleaved_model_requires_paired_dry_painting` 拒绝独立第二路径和湿混，同时接受关闭状态下的模型保留值。既有 27 种贡献模式、蒙版、透明锁、选区、偏移、纸纹、图像笔尖和历史回归保持。
- 橡皮与未开启手指绘画的涂抹忽略颜色模块并正常执行；`regular_smudge_does_not_inject_the_second_color` 验证普通涂抹不会注入第二色，`finger_painting_smudge_deposits_both_dual_color_contributions` 验证手指绘画涂抹的第一笔沉积实际产生主色中心与第二色外圈，历史仍为单笔。
- `paint-core --test import` 2 项通过；空图层栈按既有 `empty-layer-stack` 契约接受，其他非法元数据/像素/预算仍拒绝。

## 保存交换

- `paint-io --test brush_dual_color_boundary` 4 项通过：颜色对象严格读写 `interleaved-stamp-v1`，state 自动升到22，旧 state21 和启用独立路径的 state22 明确拒绝。
- `paint-io --test brush_dual_stamp_exchange` 3 项通过：behavior17/`dual-color-stamp`、VBR1.28 完整往返，两条真实笔触在重开后逐像素一致；VBR1.27 和 behavior16 明确拒绝，独立第二路径包明确拒绝。
- 既有双色、纹理、来源及分组包验收未改变；旧 `separated-stroke-v1` 字节和 state16/VBR1.14 语义保持。

## 原生与 Qt

- ABI 升到1.56，新增 `PAINT_FEATURE_DUAL_COLOR_INTERLEAVED`（bit51）。`paint-ffi --test async_session groups_public_state20_exact_capture_two_strokes_and_history` 通过，确认能力位在公开 capabilities 中。
- Windows Release `cmake --build build --config Release` 通过，包含 Rust 核心、paint_ffi、cbindgen 头一致性、`drawverse.exe` 和 `qt_ui_tests.exe`；四个 C11/C++20 静态/共享 ABI 烟测实际执行通过。
- Qt `dualColorClosedValuesHighestVersionResetAndPreviewTokens` 通过：逐印章模型完整保存、导出/导入保持、state22、路径模型组合被拒绝。
- Qt `dualColorControlsTwoColorPreviewCaptureExchangeAndReopen` 通过：既有整笔分隔双色的真实预览、捕获、像素、交换和重开未回归；设置页不再显示“使用第二颜色”，开启双色后配置和旧配置规范化都固定使用背景色。
- Qt `dualColorInterleavedPreviewPixelsAndPersistence` 通过：真实逐印章预设预览非空、真实笔触产生像素、撤销历史一条，偏好保存为 state22 且模型保持 `interleaved-stamp-v1`。
- Qt 设置页新增“整笔分隔合成/逐印章交织”下拉；逐印章要求配对笔尖，独立路径组合在设置层拒绝。橡皮和普通涂抹忽略双色，手指绘画涂抹使用双色；双色湿混的涂抹/橡皮经用户确认没有实际意义，保持不处理并移出待办。

## 证据

- `artifacts/dual-stamp-core-tests.log`
- `artifacts/dual-stamp-io-tests.log`
- `artifacts/dual-stamp-ffi-test.log`
- `artifacts/dual-stamp-clippy.log`
- `artifacts/dual-stamp-qt-build.log`
- `build/bin/Release/dual-stamp-qt-result.txt`
- `build/bin/Release/dual-stamp-qt-windows-result.txt`

## 边界

整笔不透明度只应用一次；逐印章只改变主/第二贡献的叠加时机，不改变工具混合模式、选区、图层锁或历史事务。该模型不处理湿混、橡皮、涂抹和独立第二路径。实际数位板手感和跨平台仍由后续验收，未用本轮 Qt 软件窗口测试替代设备测试。
