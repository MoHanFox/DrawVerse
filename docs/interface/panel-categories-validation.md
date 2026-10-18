# 面板分组与统一内容验收（2026-10-10）

设计见 [panel-categories-design.md](panel-categories-design.md)。仅 Qt/QML 修改，Rust 和 C ABI 1.8.0 不变；布局 v6 增加可选水平 row 角色，旧的普通水平节点继续表示独立列。按用户后续要求保留历史每 20ms 逐条撤销/重做的跳转行为，此次仅删除历史面板底部按钮。

外侧左右边吸附完整列，内侧左右边把完整组与新面板组成并列组，垂直插入在整组之前/之后。拖动预览为整列窄边发光，包含图标列的空白高度；鼠标附近目标优先，窗口边缘接触为无指针目标时的后备。回归用“导航并列历史/颜色标签，图层为下方另一组”的真实结构验证树角色、最外侧与内侧识别、独立新列、整列从主窗拖出、Escape 取消、吸附另一列外侧、保存恢复，左右结构均不被摊成上下排列。

图标折叠把同组左右内容汇成一列图标；不同组之间留 8px 分隔槽，居中的 18px × 1px 横线为 #1B1C1F。同组任一图标打开整个左右/标签树，其他成员图标切换活动内容并置顶；当前活动图标再次点击收回。不同组允许同时打开，空白/失焦不关闭，宽高和偏移记住。双击展开窗顶部只关闭该窗，来源列仍为图标状态，其他组不受影响。测试读取真实 QML 项、操作鼠标与窗口，检查字号/调色板一致，历史内容没有 undo/redo 图标。

展开主窗、原生浮窗和侧边窗继续共用 PanelContent/PanelGroup；PanelWindow 统一浮窗调色板，避免默认 Controls 把按钮绘成白色。为避免 DockWorkspace→DockTile→PanelFlyout→DockWorkspace 的静态 QML 类型循环，图标列按需加载侧边窗，内部仍使用同一 DockWorkspace 和同一内容组件；活动画布属性采用实时绑定，切换文档不保留旧画布引用。

浮动列仅最上方拖动/标签栏背景 75% 不透明，下方标题/标签栏 #05080A 完全不透明；顶栏之后正文背板 #1C1E21 遮住全部空白。读取顶部 alpha、下方栏颜色及实际 grabWindow 像素，验证下方栏与靠近底部空白 alpha=255。侧边组和单面板浮窗顶部透明回归也保留。

导航只保留填满内容区的缩略图，删除尺寸、百分比、适配/实际像素按钮和下方空白。红框为画布视口映射到文档后裁剪的区域，平移、缩放、适配、多文档切换、独立文档浮窗均同步；仅视图变更不增加绘画历史。测试精确比较预览映射的框位置和大小，检查旧按钮消失。Windows 实窗截图 artifacts/panel-categories-native-final.main.png / .row.png 已人工检查，诊断图片不提交。

Qt 普通回归含生命周期 57 项、两倍 DPI 33 项，无失败/跳过；两项 CTest 共 51.18 秒。Windows 11 / Qt 6.8.3 / MSVC 实窗六个行为槽含生命周期 8 项通过，5100ms，覆盖鼠标拖动/取消、同组整窗、红框、顶部/下方像素和 Win32 SendInput 调整侧边窗尺寸；日志 artifacts/panel-categories-native-final.txt。跨窗坐标警告来自拖动时向目标窗口发送全局坐标的 Qt Test 路径，不影响行为断言。

完整 python tools/check.py 通过：Python 工具与真实 Git 检出、Rust fmt/clippy/工作区与文档测试、release、cbindgen 严格字节检查、C11/C++20 static/shared ABI、Qt 普通/两倍 DPI，6 项 CTest 无失败，共 52.45 秒。日志 artifacts/panel-categories-final-check.log 与 build/verify-ui/qt-ui-tests.txt / qt-ui-highdpi.txt。已重新构建 build/qt/bin/Release/drawverse.exe 并完成 deploy_qt，日志 artifacts/panel-categories-deploy.log。所有文本 LF/no-BOM，git diff --check 通过，构建输出不提交。macOS/Linux 原生透明与拖动仍需对应实机验证。
