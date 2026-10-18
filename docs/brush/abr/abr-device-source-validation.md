# ABR设备来源映射验收

2026-10-18，Windows，对应 [设计](abr-device-source-design.md)。只运行相关最小回归，未执行全量 `tools/check.py` 或跨平台验收。

- `paint-io --test abr_conversion` 30项通过；新增两项验证 `bVTy=3/8/6` 分别映射尺寸/流量/不透明度来源、清除笔压标记并写入 converter v9，以及倾斜尺寸来源在真实 64×64 文档中改变笔刷半径。
- `paint-core` 的 `brush_speed`、`brush_tangent`、`brush_distance`、`brush_tilt`、`brush_rotation`、`brush_direction` 共21项及 `brush_source_matrix` 1项通过；单来源恒等曲线使用归一化输入，默认 `All` 和既有压力行为保持。
- `paint-io` 内部 `converter_nine_source_baseline_keeps_device_matrix` 通过：converter 9 源基线保留 tilt/sourceMatrix，converter 8 携带同一基线明确拒绝。
- Windows Release `cmake --build build/qt --config Release --target qt_ui_tests` 通过；`abrDeviceSourceHandoffImportsTiltRotationDirectionAndMatrix` 通过合成 handoff v8 导入、state32、三种动态和矩阵恢复，并验证 v7 拒绝。
- 严格 Clippy、`cargo fmt --all -- --check`、全目标 `cargo check` 通过。

证据为当前 Windows 构建和上述测试退出码。未验证 ABR 未知控制、渐隐/抖动、真实 Photoshop 设备语义、完整像素等价、16 位笔尖或多平台。
