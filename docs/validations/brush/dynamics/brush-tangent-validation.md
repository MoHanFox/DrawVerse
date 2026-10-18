# 切向压力动态验收

2026-10-18，Windows，对应 [设计](../../../brush/dynamics/brush-tangent-design.md)。本阶段完成核心切向压力响应、state25/VBR1.31、Qt 设置和最小回归；不运行全量 `tools/check.py` 或跨平台验收。

- `paint-core --test brush_tangent` 4项通过：-1/1设备范围映射、尺寸曲线真实像素差异、流量/不透明度曲线沉积差异、关闭时像素与历史参考一致，以及第二笔尖拒绝自身切向配置。
- `paint-core --test brush_speed` 4项和 `brush_dual` 9项通过：速度行为与既有双笔尖几何保持；第二笔尖自身仍拒绝切向压力配置。
- `paint-io --test brush_tangent_exchange` 1项通过：`tangent-v1`完整保存到state25/VBR1.31/behavior20/`tangent-dynamics`，重开读取原曲线；旧state24明确拒绝。
- `paint-io --test brush_state` 5项、`vbr_image` 12项及速度交换回归通过；分组/旧版本字节保持，未来minor断言使用当前`MAX_EXECUTABLE_MINOR + 1`。
- `cargo fmt --all -- --check`、严格Clippy、Windows Release Qt构建和活动C++资源检查通过。
- `tangentDynamicsPresetsPreviewAndReopen`通过：state版本25、真实预览、保存和关闭重开恢复完整切向对象。
- `tangentDynamicsQmlToggleAndCurve`通过：Qt“传递”页开关、QML `checked`绑定、切向曲线设置和专用编辑器加载；与速度四项最小测试一起通过。
- 预览仍固定白色/正常模式；未增加实体数位板非零切向压力的认证，也未宣称ABR切向来源或完整动态矩阵完成。

证据：`artifacts/dynamics-qt-combined.txt`、`artifacts/tangent-qt-model.txt`、`artifacts/tangent-qt-qml.txt`及当前Windows构建日志。测试只覆盖当前构建，不据此宣称完整Photoshop等效或完整笔刷系统完成。
