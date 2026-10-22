# WinTab 双设备流隔离验收

2026-10-22，Windows 11 / Qt 6.8.3 / Release / 原生 windows 平台与 software Quick 后端。

- 本机 `auto` 实际启用 `wintab`。设备选择回归确认 WinTab 后端接受 `wintab` 并拒绝重复的 `wmpointer`；Windows Ink 路径继续接受 `wmpointer`。
- `duplicateTabletPressStillEndsOnRelease` 在真实窗口上先注入被拒绝的 `wmpointer` Press 和 LeaveProximity，再以 WinTab 风格设备完成 Press、重复 Press、Move、Release。被拒绝流没有开始或结束笔触，重复 Press 后仍正常收笔并只产生一条历史。
- 既有离笔回归两行（笔刷、液化）继续通过：未收到 Release 时 proximity loss 结束任务，迟到 Release 不影响下一笔。
- 定向命令：`qt_transform_tests tabletBackendSelectionChecksDriverStateAndPersistsChoice tabletProximityLossEndsStrokeAndEditingRemainsUsable duplicateTabletPressStillEndsOnRelease`。含初始化/清理共 `6 passed, 0 failed`，日志 `artifacts/tablet-duplicate-device-fix.txt`。
- 标准完整 Release 与 C ABI 生成头检查通过，产物更新为 `build/bin/Release/drawverse.exe`。未运行无关全量测试。

输入仍为真实窗口中注入的 QTabletEvent，未代替实体笔驱动的连续压力、悬停与翻面验收；本机后端探测为 WinTab，ABI1.79/state45 保持不变。
