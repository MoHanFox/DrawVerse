# 快捷键光标与鼠标笔触恢复验收

2026-10-22，Windows 11 / Qt 6.8.3 / Release / 原生 windows 平台与 software Quick 后端。

- `shortcutCursorSurvivesModifierlessTabletMoves`：Alt 按下后发送不带 Alt 的 TabletMove，吸色光标保持；Ctrl 按下后发送不带 Ctrl 的 TabletMove，临时移动光标保持；KeyRelease 后恢复画笔状态。修复前 Alt 第一条无 modifier 移动即被清掉。
- `duplicateMousePressStillEndsOnRelease`：真实窗口画布上重复 MouseButtonPress 后仍保持 drawing，Move/Release 正常结束，只提交一条笔触历史。
- 原 `duplicateTabletPressStillEndsOnRelease` 与笔刷/液化 proximity loss 两行继续通过。
- `layerMoveFollowsBeforeReleaseAndCancelsWithSingleHistory` 继续通过，覆盖 Ctrl 临时拾取移动、松手单历史、Esc 取消及移动结束后显示刷新。
- 定向 TransformTests 含初始化/清理共 `7 passed, 0 failed`，日志 `artifacts/tablet-cursor-tool-conflict.txt`；UiTests 相关共 `3 passed, 0 failed`，日志 `artifacts/ctrl-move-tool-conflict.txt`。
- 鼠标长绘停刷候选已从显示与输入两层收口：普通笔触开始和互动工具状态结束都会清除遗留常驻预览；本地/客户端笔触状态分裂可由下一次 Press/Release 恢复。未用实体笔做长时间连续压力与悬停验收。

ABI1.79/state45 保持不变，未运行无关全量测试。
