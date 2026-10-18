# 菜单底层圆角模糊与最大化拖动验收（2026-10-10）

设计见 [menu-blur-layer-design.md](menu-blur-layer-design.md)。仅修改 Qt/QML；Rust、ABI 1.8.0 与布局 v6 保持。

主 HWND 停用全窗口 Accent。独立、禁用输入、不激活、不进入任务栏的原生 HWND 只覆盖菜单高度，并始终紧随主窗处于其下方。Windows UI Composition HostBackdropBrush 提供实时背景模糊，RectangleClip 裁剪视觉的实际上方圆角，下面保持直角。没有桌面截图缓存。最小化命令到达前先隐藏背景 HWND，移动、缩放、DPI 和窗口状态变化同步尺寸/位置/裁剪，主窗销毁时释放视觉及辅助 HWND。要求 Windows 11 的 HostBackdropBrush 属性；初始化失败返回 false 并保留透明色材质，不回退为矩形 Accent。

失败复现：单独的 Accent 背景 HWND 即使设置了圆角 SetWindowRgn，圆角外的 2px 黑白格边缘强度仍从 765 降至 9；仅测纯色角点、QML 截图或区域形状均不能发现该泄漏。改为合成视觉裁剪后，圆角外边缘恢复为 765，与背景一致；菜单内条纹边缘从 270 降至 5，证明模糊仍存在。以上强度为相邻像素 RGB 差绝对值之和。

通过的原生回归：

- nativeMenuBlurLayerFollowsWindowAndHidesBeforeMinimize：辅助 HWND 样式、无所有者、位置、28px 菜单高度、上圆下直区域及 Z 顺序；普通/最大化各三轮最小化恢复、移动/缩放、隐藏/显示、关闭/开启模糊与销毁，无 iconic 背景窗口。
- nativeMenuBarBlursLiveBackdropWithoutMenuSnapshots：普通/最大化各三轮恢复，真实合成屏幕上的菜单模糊、圆角、红色关闭 hover，以及恢复前后菜单像素（最多两级通道舍入）；另各三轮恢复后还原正常状态，黑白格圆角外保持清晰。
- 初版 maximizedMainWindowIgnoresTitleBarDrag：当时验证最大化/全屏拖动保持状态；用户后续澄清应恢复普通窗口并跟随鼠标，此断言已由下方新回归替代。
- 原有无边框恢复、快速再次最小化、灰黑分隔和新增橡皮擦共享预设的实窗回归一并通过。

Windows 11 / Qt 6.8.3 / MSVC 原生六槽全部通过（含初始化/清理 8 项，61869ms），日志 artifacts/menu-layer-native-final.txt。屏幕像素证据 artifacts/menu-layer-final.png.corner-source.png、.sharp-corner.png、.sharp.png、.blur.png、.title.png，已检查。诊断图片/日志不提交。

最终源码 `python tools/check.py` 全部通过：Python 工具与真实 Git 检出、Rust fmt/clippy/工作区和文档测试、release、cbindgen 严格字节检查、C11/C++20 static/shared ABI、Qt 普通和两倍 DPI。六项 CTest 全部通过，50.00 秒；Qt 普通 47 槽（含生命周期 49 项），两倍 DPI 25 槽（含生命周期 27 项）。日志 artifacts/menu-layer-full-check.log、build/verify-ui/qt-ui-tests.txt、build/verify-ui/qt-ui-highdpi.txt。

`cmake --build build/qt --config Release` 与 Qt 部署通过，程序为 build/qt/bin/Release/drawverse.exe，日志 artifacts/menu-layer-launch-build.log。文本 LF，git diff --check 通过。macOS/Linux 原生窗口尚需相应平台实机验证；本轮没有自动点击 Explorer 任务栏按钮，沿用原有系统最小化/恢复命令和任务栏能力回归。

## 用户澄清后的最大化拖动（2026-10-10）

上述初版“保持最大化”的行为已替换。最大化菜单空白区按下仍保持最大化，移动越过系统拖动阈值后恢复此前普通窗口尺寸；按标题栏按下点的水平比例和垂直偏移对齐当前鼠标，沿用同一个 MouseArea 抓取持续移动，松开/取消结束。恢复当下同步主窗和模糊视觉的圆角。普通窗口沿用系统移动，全屏仍不移动，显式还原和双击保留。

maximizedTitleBarDragRestoresAndFollowsPointer 在普通/两倍 DPI 验证两处标题位置、阈值内保持最大化、越过阈值恢复尺寸与鼠标锚点、后续位移连续跟随、松开停止、最小化后仍恢复普通几何及双击/全屏行为。两倍 DPI 离屏显示面积小于应用最小宽度时，使用 Logo 左侧真实空白拖动区，避免误点击菜单项。

Windows 原生测试还使用 SendInput 的真实按下/松开与 SetCursorPos 移动，确认上述恢复和跟随行为，主 HWND 与模糊 HWND 均恢复圆角且位置同步。失败退出也会释放测试注入的鼠标按键。原有普通/最大化最小化恢复、菜单实时模糊与黑白格圆角外像素回归一并通过：原生三槽，含生命周期 5 项，54375ms；日志 artifacts/maximized-drag-native-final.txt。

最终 tools/check.py 全部通过，六项 CTest 47.58 秒；普通 47 槽（含生命周期 49 项），两倍 DPI 25 槽（含生命周期 27 项），无跳过或失败。日志 artifacts/maximized-drag-full-check.log。启动程序与 Qt 部署重新构建通过，日志 artifacts/maximized-drag-launch-build.log；程序仍为 build/qt/bin/Release/drawverse.exe。Rust/C ABI 和模糊实现保持不变，文本 LF/no-BOM、git diff --check 通过。
