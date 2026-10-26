# WinTab 双设备流隔离验收

2026-10-22，Windows 11 / Qt 6.8.3 / Release / 原生 windows 平台与 software Quick 后端。

- 本机 `auto` 实际启用 `wintab`。设备选择回归确认 WinTab 后端接受 `wintab` 并拒绝重复的 `wmpointer`；Windows Ink 路径继续接受 `wmpointer`。
- `duplicateTabletPressStillEndsOnRelease` 在真实窗口上先注入被拒绝的 `wmpointer` Press 和 LeaveProximity，再以 WinTab 风格设备完成 Press、重复 Press、Move、Release。被拒绝流没有开始或结束笔触，重复 Press 后仍正常收笔并只产生一条历史。
- 既有离笔回归两行（笔刷、液化）继续通过：未收到 Release 时 proximity loss 结束任务，迟到 Release 不影响下一笔。
- 定向命令：`qt_transform_tests tabletBackendSelectionChecksDriverStateAndPersistsChoice tabletProximityLossEndsStrokeAndEditingRemainsUsable duplicateTabletPressStillEndsOnRelease`。含初始化/清理共 `6 passed, 0 failed`，日志 `artifacts/tablet-duplicate-device-fix.txt`。
- 标准完整 Release 与 C ABI 生成头检查通过，产物更新为 `build/bin/Release/drawverse.exe`。未运行无关全量测试。

输入仍为真实窗口中注入的 QTabletEvent，未代替实体笔驱动的连续压力、悬停与翻面验收；本机后端探测为 WinTab，ABI1.79/state45 保持不变。


## 2026-10-26 手绘笔按钮重复点击与顶栏菜单

用户反馈高漫驱动启用Windows Ink，笔点击像素增加2px，顶栏菜单打开后立即关闭。单路QWindowSystemInterface Tablet对CompactSpinBox只增加1，附加带Windows笔来源标记的原生鼠标后可增加到2。既有wmpointer Tablet过滤覆盖不到该鼠标消息；Qt在部分WinTab路径可能仍将其交给普通鼠标入口。

生产改动仅TabletInput：WinTab实际启用时安装原生消息过滤器，在dispatcher入口消费具有MI_WP_SIGNATURE且非touch的客户端笔鼠标移动/按钮消息。仅匹配应用已有Quick窗口HWND，普通鼠标、触摸标记、滚轮、非客户区、非Quick系统对话框与Windows Ink后端均放行；Qt未处理Tablet时生成的控件鼠标回退保留。无按时间/位置防抖，不改按钮步长或菜单业务。

最终新增tabletClickWithNativePenMouseDoesNotRepeatControlsOrDismissMenu使用真正Windows SendInput队列携带dwExtraInfo，结合已注册WinTab设备的Qt平台Tablet入口：单笔→按钮+1；伴随笔鼠标→值不再变化；无标记的原生鼠标→正常+1；两次快速独立笔点击→各+1；顶栏文件菜单打开后伴随消息不能关闭；文档未绘画/历史不变。分类断言还覆盖touch标记、滚轮、非客户区和Windows Ink放行。先前用SendMessage的探针能复现原控件重复，但它绕过Qt dispatcher原生输入过滤，不能作为修复后验收；最终改用SendInput，未放宽点击/菜单断言。

ordinaryTabletControlsAndTextFocusRemainNormal追加单路像素按钮+1，原快速点击、复选框/滑条/文本焦点保持；连同selectedBackendAlsoFiltersNativeTabletMenu和duplicateTabletPressStillEndsOnRelease，最终软件/D3D11各6项通过（四功能+初始化/结束）。按用户要求仅跑相关最小用例，未跑全套。Release应用与定向测试构建、生成头检查和diff检查通过，build/bin/Release/drawverse.exe更新。无Rust/ABI1.80/state45变化。

日志：artifacts/pen-click-final-{d3d11,software}.txt、pen-click-final-build.log。测试验证实际Windows输入队列、Qt控件和菜单行为，未使用实体高漫采样，实际驱动持续压感/侧键/翻面与用户点击手感仍待实机确认。
