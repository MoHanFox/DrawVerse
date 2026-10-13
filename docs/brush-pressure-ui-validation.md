# 压力曲线入口验收

ABI 1.35 声明 `PAINT_FEATURE_PRESSURE_DYNAMICS=1073741824`，DTO 不变。Qt v5 与 VBR1.3 接入尺寸/流量压力曲线及最低比例，形状动态/传递分类采用实际原生预览。来源元数据保持独立。

最小 Qt 回归 `pressureCurveEditorPersistsAndExchangesNativeCurves`：真实面板点击增点、严格拒绝重复 X、配置保存与关掉压力开关仍保留数据、真正后台预览生成、现有原生 VBR 导出导入往返全部通过；无 QML 警告。尚未逐一自动操作拖动/删除/重置和所有数值输入，不能宣称已覆盖全部交互。

FFI `pressure_packages_round_trip_through_native_entry`：原生 package 读写 VBR1.3 实际通过，读出的 JSON 可作为有效 Package 解析，写出的包恢复压力字段。原临时 Unsupported 边界撤销。核心压力动态原有 5 项测试通过；原生 C/C++ 4 项 smoke 和 cbindgen 检查通过。

本阶段与独立尺寸/整笔不透明度一起验收，后者的新规则覆盖旧低不透明度像素兼容声明，见 `brush-stroke-opacity-design.md`。未跑全量验证、未做真实数位板硬件验收；其他动态与湿混尚未完成。
