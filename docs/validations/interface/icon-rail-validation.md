# 图标列与画布标题修复验收（2026-10-09）

设计见 icon-rail-design.md。本次仅修改 C++ / QML UI，Rust 和 ABI 1.8.0 未修改，布局继续使用 v6。

## 结果

主工作区边缘在目标判定中优先于浮动工具窗，解决其他浮窗矩形抢占归位目标的问题。左右 36px 接收带及外侧 12px 容差持续显示预览；仅画布和无文档时都可以把真实浮窗拖回主窗口。画布主体仍不接收工具，文档留空区保留。

只保留图标折叠。上下插入的整组、单个标签拆分、新建自定义面板、返回工作区与布局恢复都保持接收列的图标状态。旧布局标签折叠转为图标，并补全整列状态。删除小导航栏和菜单里的“折叠为标签 / 展开面板组”。图标可拖出为展开的原生工具面板，来源仍是图标列；列顶部仍可整体拖动。

图标临时窗从当前组状态读取活动面板；历史、图层和导航点击可切换，内部标签切换后再次点击图标也保持一致。再次点击活动图标或 Escape 关闭；点击主画布空白关闭并消费点击，不产生笔触。临时窗沿来源列侧边上下滑动，横坐标和屏幕/主工作区边界保持约束；展开列或开始实际拖动时销毁临时窗。

移除文档条的新建加号，文件菜单新建入口保留。单画布浮窗只有一行标题；多个文档组合后增加窗口标题行，标题随此浮窗活动文档及修改状态变化。文档从多项减少到一项恢复单行；TextMetrics 测量标签，不产生宽度绑定循环。

## Qt 回归与视觉证据

iconRailsSwitchSlideDismissAndDragPanels 覆盖整组和单标签加入图标列、自定义面板、同一临时窗图层/历史切换、内部标签切换、侧边滑动、空白/Escape/重复图标关闭、图标真正拖出与归位、持久化和旧标签状态迁移。

panelsReturnToCanvasOnlyAndEmptyWorkspaceEdges 用真实浮窗鼠标抓取在仅画布主区左边缘归位，再关闭全部文档并在右边缘归位，保留大于 320px 的空文档区域。floatingCanvasAndToolsDockAcrossNativeWindows 补充单/多画布标题、活动窗口名、移除加号和删除其他标签后恢复单行，并保留原有独立绘画/视口/历史/ORA/关闭任务验收。

Windows 11 / Qt 6.8.3 / MSVC 实窗三个测试槽全部通过（含初始化/清理共 5 项），QML 警告为空。日志 artifacts/icon-rail-native.txt。截图已检查：artifacts/icon-rail-native.main.png、artifacts/icon-rail-native.history.png、artifacts/icon-rail-documents.png.single.png、artifacts/icon-rail-documents.png。跨窗口 Qt Test 坐标警告表示鼠标离开源窗口，不是产品 QML 警告。截图和构建产物不提交。

最终源码 `python tools/check.py` 完整通过：Python 真实 Git 检出与工具测试、Rust fmt/clippy/工作区与文档测试、Release 构建、cbindgen 头严格字节检查、C11/C++20 静态/动态 ABI、Qt 普通与两倍 DPI 六项 CTest 全通过（40.52 秒）。普通 Qt 40 个测试槽（含生命周期共 42），两倍 DPI 18 个测试槽（含生命周期共 20）；两倍 DPI 根据可用屏幕方向拖动，并验证临时窗在主工作区与屏幕交集内。日志 artifacts/icon-rail-final-check.log、build/verify-ui/qt-ui-tests.txt、build/verify-ui/qt-ui-highdpi.txt。

最终源码 Windows 实窗再次通过三个交互测试槽（含生命周期共 5，5004ms），以上截图已重新生成检查。`cmake --build build/qt --config Release` 及 Qt 部署通过，日志 artifacts/icon-rail-launch-build.log；可运行 build/qt/bin/Release/drawverse.exe。`git diff --check` 通过；全部修改及新增文本为 LF；构建输出不提交。

macOS/Linux 原生窗口与真实数位板硬件仍待对应平台验收。布局不自动保存或重开文档像素，其他未实现的核心模块与 Python 插件范围不变。
