# 资源列表独占区域内滚轮

2026-10-21。设置页 Flickable 内嵌资源 GridView，快速滚到边缘时 Qt 默认滚轮处理允许事件传播，导致外层设置页跟着移动。

共享 BrushResourcePicker 的网格使用 WheelHandler 消费区域内所有滚轮事件，停止旧惯性并按 pixelDelta 或 angleDelta 更新有界 contentY。到达首尾、短列表无可滚内容或触摸板动量阶段也消费事件。网格外仍交给外层；鼠标拖动、资源点击与滚动条保持 Qt 原有处理。主笔尖、纹理、第二笔尖和描边资源共用，不引入 ABI 或 GPU 变化。

验证入口 `qt_workflow_tests::resourceGridConsumesFastWheelAtBothEdges`；见 [记录](../../validations/brush/ui/brush-resource-wheel-validation.md)。
