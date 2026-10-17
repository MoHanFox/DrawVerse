# ABR计算双笔刷验收

2026-10-18，Windows，对应 [设计](abr-dual-brush-design.md)。只运行相关最小回归，未执行全量 `tools/check.py`。

- `paint-io --test abr_conversion` 通过：计算双笔刷直径、硬度、混合模式、独立间距、散布/数量、随机 Flip 和形状 flipX/flipY 映射；sampled 双笔刷样例绑定第二图片 mask。
- 核心 `dual_flip_is_seeded_per_stamp` 和 `dual_parameters_preserve_random_flip` 通过：翻转位逐印章变化并随 DualParameters 持久化。
- Qt `dualFlipQmlToggle` 通过：随机 Flip 和固定 X/Y 镜像复选框都能写入双笔刷配置。
- `brush_source` converter11双笔刷基线测试通过：source version3保留 `dualBrush`，converter10携带该字段明确拒绝。
- `paint-io --test abr_import` 10项通过；新增真实ABR `samp`双记录、sampled dual handoff JSON和持久化mask身份用例。
- Windows Release Qt `abrComputedDualHandoffImportsIntoState12` 通过：handoff10导入、state版本至少12、双笔刷启用及直径恢复，handoff9拒绝双笔刷字段。
- 既有的渐隐、设备来源、纹理溶解和矩阵Qt用例继续通过；严格Clippy、格式检查及Release构建通过。

第二笔尖散布设备控制已在后续阶段接入，见[散布设备控制验收](brush-scatter-device-validation.md)；仍未验证真实Photoshop像素等价或跨平台。
