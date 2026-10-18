# 笔触方向动态验收

2026-10-18，Windows，对应 [设计](../../../brush/dynamics/brush-direction-design.md)。本阶段完成核心方向响应、state29/VBR1.35、Qt 设置和最小回归；不运行全量 `tools/check.py` 或跨平台验收。

- `paint-core --test brush_direction` 3项通过：方向角映射、尺寸曲线真实像素差异、流量/不透明度沉积差异。
- `paint-core --test brush_rotation` 3项、`brush_tilt` 3项及 `brush_distance` 4项通过；第二笔尖拒绝自身方向配置。
- `paint-io --test brush_direction_exchange` 1项通过：`direction-v1`完整保存到state29/VBR1.35/behavior24/`direction-dynamics`，重开读取原曲线；旧state28明确拒绝。
- `paint-io --test brush_state` 5项及 `vbr_image` 12项通过；分组、资源、来源和旧VBR字节保持。
- `cargo fmt --all -- --check`、严格Clippy、schema JSON解析及Windows Release Qt构建通过。
- `directionDynamicsPresetsPreviewAndReopen`通过：state版本29、真实预览、保存和关闭重开恢复完整方向对象。
- `directionDynamicsQmlToggleAndCurve`通过：Qt“传递”页开关、QML `checked`绑定、曲线设置和专用编辑器加载；速度/切向/距离/倾斜/旋转/方向共十四项Qt动态测试一起通过。
- 预览仍固定白色/正常模式；未增加实体设备方向手感、ABR方向来源或完整动态矩阵认证。

证据：`artifacts/dynamics-qt-combined.txt`、`artifacts/direction-qt.txt`及当前Windows构建日志。测试只覆盖当前构建，不据此宣称完整Photoshop等效或完整笔刷系统完成。
