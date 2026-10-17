# 笔旋转动态验收

2026-10-18，Windows，对应 [设计](brush-rotation-design.md)。本阶段完成核心旋转响应、state28/VBR1.34、Qt 设置和最小回归；不运行全量 `tools/check.py` 或跨平台验收。

- `paint-core --test brush_rotation` 3项通过：旋转角/设备能力映射、尺寸曲线真实像素差异、流量/不透明度沉积差异。
- `paint-core --test brush_tilt` 3项、`brush_distance` 4项及双笔尖回归保持；第二笔尖拒绝自身旋转配置。
- `paint-io --test brush_rotation_exchange` 1项通过：`rotation-v1`完整保存到state28/VBR1.34/behavior23/`rotation-dynamics`，重开读取原曲线；旧state27明确拒绝。
- `paint-io --test brush_tilt_exchange`、`brush_distance_exchange`、`brush_state` 5项及 `vbr_image` 12项通过；分组、资源、来源和旧VBR字节保持。
- `cargo fmt --all -- --check`、严格Clippy、schema JSON解析及Windows Release Qt构建通过。
- `rotationDynamicsPresetsPreviewAndReopen`通过：state版本28、真实预览、保存和关闭重开恢复完整旋转对象。
- `rotationDynamicsQmlToggleAndCurve`通过：Qt“传递”页开关、QML `checked`绑定、曲线设置和专用编辑器加载；速度/切向/距离/倾斜/旋转共十二项Qt动态测试一起通过。
- 预览仍固定白色/正常模式；未增加实体设备旋转手感、ABR旋转来源或完整动态矩阵认证。

证据：`artifacts/dynamics-qt-combined.txt`、`artifacts/rotation-qt.txt`及当前Windows构建日志。测试只覆盖当前构建，不据此宣称完整Photoshop等效或完整笔刷系统完成。
