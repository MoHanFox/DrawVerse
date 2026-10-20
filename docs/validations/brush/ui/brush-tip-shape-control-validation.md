# 笔尖角度与圆度控制器验收

设计：[笔尖角度与圆度控制器](../../../brush/ui/brush-tip-shape-control-design.md)。

2026-10-21 更新：真实鼠标连续 20 次移动期间，方向盘显示达到 90°，核心参数仍为 0°，settingsChanged/selectedPreviewChanged 均为 0；释放后均恰好一次，参数为 90°。原角度、旋转圆度、边界、数值同步与主/第二笔尖独立两个用例在软件 Qt 和 D3D11 各通过，Release 构建通过。ABI1.74/state44 不变；本项减少完整预览请求，未声称实体持续输入性能已经验收。

2026-10-20，Windows Release，Qt 6.8.3。

- `qt_workflow_tests tipShapeDialDragsAngleAndRotatedRoundness`：真实鼠标拖动角度至 90°，沿旋转后的短轴改圆度，1% 下限与拖出圆盘恢复 100%，释放后移动不改值；外部参数更新、右侧输入双向同步与 0/101% 输入拒绝通过。
- `tipShapeSettingsShareControlAndPreserveIndependentParameters`：实际设置面板主/第二笔尖接入、参数独立、已有来源隐藏、导入小数不因显示被量化、snapshot/restore 保持通过。
- `brushScalarControlsCommitIntegerDisplayUnits`：原设置页整数参数回归通过；无窗口装配的旧 ScrollView `contentWidth` 绑定环警告仍出现，本项未修改该布局。
- 上述三项在软件后端通过；前两项在实际 D3D11 后端通过，硬件截图目视确认椭圆、短轴手柄、箭头和数值排布正确。
- `cmake --build build/qt --config Release --target drawverse qt_transform_tests qt_selection_tests qt_workflow_tests -- /m:1` 通过，产物 `build/qt/bin/Release/drawverse.exe`；附带构建不代表运行了变换/选区测试。生成头一致性检查通过。

没有改变 ABI 1.71、笔刷 state/behavior 或 VBR 格式，没有新增笔刷 GPU 计算。未跑全量、跨平台或实体数位笔测试。前一轮长任务/移动/选区/快捷键/历史/描边等事项不包含在此项完成结论中。
