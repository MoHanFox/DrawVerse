# 任务栏与原生窗口恢复验收（2026-10-10）

设计见 [taskbar-window-design.md](taskbar-window-design.md)。仅修改 Qt/QML，ABI 1.8.0 与布局 v6 保持。

主窗显式声明系统最小化/最大化/系统菜单能力；Window | FramelessWindowHint 单独组合不会得到 Qt Windows 后端的默认最小化标志。保留 QML 标题栏，通过主窗 HWND 限定的 native filter 阻止非客户区标题框绘制，其他窗口与系统状态消息仍由 Qt 处理。

Windows 原生后端单独保存普通窗口位置和尺寸，最大化/最小化过渡不覆盖记录；离屏和其他后端保留 Qt 自身几何恢复行为。最大化延迟恢复检查当前显示状态，快速恢复后再次最小化不会被旧请求打开。

旧代码新增回归先失败于 WS_MINIMIZEBOX 缺失；仅补标志后，原生截图继续复现用户报告的小标题框覆盖菜单，QQuickWindow::grabWindow 和 WS_CAPTION 检查均不能发现它。加入非客户区绘制过滤后，恢复前后真实合成截图的菜单区域一致。DWM 对文字混合偶有单通道一级舍入，回归允许各通道最多两级差异，保留全部区域像素检查。

mainWindowRestoresWithoutNativeFrameOrClippedContent：普通/最大化各三轮系统命令或 QML 按钮最小化，SC_RESTORE 恢复；系统窗口能力、iconic 窗口移出工作区、正常窗口无 caption、完整客户区与内容、几何和圆角区域。恢复后从最大化还原普通窗口仍回到原位置/尺寸；另检查恢复与再次最小化连续到达时保持 iconic。

nativeMenuBarBlursLiveBackdropWithoutMenuSnapshots：普通/最大化各三轮真实原生恢复；菜单中部保持实时模糊，正常上角保留外部背景，最大化无圆角；同一显示状态下恢复前后左侧菜单全部像素比较，直接覆盖系统标题框残留。每种状态单独采样基线，避免把不同圆角/窗口尺寸下的 DWM 模糊核差异作为残影。

dockSeparatorsUseThinDarkGrayAcrossMainAndFloatingColumns：主窗面板列及组合工具浮窗的分隔统一一像素 #2B2D31，普通边框仍采用 Theme.line。菜单测试先移开鼠标再验证默认关闭按钮颜色，避免继承前一个实窗测试的 hover。

Windows 11 / Qt 6.8.3 / MSVC 原生三槽通过（含生命周期 5 项，14956ms）：恢复与快速再次最小化、灰黑分隔、普通/最大化重复恢复后的真实合成模糊/圆角/菜单像素。日志 artifacts/taskbar-native-final.txt。另有菜单与图标临时面板实窗二槽通过（含生命周期 4 项，42382ms），日志 artifacts/taskbar-native-interaction.txt。截图 artifacts/taskbar-final.png.title.png 已检查，普通窗口上角正确、实时模糊保留、关闭 hover 红色，未出现原生标题残留。截图和日志不提交。

最终源码 `python tools/check.py` 全部通过：Python 工具与真实 Git 检出、Rust fmt/clippy/工作区及文档测试、release、cbindgen 头严格字节检查、C11/C++20 静态/动态 ABI、Qt 普通与两倍 DPI。六项 CTest 全通过，45.28 秒；普通 44 个测试槽（含生命周期 46 项），两倍 DPI 22 个测试槽（含生命周期 24 项）。日志 artifacts/taskbar-all-checks.log、build/verify-ui/qt-ui-tests.txt、build/verify-ui/qt-ui-highdpi.txt。

`cmake --build build/qt --config Release` 与 Qt 部署通过，日志 artifacts/taskbar-launch-build.log，更新的程序为 build/qt/bin/Release/drawverse.exe。修改文本保持 LF，git diff --check 通过。原生任务栏能力与系统最小化/恢复命令路径均回归；未自动点击 Explorer 的任务栏按钮。macOS/Linux 原生窗口尚需相应平台实机验收。
