# 起收笔 Qt 验收

2026-10-16，Windows 11 / Qt 6.8.3 Release，software Quick，环境倍率 1。对应[设计](../../../brush/wet/brush-taper-ui-design.md)。

Qt 完整镜像 state18 的 strokeTaper，关闭值、复制及 resetState 基准均保留；默认对象省略，不改变旧预设版本。当前预设、准确准备、白色自动预览、保存重开及 VBR1.22 包交换已改用 ABI1.51 显式 taper 入口，最低版本和 feature 检查同步。无新增 ABI 或格式版本。

设置页增加起收笔分类、总开关、起/收端选择、0–4096 文档像素长度、独立尺寸/opacity 曲线与最低比例。共用曲线编辑器，端点固定，编辑自动 PCHIP，旧 linear 数据保持；横轴与最低值说明明确采用距离语义。窄参数列启用至少 200px 内容的横向滚动，扩大面板后完整显示，未改变其他分类的原宽度策略。

Windows Release 和生成头一致性检查通过。最小 Qt 五项通过（含初始化、清理共 7 passed，0 failed）：

- `taperModelPreservesClosedCurvesResetAndExchange`：启用/关闭、f64 长度、自定义曲线、独立最低比例、复制初始基准、重置、快照恢复和包导入基准；最高 state18、behavior15/capability、旧默认版本不变；九类非法范围/类型/字段/模型/端点/f32 重合及低版本夹带原子拒绝。
- `taperControlsCaptureWhitePreviewPixelsVbrAndReopen`：真实键盘输入长度、鼠标启用、编辑器加节点后 PCHIP、实际引擎预览非空且所有有 alpha 像素 RGB255；真实笔触途中关闭预设仍执行捕获版本，笔端 alpha 小于中央；VBR 导出并在新资源目录导入的真实完整图像相等，撤销重做相等，关闭效果则像素不同；关闭配置实际落入偏好并重启恢复，无 QML 警告。
- 既有 `smoothingControlsPreviewCaptureExchangeAndPreferencesReopen`、`wetModelPreservesClosedPolicyResetBaselineAndExchange`、`brushToolbarTransferTracksActivePresetForPaintAndErase` 回归通过。

布局修改后重跑起收笔真实完整链路及共用编辑器的平滑回归两项，通过（初始化清理共 4 passed）；截图人工检查扩大面板后长度、四向节点控制与两条曲线完整可读。窄列内容宽度下限有断言。日志 `artifacts/brush-taper-ui-{release.log,qt.txt,layout-qt.txt}`，截图 `artifacts/brush-taper-ui.png`，均为不入库本机产物。未重跑无关全量，未进行跨平台或真实硬件压力验收。

本阶段起收笔已具有用户可见入口。内核的单双流/双色/湿混/涂抹/选区/蒙版及失败回滚矩阵沿用前置 Document、IO、ABI 阶段证据，不将本轮普通笔触 Qt 用例冒充全部算子重验。完整笔刷系统仍有湿混双色/橡皮、非分隔双笔刷、高级 ABR/16 位与 VBR 分组及整体预设验收等后续范围。
