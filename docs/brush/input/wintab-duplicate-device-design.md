# WinTab 与 Windows Ink 双设备流隔离

2026-10-22，用户反馈 WinTab 模式下悬停笔尖轮廓持续闪烁，落笔后一直保持绘制状态且只能按 Esc 退出。

Qt 6.8.3 的 Windows 后端可以同时收到两套数位笔事件：WinTab 支持创建的设备名为 `wintab`，Windows Pointer 处理器创建的设备名为 `wmpointer`。启用 WinTab 不会关闭后者的 `WM_POINTER` 输入，因此同一物理笔可能交替送来两套 Hover、Press、Move、Release 与 proximity 事件。

画布只接受当前后端对应的设备流。活动后端为 `wintab` 时，`CanvasItem` 在应用事件过滤阶段消费 `wmpointer` 的 TabletPress/Move/Release 和 Enter/LeaveProximity；其他平台与 Windows Ink 路径保持原行为。过滤发生在 Quick、菜单与变换路由之前，避免同一物理笔被两个设备重复解释。

笔触本地状态也改为幂等：已接受的 Press 遇到重复 Press 不清除 `m_stroke/m_tablet`；Release 和 proximity loss 以 `PaintCoreClient::drawing()` 为准结束已接受笔触。即使驱动仍产生重复 Press，后续 Release 也能收笔，不能留下永久绘制态。

本修复不改变输入坐标、压力映射、绘画算法、ABI、state 或文件格式。
