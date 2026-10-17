# ABR计算双笔刷验收

2026-10-18，Windows，对应 [设计](abr-dual-brush-design.md)。只运行相关最小回归，未执行全量 `tools/check.py`。

- `paint-io --test abr_conversion abr_computed_dual_brush` 通过：计算双笔刷直径、硬度、混合模式、独立间距和双笔刷散布/数量映射到真实 `DualBrush`，converter版本为11。
- `brush_source` converter11双笔刷基线测试通过：source version3保留 `dualBrush`，converter10携带该字段明确拒绝。
- `paint-io --test abr_import` 9项继续通过；ABR handoff版本选择对converter11返回10。
- Windows Release Qt `abrComputedDualHandoffImportsIntoState12` 通过：handoff10导入、state版本至少12、双笔刷启用及直径恢复，handoff9拒绝双笔刷字段。
- 既有的渐隐、设备来源、纹理溶解和矩阵Qt用例继续通过；严格Clippy、格式检查及Release构建通过。

未验证sampled双笔刷资源关联、Flip、散布设备控制、真实Photoshop像素等价或跨平台。
