# 工具详细栏传递参数验收

2026-10-16，Windows 11 / Qt 6.8.3 / Release，software Quick，环境倍率 1。对应 [设计](brush-toolbar-transfer-design.md)。

画笔和橡皮的工具详细栏直接调整活动 BrushLibrary 的不透明度、流量及各自独立压感，复用通用 CompactSlider / CompactCheckBox。移动工具隐藏这些传递控件。涂抹图标后仅保留 1px 分隔线，笔刷仍由画笔面板选择，既有独立强度与压感不变。无 ABI 或存储格式变化。

Windows Release 构建通过，包含既有生成头一致性检查。最小 Qt 三项通过（含初始化、清理共 5 passed，0 failed）：

- `brushToolbarTransferTracksActivePresetForPaintAndErase`：画笔及橡皮各自通过真实键盘操作两个滑块、鼠标点击压感开关更新当前预设；反向修改与切换预设后数值、开关同步，关闭压感保留曲线；快照保存恢复保持独立参数，无 QML 警告。
- `smudgeSharedBrushParametersInputAndDelayedHelp`：下拉框移除、1px 分隔线可见，通过共用模型选择笔刷后仍为涂抹工具；尺寸/间隔、强度、复选框、一秒注释、真实鼠标和数位笔绘画及历史回归保持。
- `brushPresetClicksKeepEraserAndUseSelectedPresetSize`：从画笔面板选择笔刷不退出橡皮，当前预设参数及真实绘画/擦除像素与历史保持。

日志为 `artifacts/brush-toolbar-transfer-release.log`、`artifacts/brush-toolbar-transfer-qt.txt`，本机产物不入库。未重跑无关全量，未进行跨平台验收。
