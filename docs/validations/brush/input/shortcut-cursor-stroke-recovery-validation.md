# 快捷键光标与鼠标笔触恢复验收

2026-10-22，Windows 11 / Qt 6.8.3 / Release / 原生 windows 平台与 software Quick 后端。

- `shortcutCursorSurvivesModifierlessTabletMoves`：先只用鼠标验证 Alt/Ctrl，收到无 modifier 的 MouseButtonRelease 时立即恢复画笔光标，不依赖手绘板；再验证无 modifier 的 TabletMove 不会冲掉已按下状态，KeyRelease 后再移动仍保持画笔状态且不得重新锁存旧 modifier。另注入只有 KeyPress、没有 KeyRelease 的 Alt/Ctrl，Win32 物理键态短轮询必须清除工具光标；窗口原生 `cursor()` 与 Item 状态一致（Ctrl 时 `SizeAllCursor`，松开后 `BlankCursor`），修复前两者会长期不同步。
- 快捷键轮询路径与笔尖姿态路径已分离，轮询不会反复调用姿态读取；现有真实 Tablet Hover/Press/Move/Release 与 Caps Lock 精确光标用例继续通过，窗口原生光标断言保持。
- `duplicateMousePressStillEndsOnRelease`：真实窗口画布上重复 MouseButtonPress 后仍保持 drawing，Move/Release 正常结束，只提交一条笔触历史。
- 原 `duplicateTabletPressStillEndsOnRelease` 与笔刷/液化 proximity loss 两行继续通过。
- `layerMoveFollowsBeforeReleaseAndCancelsWithSingleHistory` 继续通过，覆盖 Ctrl 临时拾取移动、松手单历史、Esc 取消及移动结束后显示刷新。
- 定向 TransformTests 含初始化/清理共 `7 passed, 0 failed`，日志 `artifacts/tablet-cursor-tool-conflict.txt`；UiTests 相关共 `3 passed, 0 failed`，日志 `artifacts/ctrl-move-tool-conflict.txt`。
- 鼠标长绘停刷候选已从显示与输入两层收口：普通笔触开始和互动工具状态结束都会清除遗留常驻预览；本地/客户端笔触状态分裂可由下一次 Press/Release 恢复。未用实体笔做长时间连续压力与悬停验收。

ABI1.79/state45 保持不变，未运行无关全量测试。
