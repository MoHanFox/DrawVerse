# VBR 1.0 兼容性验收

2026-10-18，Windows。对应[设计](../../../brush/vbr/vbr-format-1.0-compatibility-design.md)。

- 写出 manifest 的 `minor` 固定为 `0`，带双笔刷、纹理、湿混、平滑、来源和分组等功能的导出均通过。
- 将同一包的历史 `minor` 改为 `0/1/8/24/44/999` 后均可读取；只有非 `1` 的 major、损坏资源、非法字段、错误引用和越界数据继续拒绝。
- 双笔刷对象缺失内部字段时补全后可绑定并执行；整体为 `null` 或缺少 `behaviorVersion`/`requiredCapabilities` 时按关闭或实际字段归一化处理，可选对象缺省保持关闭。
- 颜色、设备、散布、形状、平滑、双色、相对尺寸、湿混及 ABR 来源的历史缺项用例已迁移为“补全成功”，错误类型和越界值继续拒绝。
- `paint-io` 相关目标：`vbr_image` 13项、`vbr` 4项、`vbr_source` 15项、`brush_pressure` 7项，以及颜色/设备/散布/形状/平滑/双色/尺寸/湿混/来源相关交换目标通过。
- Windows Release 构建 `qt_ui_tests` 通过；第二笔刷默认值关闭与导入后重新关闭的最小 Qt 用例通过。禁用子菜单悬停由 QML 启用条件覆盖，未新增依赖弹层鼠标模拟的自动化用例。
- 未运行全量 `tools/check.py`、跨平台、实体数位板或 GPU 验收。仓库内历史版本 Schema 保留为旧字段说明，不再代表当前写出 manifest 的 minor。
