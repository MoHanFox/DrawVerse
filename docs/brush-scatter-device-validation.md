# 散布设备控制验收

2026-10-18，Windows/MSVC/Qt 6.8.3 Release。对应[设计](brush-scatter-device-design.md)。只运行相关最小回归，未执行完整 `tools/check.py` 或跨平台/硬件验收。

- `paint-core --lib brush_scatter` 5项通过：默认关闭控制；压力控制幅度和数量；渐隐到最低比例并夹限；倾斜/方向/旋转通道；非法最低比例、渐隐步数和关闭控制拒绝。
- `paint-core --test brush_scatter` 8项通过：原有固定种子、数量抖动、图片/选区/擦除/蒙版/涂抹和历史回归保持；默认控制与原像素一致；压力控制真实改变同一笔触的印章数量。
- `paint-core --test brush_dual secondary_scatter_pressure_control_changes_real_stamp_count` 通过：第二笔尖压力控制真实改变同一点的主/第二印章数量。
- `paint-io --lib brush_scatter` 3项通过：旧 JSON 不带控制字段、控制字段往返和非法控制 JSON 拒绝。
- `paint-io --test brush_scatter_io` 7项通过：state35/VBR1.42 保留主/第二笔尖控制与真实两笔像素；旧 state/VBR 版本拒绝；既有 seed/数量边界和 VBR1.42 控制 Schema 保持。
- `paint-io --test abr_conversion` 34项通过：ABR `bVTy=1/2/3/6/8`、`Mnm`/`fStp` 主/第二映射、未知控制报告、旧默认报告和既有双笔刷/形状/颜色/纹理回归。
- Windows Qt `scatterSettingsPersistExchangePreviewAndCapturedPixels`、`abrScatterClosedAndActiveImportResetPersistAndExchangeRealPixels`、`scatterDeviceControlsPersistAndExportNativeContract` 通过；总计 5 passed（含初始化/清理），覆盖主/第二控制状态35、行为27/`scatter-controls`、ABR handoff11、VBR 状态往返和真实像素路径。
- Windows Release Qt 构建、Rust workspace all-target check、格式检查通过。未验证实体设备的非零倾斜/方向/旋转、跨平台、完整 PS 像素等价或 GPU。

明确未完成：Hue/Saturation/Color/Luminosity、ABR 随机抖动、16 位笔尖和完整 PS 对照；这些不能被本阶段结果替代。
