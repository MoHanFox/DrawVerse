# 自由停靠验收（2026-10-09）

设计见 flexible-docking-design.md。移除固定 DockColumn 和独立 ToolStripWindow：主窗/原生浮窗都按有内容的分割树渲染，画布、工具条和面板组共用边缘停靠，空分支自动收缩。拖动工具条顶部握柄可放在工作区或任意面板左右侧；拖动画布文档标签可浮动，双击标签可浮动/归位。布局 v5 保存实际树和浮窗，兼容转换 v1–v4。

## 回归证据

- `python tools/check.py` 完整通过，日志 `artifacts/flexible-dock-final-check.log`；包含真实 Git 检出测试、Rust fmt/clippy/测试、生成头字节校验、C11/C++20 static/shared 四组 ABI 和全部 Qt 检查。37 项 Qt 测试、15 项两倍 DPI 测试通过（不计初始化/清理），六个 CTest 目标无失败。
- `flexibleDockTreeSupportsFourEdgesAndValidatesPersistence` 覆盖全部四边、工具条在各面板左右侧/画布右侧、浮窗混合树、移走最后一组后无空栏，以及 v5 恢复、损坏布局原子拒绝和 v4 迁移。原浮窗根叶再次独立拖出时使用新窗口 ID，原窗口中的其他叶不丢失。
- `floatingCanvasAndToolsDockAcrossNativeWindows` 在真实 QML 中双击浮动画布，通过原生 DragEnter/Move/DropEvent 将颜色面板、工具条和画布组合、再次拆分/合并。始终保留同一 CanvasItem 和已有窗口/控件；鼠标/数位板压感绘画、撤销/重做、缩放、平移、分隔条调整、切换工具、选区和关闭组合浮窗后归位均通过。
- Windows 实窗三个定向测试全部通过：上述浮动画布回归、菜单栏实时高斯背景与原生圆角、菜单反复开关无鬼影及浮窗侧面菜单。日志 `artifacts/flexible-dock-final-windows.txt`，无 QML 警告。实窗截图 `artifacts/flexible-dock-final-windows.png` 已目视检查：颜色面板左侧、画布中间、工具条右侧，棋盘格/鼠标和压感笔迹正常；菜单截图 `artifacts/flexible-dock-final-menu.png` 无残留重叠。

测试等待 QML 几何布局完成后才测量分隔条，先激活原生窗口，再测试悬停/键盘；菜单测试窗口限制在可用屏幕内，防止桌面光标无法到达屏幕外的标题按钮。这些前置条件适用于真实窗口和高 DPI 测试，不改变应用绘画或按钮逻辑。

常用启动目录 `build/qt/bin/Release/drawverse.exe` 已同步构建。代码修改限于 C++/QML UI 与对应文档/Qt Test；Rust、C ABI、文档格式和绘画引擎未改。Windows 本机及普通/两倍 DPI 验证完成，macOS/Linux 原生窗口仍需对应平台验证。
