# 工具列、稳定视口与交互验收（2026-10-10）

设计见 [workspace-tools-design.md](workspace-tools-design.md)。本轮完成用户 1–18 项界面反馈；仅 Qt/QML 修改，Rust/C ABI 1.8 保持，布局 v6 增加可选 toolWidth，笔刷设置 JSON v1 增加可选 toolRadii。历史每 20ms 逐条跳转保留。

工具列侧向停靠围绕整个面板列，测试构造图层/颜色上下两组，工具列高度覆盖两组。混合浮窗拆走面板后剩余工具窗立即缩回；正文宽度 36–72px，纯工具浮窗含边框 38–74px，能放大后缩回，高度可调，宽度保存恢复。停靠分隔线同样调整工具宽度。

拖动吸附只按鼠标到目标边距离 24 逻辑像素判定，窗口边接触不触发；回归明确让拖动窗边贴到目标、鼠标仍远离，检查目标不被选择。主窗边、独立列外侧和浮窗边统一显示独立非交互透明提示窗，窄高亮位于目标边外；检查四边几何、输入穿透和释放销毁，Ctrl 抑制/取消保留原有回归。折叠列展开按钮单击展开、按住拖动整列，Escape 取消后仍折叠，拖动松开不展开。

左右列展开/收起均保持文档原点场景坐标和缩放，新的画布范围只裁剪；不同原生窗口移动及文档切换保留原有行为。文档标题上圆角 10px、下平边，详情栏高 36px；实际 QML 和原生截图检查通过。画布偏移阴影已删除，画布外像素为 #17191C。主导航栏为 #05080A、75% 不透明，主窗最外 1px 描边 #63666B，内部描边 #1B1C1F。

画笔/橡皮各自记住大小，同一工具跨预设保留大小，真实笔触/擦除仍共享活动预设。大小修改不重新生成列表预览，所有预览使用固定中等大小；间距/不透明度仍按预设生成异步引擎预览，旧 token 被拒绝。两工具大小保存回读、旧设置兼容和错误数据验证通过。前景/第二色块单击交换、双击输入对应 HEX，双击不先交换；第二颜色由同一 PaintCoreClient 提供。

CapsLock 关闭显示实际工具大小×视图缩放的黑白双线笔刷轮廓，开启显示十字；移动/选区/空格的光标保持。点击笔刷后焦点仍在面板时，首次空格拖动画布平移 30×20px，撤销深度仍为 0；文字输入保留空格。顶部菜单采用独立窗口最高层，原生检查 WS_EX_TOPMOST 以及再次 raise 浮动面板后菜单仍在其上，反复打开没有残影；小数倍 DPR 的软件截图未绘制边缘按原有侧边菜单策略裁去，仍比较实际菜单像素。

原生标题栏单击/双击不提前进入系统移动，普通窗口越过阈值后才调用 startSystemMove，最大化拖动继续恢复普通尺寸并跟随鼠标。Windows 排队系统消息可能不给 nativeEventFilter 的 result 指针，已检查非空后写入。回归使用 PostMessage 验证队列过滤；同步 SendMessage 绕过全局过滤器、进入系统移动循环后的人工退出不计为通过。

最终验证：

- Windows 11 / Qt 6.8.3 / MSVC，原生 10 个行为槽（含生命周期 12 项）全部通过，17036ms，无失败、跳过或 QML 警告；日志 artifacts/workspace-tools-native-final.txt。包括工具整列/缩回/保存、按钮拖动/像素固定、颜色/轮廓/空格、菜单置顶/像素、四边提示/鼠标判定、最大化恢复跟随、无边框恢复、底层模糊 HWND 和真实桌面圆角模糊像素。
- 普通/最大化各多轮最小化恢复、移动/缩放、模糊开关和销毁，最底层圆角外的黑白格保持清晰，菜单内部仍真实模糊；本轮颜色和外描边更改后原生回归继续通过。
- 完整 python tools/check.py 全部通过：Python 工具/真实 Git 检出、Rust fmt/clippy/工作区及文档测试、release、cbindgen 严格字节检查、C11/C++20 static/shared ABI、Qt 普通和两倍 DPI。6 项 CTest 无失败，共 56.38 秒；Qt 普通含生命周期 61 项、两倍 DPI 37 项。日志 artifacts/workspace-tools-final-check.log、build/verify-ui/qt-ui-tests.txt / qt-ui-highdpi.txt。
- cmake --build build/qt --config Release --target deploy_qt 通过；程序 build/qt/bin/Release/drawverse.exe 已更新，日志 artifacts/workspace-tools-deploy.log。
- 原生截图 artifacts/workspace-tools-native-preview.main.png 已检查：工具列从顶到底，半圆角标题、较高详情栏、无画布阴影及外描边显示正确；诊断图/日志不提交。文本 LF/no-BOM、git diff --check 通过。

Qt 6.8 的独立 Popup.Window 路径经过上述实测；Qt 6.5–6.7 提供 SideMenuWindow 下方锚定回退，本机没有该版本 SDK，未做运行验证。macOS/Linux 原生透明与层序仍需各自实机验证。
