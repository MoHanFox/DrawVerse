# 菜单底层圆角模糊与最大化拖动验收（2026-10-10）

设计见 [menu-blur-layer-design.md](menu-blur-layer-design.md)。仅修改 Qt/QML；Rust、ABI 1.8.0 与布局 v6 保持。

主 HWND 停用全窗口 Accent。独立、禁用输入、不激活、不进入任务栏的原生 HWND 只覆盖菜单高度，并始终紧随主窗处于其下方。Windows UI Composition HostBackdropBrush 提供实时背景模糊，RectangleClip 裁剪视觉的实际上方圆角，下面保持直角。没有桌面截图缓存。最小化命令到达前先隐藏背景 HWND，移动、缩放、DPI 和窗口状态变化同步尺寸/位置/裁剪，主窗销毁时释放视觉及辅助 HWND。要求 Windows 11 的 HostBackdropBrush 属性；初始化失败返回 false 并保留透明色材质，不回退为矩形 Accent。

失败复现：单独的 Accent 背景 HWND 即使设置了圆角 SetWindowRgn，圆角外的 2px 黑白格边缘强度仍从 765 降至 9；仅测纯色角点、QML 截图或区域形状均不能发现该泄漏。改为合成视觉裁剪后，圆角外边缘恢复为 765，与背景一致；菜单内条纹边缘从 270 降至 5，证明模糊仍存在。以上强度为相邻像素 RGB 差绝对值之和。

通过的原生回归：

- nativeMenuBlurLayerFollowsWindowAndHidesBeforeMinimize：辅助 HWND 样式、无所有者、位置、28px 菜单高度、上圆下直区域及 Z 顺序；普通/最大化各三轮最小化恢复、移动/缩放、隐藏/显示、关闭/开启模糊与销毁，无 iconic 背景窗口。
- nativeMenuBarBlursLiveBackdropWithoutMenuSnapshots：普通/最大化各三轮恢复，真实合成屏幕上的菜单模糊、圆角、红色关闭 hover，以及恢复前后菜单像素（最多两级通道舍入）；另各三轮恢复后还原正常状态，黑白格圆角外保持清晰。
- maximizedMainWindowIgnoresTitleBarDrag：最大化/全屏按下拖动及 SC_MOVE 均保持显示状态和几何；显式按钮/双击还原保留。
- 原有无边框恢复、快速再次最小化、灰黑分隔和新增橡皮擦共享预设的实窗回归一并通过。

Windows 11 / Qt 6.8.3 / MSVC 原生六槽全部通过（含初始化/清理 8 项，61869ms），日志 artifacts/menu-layer-native-final.txt。屏幕像素证据 artifacts/menu-layer-final.png.corner-source.png、.sharp-corner.png、.sharp.png、.blur.png、.title.png，已检查。诊断图片/日志不提交。

最终源码 `python tools/check.py` 全部通过：Python 工具与真实 Git 检出、Rust fmt/clippy/工作区和文档测试、release、cbindgen 严格字节检查、C11/C++20 static/shared ABI、Qt 普通和两倍 DPI。六项 CTest 全部通过，50.00 秒；Qt 普通 47 槽（含生命周期 49 项），两倍 DPI 25 槽（含生命周期 27 项）。日志 artifacts/menu-layer-full-check.log、build/verify-ui/qt-ui-tests.txt、build/verify-ui/qt-ui-highdpi.txt。

`cmake --build build/qt --config Release` 与 Qt 部署通过，程序为 build/qt/bin/Release/drawverse.exe，日志 artifacts/menu-layer-launch-build.log。文本 LF，git diff --check 通过。macOS/Linux 原生窗口尚需相应平台实机验证；本轮没有自动点击 Explorer 任务栏按钮，沿用原有系统最小化/恢复命令和任务栏能力回归。
