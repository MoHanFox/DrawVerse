# 倾斜幅度动态验收

2026-10-18，Windows，对应 [设计](brush-tilt-design.md)。本阶段完成核心倾斜响应、state27/VBR1.33、Qt 设置和最小回归；不运行全量 `tools/check.py` 或跨平台验收。

- `paint-core --test brush_tilt` 3项通过：倾斜幅度与设备能力映射、尺寸曲线真实像素差异、流量/不透明度沉积差异。
- `paint-core --test brush_tangent` 4项、`brush_distance` 4项及双笔尖回归保持；第二笔尖拒绝自身倾斜配置。
- `paint-io --test brush_tilt_exchange` 1项通过：`tilt-v1`完整保存到state27/VBR1.33/behavior22/`tilt-dynamics`，重开读取原曲线；旧state26明确拒绝。
- `paint-io --test brush_distance_exchange`、`brush_state` 5项及 `vbr_image` 12项通过；分组、资源、来源和旧VBR字节保持。
- `cargo fmt --all -- --check`、严格Clippy、schema JSON解析及Windows Release Qt构建通过。
- `tiltDynamicsPresetsPreviewAndReopen`通过：state版本27、真实预览、保存和关闭重开恢复完整倾斜对象。
- `tiltDynamicsQmlToggleAndCurve`通过：Qt“传递”页开关、QML `checked`绑定、曲线设置和专用编辑器加载；速度/切向/距离/倾斜共十项Qt动态测试一起通过。
- 预览仍固定白色/正常模式；未增加实体设备倾斜手感、ABR倾斜来源或完整动态矩阵认证。

证据：`artifacts/dynamics-qt-combined.txt`、`artifacts/tilt-qt.txt`及当前Windows构建日志。测试只覆盖当前构建，不据此宣称完整Photoshop等效或完整笔刷系统完成。
