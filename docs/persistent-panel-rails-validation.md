# 常驻图标面板列验收（2026-10-10）

设计见 [persistent-panel-rails-design.md](persistent-panel-rails-design.md)。仅修改 Qt/QML；核心、ABI 1.8.0 与布局 v6 不变。按本次用户说明，取代之前“全应用唯一临时窗、空白/失焦收回”的行为。

每个图标对应独立原生工具窗，内容只显示对应面板；同组/不同组可同时打开，改变来源活动标签不改变其他已打开面板。重复点击已打开图标调用 raise/requestActivate，Windows 原生 Z 顺序断言验证最近点击位于其他面板之前。空白和窗口失去激活保持打开，收回按钮与面板内 Escape 仅关闭该窗；图标拖出、展开列、移除来源或销毁工作区清理对应窗口。

图标列背景与空白统一 Theme.surface (#1C1E21)：布局为紧凑图标节点补充完整列背景，浮动列保留高度、允许调高。Qt 截图像素断言确认主窗附着列与独立列空白同色。短浮动列旁的展开面板不按列高裁剪；导航沿侧边锚定、移动父窗同步位置，正文底部可超出列底部。面板边框使用真实 startSystemResize，支持独立宽高与面板最小尺寸。Windows SendInput 回归在边框角区中心注入真实按下、移动和松开，验证宽高均改变而列高保持不变；等待边框布局后注入，分数 DPI 下避开窗口最外侧的透明读回边缘。

整个浮动列以独立列归位，保留自身展开/图标状态。菜单返回保留整个工具树；真实主区边缘拖动验证两种状态，保存/重开也保持。明确插入接收列时仍继承接收列状态；单个图标拖出为展开面板。原普通主窗/浮窗标签排序、面板菜单空白关闭保留。

回归覆盖：iconRailsSwitchSlideDismissAndDragPanels、floatingIconRailsKeepBackgroundSidePanelsAndReturnState、panelTabsSwapWithinTheirBarBeforeDetaching、panelColumnsUseOneHeaderAndDragTheActualWindow，以及原菜单和工具条回归。工具条浮动会重建控件，测试现在查询浮动后的新拖动柄，避免访问已销毁的旧控件。Qt 软件截图在分数 DPI 的右/下侧最多一个逻辑像素的未绘制读回边缘，也在原生 RGBA 窗口上裁除；菜单角形状/颜色及重开完整正文比较仍保留。

Windows 11 / Qt 6.8.3 / MSVC 六个实窗槽全部通过，含初始化/清理 8 项，4855ms；日志 artifacts/persistent-rails-native-final.txt。已检查 artifacts/floating-native.rail.png、.layers.png、.history.png 和 artifacts/persistent-native.main.png：短图标列背景一致，面板正文高于来源列且内容完整。诊断图片与日志不提交。

最终源码 `python tools/check.py` 完整通过：Python 工具与真实 Git 检出、Rust fmt/clippy/工作区和文档测试、release、cbindgen 严格字节检查、C11/C++20 static/shared ABI、Qt 普通与两倍 DPI。六项 CTest 全部通过，49.02 秒；Qt 普通含生命周期 50 项，高 DPI 28 项，无失败或跳过。日志 artifacts/persistent-rails-final-check.log、build/verify-ui/qt-ui-tests.txt、build/verify-ui/qt-ui-highdpi.txt。

当前用户正在运行 build/qt/bin/Release/drawverse.exe，链接器无法覆盖该文件。已将本轮完整验证构建的 build/verify-ui/bin/Release/drawverse.exe 复制为 build/qt/bin/Release/drawverse-panel-rails.exe，并执行 windeployqt 部署；SHA-256 两者相同。未关闭当前程序。部署日志 artifacts/persistent-rails-deploy.log，测试与部署程序使用同一实现资源。文本 LF/no-BOM，git diff --check 通过；macOS/Linux 原生操作仍需相应实机验证。
