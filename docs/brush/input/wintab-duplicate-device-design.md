# WinTab 与 Windows Ink 双设备流隔离

2026-10-22，用户反馈 WinTab 模式下悬停笔尖轮廓持续闪烁，落笔后一直保持绘制状态且只能按 Esc 退出。

Qt 6.8.3 的 Windows 后端可以同时收到两套数位笔事件：WinTab 支持创建的设备名为 `wintab`，Windows Pointer 处理器创建的设备名为 `wmpointer`。启用 WinTab 不会关闭后者的 `WM_POINTER` 输入，因此同一物理笔可能交替送来两套 Hover、Press、Move、Release 与 proximity 事件。

画布只接受当前后端对应的设备流。活动后端为 `wintab` 时，`CanvasItem` 在应用事件过滤阶段消费 `wmpointer` 的 TabletPress/Move/Release 和 Enter/LeaveProximity；其他平台与 Windows Ink 路径保持原行为。过滤发生在 Quick、菜单与变换路由之前，避免同一物理笔被两个设备重复解释。

笔触本地状态也改为幂等：已接受的 Press 遇到重复 Press 不清除 `m_stroke/m_tablet`；Release 和 proximity loss 以 `PaintCoreClient::drawing()` 为准结束已接受笔触。即使驱动仍产生重复 Press，后续 Release 也能收笔，不能留下永久绘制态。

本修复不改变输入坐标、压力映射、绘画算法、ABI、state 或文件格式。


## 2026-10-26 普通按钮与菜单的重复点击排查

用户反馈手绘笔单击像素加减变动2px，顶部菜单打开后立即关闭。验收使用真实Qt窗口及平台输入入口覆盖像素SpinBox和顶栏菜单，同时检查单路Tablet、双设备Tablet、伴随鼠标及快速独立点击。按事件来源/设备所有权只让一套输入驱动交互，不使用时间防抖；原生鼠标、触摸、控件长按、菜单选项、画布压力与释放不得被误拦。只运行与此次问题相关的最小输入回归，并更新Release交付；实体驱动行为与注入测试区分记录。


WinTab路径的补充边界：Windows笔生成的WM_LBUTTON等客户端鼠标消息可伴随Tablet输入。仅在WinTab实际启用时，通过原生事件过滤识别MI_WP_SIGNATURE=0xff515700且非touch的消息，在Qt/菜单路由前消费；普通鼠标、touch标记、滚轮/非客户区消息及Windows Ink路径保持。Qt依据最近WM_POINTER类型决定是否过滤，但WinTab输入不一定更新该类型，不能依赖QMouseEvent.source（可能仍标NotSynthesized）。不按时间/位置推断，不拦截Qt在Tablet未处理时生成的控件鼠标回退。


过滤器在配置后端时安装一次，跟随应用销毁，仅接收windows_generic_MSG且hwnd属于已有QQuickWindow；原生文件对话框等非Quick窗口保持系统输入。Windows Ink后端不应用此WinTab补丁。真实鼠标以无笔标记的SendInput检验，笔伴随消息以dwExtraInfo保留标记进入原生队列，不通过SendMessage绕过Qt dispatcher。参考Qt6.8.3的[qwindowspointerhandler.cpp](https://github.com/qt/qtbase/blob/v6.8.3/src/plugins/platforms/windows/qwindowspointerhandler.cpp)鼠标来源处理与[qwindowscontext.cpp](https://github.com/qt/qtbase/blob/v6.8.3/src/plugins/platforms/windows/qwindowscontext.cpp)原生过滤调用位置。
