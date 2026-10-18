# 笔触距离动态验收

2026-10-18，Windows，对应 [设计](../../../brush/dynamics/brush-distance-design.md)。本阶段完成核心距离响应、state26/VBR1.32、Qt 设置和最小回归；不运行全量 `tools/check.py` 或跨平台验收。

- `paint-core --test brush_distance` 4项通过：距离尺寸曲线改变笔触起点/终点真实像素，流量与不透明度沉积差异，关闭时像素/历史一致，第二笔尖拒绝自身距离配置。
- `paint-core --test brush_speed` 4项、`brush_tangent` 4项及 `brush_dual` 9项通过；速度、切向压力和双笔尖既有几何/历史保持。
- `paint-io --test brush_distance_exchange` 1项通过：`distance-v1`完整保存到state26/VBR1.32/behavior21/`distance-dynamics`，重开读取原曲线；旧state25明确拒绝。
- `paint-io --test brush_tangent_exchange`、`brush_state` 5项及 `vbr_image` 12项通过；分组、资源、来源和旧VBR字节保持。
- `cargo fmt --all -- --check`、严格Clippy、schema JSON解析及Windows Release Qt构建通过。
- `distanceDynamicsPresetsPreviewAndReopen`通过：state版本26、真实预览、保存和关闭重开恢复完整距离对象。
- `distanceDynamicsQmlToggleAndCurve`通过：Qt“传递”页开关、最大距离、QML `checked`绑定、曲线设置和专用编辑器加载；速度/切向/距离共八项Qt最小测试一起通过。
- 预览仍固定白色/正常模式；未增加实体设备手感、ABR距离来源或完整动态矩阵认证。

证据：`artifacts/dynamics-qt-combined.txt`、`artifacts/distance-qt-model.txt`、`artifacts/distance-qt-qml.txt`及当前Windows构建日志。测试只覆盖当前构建，不据此宣称完整Photoshop等效或完整笔刷系统完成。
