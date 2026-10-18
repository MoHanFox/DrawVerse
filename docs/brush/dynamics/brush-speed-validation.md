# 速度动态验收

2026-10-18，Windows，对应 [设计](brush-speed-design.md)。本阶段完成核心速度响应、主/第二笔尖尺寸、state24/VBR1.30、Qt 设置和最小回归；不运行全量 `tools/check.py` 或跨平台验收。

- `paint-core --test brush_speed` 4项通过：尺寸曲线随距离/时间改变真实像素；流量和不透明度曲线改变实际沉积与上限；配对第二笔尖在独立尺寸下也随速度缩放；关闭时像素和历史与参考一致。
- `paint-core --test brush_dual` 9项、`brush_dual_color` 13项通过：双笔刷原有几何、溢出、颜色、湿混、分页、选区、蒙版、锁、历史和失败回滚保持。
- `paint-io --test brush_speed_exchange` 1项通过：`speed-v1` 完整保存到 state24/VBR1.30/behavior19/`speed-dynamics` 能力，重开读取原曲线；旧 state23 和 VBR1.28 明确拒绝。
- `paint-io --test brush_state` 5项和 `vbr_image` 12项通过。分组降级用例的未来 minor 断言改为 `MAX_EXECUTABLE_MINOR + 1`，与当前 VBR1.30 上限一致；旧分组/资源/来源语义不变。
- `cargo clippy -p paint-core -p paint-io -p paint-ffi --all-targets -- -D warnings` 通过，`cargo fmt --all -- --check` 通过。
- Windows Release Qt 构建通过；`speedDynamicsPresetsPreviewAndReopen` 通过：state版本24、真实预览、保存和关闭重开恢复完整速度对象。
- `speedDynamicsQmlToggleAndMax` 通过：实际点击“启用速度动态”、修改速度上限到1200后模型同步，专用速度曲线组件可加载。
- 预览保持白色/正常模式；未新增试画功能。ABR 速度来源映射、任意动态来源矩阵、实体数位板手感、GPU 执行和 macOS/Linux 仍未验收。

证据：`artifacts/speed-qt-build.log`、`artifacts/speed-qt-model.txt`、`artifacts/speed-qt-qml.txt`。测试只覆盖当前 Windows 构建，不据此宣称完整 Photoshop 等效或完整笔刷系统完成。
