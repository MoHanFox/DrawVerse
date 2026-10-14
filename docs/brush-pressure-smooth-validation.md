# 自动平滑压力曲线验收

2026-10-14，ABI1.45/state13/VBR1.11/behavior10，见brush-pressure-smooth-design.md。尺寸、流量、不透明度及第二笔刷使用同一PCHIP算法；编辑器增加/移动/删除点或改最低比例时显式保存smooth-pchip-v1，旧未编辑曲线仍线性，默认两个点不变。UI采样由PaintCoreClient经纯C ABI读取，实际笔触精确分段求值；非单调节点允许，但局部无过冲。完整关闭配置、预设副本和重置基准沿统一模型保存。

最小证据：
- paint-core brush_pressure 7项通过：旧5项真实写入兼容，新增节点/C1/局部范围/极窄区间/样本预算，以及四角色主尺寸/流量/不透明度/第二尺寸真实像素对参考控制与历史。
- paint-io brush_pressure 7项通过：旧state/VBR不升级及平滑state13/VBR1.11精确恢复与真实像素；state12/旧包minor/错误behavior/capability/未知或null插值拒绝，Schema版本/字段断言。
- paint-ffi native_samples通过：实际核心值、NULL/错位/超长/模型/节点错误与失败清零。
- Qt pressureCurveEditorPersistsAndExchangesNativeCurves在DPR1/2通过：实际编辑器样本、自动模型、关闭保存/导入导出、真实预览及0.25压力的平滑不透明度像素/在途捕获。DPR2另有双笔刷完整状态/真实资源像素/重置基准/涂抹与颜色混合相关8项通过（不含init/cleanup）。
- Release Qt与生成头检查、C/C++静态/动态ABI4项通过，smoke调用新纯采样入口。截图artifacts/pressure-smooth-ui-1.png、-2.png及brush-reset-bottom-1.png、-2.png人工检查；重置位于参数末尾，固定预览上方，窄列限制按钮最小宽度避免压迫参数。

paint-core/paint-io/paint-ffi全targets Clippy（-D warnings）通过。

只验证本阶段相关范围，不宣称全仓tools/check.py完整通过。湿混、路径平滑IO/Qt、双色自动描边和高级ABR仍待后续。
