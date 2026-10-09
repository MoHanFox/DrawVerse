# 窗口恢复 / 边缘吸附 / 标签排序验收（2026-10-09）

设计见 [window-recovery-design.md](window-recovery-design.md)。仅修改 Qt/C++/QML，Rust 和 ABI 1.8.0 未修改，布局继续使用 v6。

任务栏能力与恢复后的原生标题框残留后续修复见 [taskbar-window-validation.md](taskbar-window-validation.md)，新增真实合成菜单像素比较。

## 实现结果

主窗跳过最小化/隐藏时的圆角裁剪，C++ 同时检查 IsIconic，避免对 Windows 的最小化小矩形设置区域。QML 记录最小化前的显示状态并恢复最大化，解决 Qt 无边框最大化的原生恢复丢失状态。文档与工具浮窗最小化期间不记录临时几何。

图标临时窗全应用唯一；替换时立即隐藏旧窗，关闭旧窗不清除新窗登记。鼠标、触控和数位笔外部按下关闭并消费输入；Escape 和失焦关闭。失焦延迟回调用窗口身份及显示修订保护新打开/重新激活的临时窗。新窗主动激活，跨列/同列切换仍可用。

工具面板四边与主工作区左右边缘统一 24px 内外容差，选择最近可见边；拖动窗边界参与计算，排除源宿主和同一拖动列。Windows 按本应用原生窗口的前后次序排除被其他本应用浮窗遮挡的目标。只有标题标签区接收标签合并，正文不再默认合并。DockEdgePreview 统一显示窄边发光，删除大面积覆盖和中央文字提示。普通拖放也使用就近边规则；同组标签排序保留。

单画布浮窗窗口背景、标题、最小化及关闭按钮共用 #202226，关闭仅 hover/按下变红。图标列改用 #1c1e21 面板表面色，图标文字色保持区分；删除工具条拖动说明。参考图布局 API 与启动自动应用已删除，菜单只保留“默认布局”，新用户使用双列停靠；保存的用户布局仍恢复。

标签在当前栏内拖动时显示目标下沿，在松手时交换源/目标位置并保留活动面板；8px 容差内不创建浮窗，越出标签栏才拆出。普通面板和图标临时窗均支持，顺序经布局 v6 持久化。

## 回归与视觉

mainWindowRestoresWithoutNativeFrameOrClippedContent：普通/最大化各三轮系统命令或 UI 按钮最小化、原生 SW_RESTORE 恢复；校验显示状态、几何、Win32 客户区/窗口尺寸一致、区域中心可见、完整渲染尺寸与不透明像素、画布仍存在。

iconRailsSwitchSlideDismissAndDragPanels：同列及跨列替换保持唯一临时窗、历史内容切换、侧边滑动、鼠标/触控/数位笔关闭不绘画；Windows SendInput 实际点击画布空白关闭；失焦、Escape、反复打开、图标直接拖出及继承状态。

draggingPanelsSelectsNearestExposedEdgeAndShowsEdgeGlow 与原有列/空工作区交互：实际窗口拖动在四边内外范围选择对应目标，正文无合并，发光模式和最终停靠一致；自身/整列排除，画布独占和无画布归位仍通过。

panelTabsSwapWithinTheirBarBeforeDetaching：普通面板及临时面板内横向拖动交换标签，不创建浮窗，活动页不丢失；保存恢复保持顺序；离开标签栏真正拆出，Escape 还原。defaultLayoutRestoresTwoDockedColumnsAndMenuEntry：菜单无两个旧名称，默认命令恢复两列并保存恢复。多文档原有测试增加单画布标题背景一致校验。菜单残影检查比较内部全部文字/背景像素，排除 1.1x 缩放最外侧的一个像素覆盖边界，保留不透明度及连续重开检查。

Windows 11 / Qt 6.8.3 / MSVC 实窗最终九个测试槽全通过（含初始化/清理 11 项，57124ms），含原生模糊与圆角验收，无 QML 警告。日志 artifacts/window-recovery-native-final.txt。截图已检查：artifacts/window-recovery-edge.png、window-recovery-rail.main.png、window-recovery-rail.history.png、window-recovery-documents.png.single.png。原生跨窗口 Qt Test 坐标提示属于测试鼠标离开源窗。截图与日志不提交。

## 完整验证

最终源码 `python tools/check.py` 全部通过：Python 工具/真实 Git 检出、Rust fmt/clippy/工作区与文档测试、release 构建、cbindgen 头严格字节检查、C11/C++20 静态/动态 ABI，Qt 普通与两倍 DPI。六项 CTest 全通过，44.44 秒；普通 43 个测试槽（含生命周期 45 项），两倍 DPI 21 个测试槽（含生命周期 23 项）。日志 artifacts/window-recovery-full-check.log、build/verify-ui/qt-ui-tests.txt、build/verify-ui/qt-ui-highdpi.txt。

`cmake --build build/qt --config Release` 与 Qt 部署通过，日志 artifacts/window-recovery-launch-build.log；可直接运行 build/qt/bin/Release/drawverse.exe。修改文本均为 LF，git diff --check 通过。核心/ABI 与构建输出未提交。macOS/Linux 原生窗口及真实数位板硬件仍需相应平台验收。
