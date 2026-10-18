# 动态来源矩阵验收

2026-10-18，Windows，对应 [设计](../../../brush/dynamics/brush-dynamics-matrix-design.md)。本阶段只运行相关最小测试，未执行全量 `tools/check.py` 或跨平台验收。

- `paint-core source_matrix_selects_dynamics_per_target_channel` 1项通过：默认 `All` 对六来源逐通道相乘；显式选择距离/倾斜/旋转后分别取得对应通道因子；`brush_source_matrix` 真实像素用例验证单来源恒等曲线使用归一化输入。
- `paint-io --test brush_source_matrix_exchange` 1项通过：`source-matrix-v1` 写入 state32/VBR1.38/behavior25/`dynamics-source-matrix`，JSON 字段和真实状态重开保持；state31 夹带对象明确拒绝。
- 动态专用包不再误走旧平包：`vbr_image::encode` 和 `vbr::encode` 将 airbrush、六类动态及 sourceMatrix 视为带版本对象；矩阵包实测 manifest minor38，方向/速度交换回归继续通过。
- `cargo check -p paint-core -p paint-io -p paint-ffi --all-targets` 通过；第二笔尖拒绝自身矩阵，主预设因子可分发给配对笔尖。
- Windows Release `cmake --build build/qt --config Release --target qt_ui_tests` 通过，Rust 核心、C ABI 头检查和 Qt 测试程序重新构建。
- `dynamicsSourceMatrixPersistsPreviewAndReopen` 通过：真实预览、state32 保存、关闭重开恢复三个来源选择。
- `dynamicsSourceMatrixQmlDropdowns` 通过：“传递”页三个下拉各7项，`setDynamicsSource` 后绑定索引同步。
- Schema `core/crates/paint-io/tests/fixtures/schema-history/dynamics/vbr-dynamics-matrix-v1.38.schema.json` 已加入并通过 JSON 解析。

证据为当前 Windows 构建和上述测试退出码。未运行全量测试，未宣称实体设备手感、ABR 动态来源映射、跨平台或 GPU 已完成。
