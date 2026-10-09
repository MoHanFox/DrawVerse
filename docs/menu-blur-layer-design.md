# 菜单底层模糊与最大化拖动（2026-10-10）

上一版主 HWND 全窗口 Accent 模糊依赖窗口区域裁剪，仍可能在移动、最小化和恢复期间遗留矩形合成范围。本次移除主 HWND 的 Accent 模糊，使用独立原生背景 HWND，仅覆盖菜单栏的物理尺寸，处于主窗之后，不激活、不接收输入、不进入任务栏、无所有者以免 Windows 强制置于主窗之上。Qt/QML 仍负责全部可交互内容。

只给独立 Accent HWND 设置窗口区域仍不能裁剪底层模糊，黑白格实测会被平滑。因此本轮使用 Windows UI Composition 的 HostBackdropBrush、DesktopWindowTarget 与 RectangleClip，对实际背景视觉设置上方圆角、下方直角；辅助 HWND 使用 NOREDIRECTIONBITMAP，不设置 Accent。尺寸/位置/DPI/最大化状态变化时更新视觉与原生区域。主窗最小化、隐藏、销毁时隐藏或销毁底层窗口，不让它参加主窗的 iconic/maximize 状态切换。只有正常状态使用 10px 上角，最大化直角。关闭模糊时移除底层窗口显示。使用 Windows SDK 的 C++/WinRT、windowsapp、CoreMessaging 与 dwmapi，无外部依赖或截图缓存；Qt 提供 STA 与消息循环，UI 线程共用一个 DispatcherQueue。

原生 HostBackdropBrush 能力由 DWMWA_USE_HOSTBACKDROPBRUSH 启用，要求支持该属性的 Windows 11。初始化失败返回 false，保持现有透明色材质降级，不以矩形 Accent 冒充成功；Windows 外后端保持现有降级行为。Rust/C ABI 与跨平台 QML 内容保持不变。

最大化/全屏时菜单栏按下和系统移动命令不启动移动，拖动保持最大化；正常窗口仍可拖动，双击与按钮继续显式最大化/还原。

验收：原生测试确认主 HWND 没有全窗口 Accent 模糊，底层 HWND 的位置、菜单高度、圆角区域、任务栏/输入样式和 Z 顺序；普通/最大化反复最小化恢复，移动与缩放，模糊图层不遗留在旧矩形范围或工作区。使用靠近角点的高对比细图案检查原生底层圆角外的真实屏幕像素，不能只检查 QML 画层或纯色角点。最大化按下拖动与系统 SC_MOVE 均保持状态及几何。完整 tools/check.py 与普通/两倍 DPI Qt Test、Windows 实窗后提交并推送。
