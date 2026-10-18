# 任务栏与原生窗口恢复（2026-10-10）

主窗口保持 QML 自绘标题栏与实时菜单高斯模糊，显式声明 WindowMinimizeButtonHint / WindowMaximizeButtonHint / WindowSystemMenuHint，使无边框 HWND 仍具有系统最小化、恢复能力。不得通过拦截任务栏点击模拟窗口隐藏，也不在最小化期间修改区域或几何。

仅对登记的主窗过滤 WM_NCCALCSIZE / WM_NCPAINT / WM_NCACTIVATE，整个窗口保留 QML 客户区，阻止 Windows 的 iconic 标题框写入透明菜单背后的合成表面。系统最小化、恢复、激活和尺寸消息继续由 Qt 处理；其他窗口不拦截，窗口销毁后登记失效，WorkspaceManager 销毁时撤销 native filter。

QML 单独保留普通窗口几何；最小化、最大化及其恢复过渡不覆盖此记录。从最大化还原为普通窗口时恢复记录，避免 Qt 无边框最大化的合成状态在系统恢复中把桌面尺寸当作普通尺寸。

几何补偿仅用于 Windows 原生后端；离屏和其他平台继续由 Qt 恢复窗口几何。

延迟恢复最大化前重新检查当前可见状态；用户已再次最小化时不执行旧的恢复请求，防止任务栏快速切换后窗口自行弹回。

验收需检查 Windows HWND 的 WS_MINIMIZEBOX / WS_MAXIMIZEBOX / WS_SYSMENU，正常窗口无 WS_CAPTION；系统命令与 QML 按钮均可最小化，恢复正常/最大化保持完整客户区及正确圆角。模糊启用后、重复最小化/恢复后，正常窗口上角的原生区域仍排除角点，并通过真实屏幕像素验证背景保留、菜单中部仍模糊。最大化仍覆盖可用桌面。

面板列与面板之间统一灰黑色 #2B2D31 的一像素分隔；鼠标经过分隔条仍可调整大小，不改变现有几何与布局 v6。Qt Test 校验主窗及组合浮窗分隔颜色和厚度。

修改仅 Qt/QML。执行普通与两倍 DPI Qt Test、Windows 实窗回归和完整 python tools/check.py，记录结果后提交并推送。

实现依据：[Qt 6.8.3 Windows 窗口标志转换](https://github.com/qt/qtbase/blob/v6.8.3/src/plugins/platforms/windows/qwindowswindow.cpp)、[WM_NCCALCSIZE](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-nccalcsize)、[WM_NCPAINT](https://learn.microsoft.com/en-us/windows/win32/gdi/wm-ncpaint)、[WM_NCACTIVATE](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-ncactivate)。
