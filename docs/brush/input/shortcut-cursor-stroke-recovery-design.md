# 快捷键光标稳定与鼠标笔触恢复

2026-10-22，用户反馈按住 Ctrl 或 Alt 悬停时画笔轮廓与临时移动/吸色光标来回闪烁，并可进一步进入无法继续绘制的工具冲突；鼠标长时间绘制后也会出现画布不更新，但保存重开能看到已提交笔触。

Alt/Ctrl 状态此前有两个不一致来源：键盘事件维护 Ctrl， TabletMove/Release 又按每条事件的 modifiers 重写 Alt。WinTab 和 Windows Ink 的部分移动事件会省略当前快捷键位；窗口失活还会直接清空 Ctrl。修复后，输入事件只负责锁存已出现的工具快捷键，明确清除仅来自对应 KeyRelease 或窗口激活/失活时的系统键盘状态；按下时使用事件与系统状态的并集，避免先落到错误工具。

系统键盘查询在按键刚释放时可能短暂保留旧值，因此只有 Press 允许使用其作为缺失 modifier 的兜底。Move、Release 和 TabletRelease 严格采用事件自身 modifier；否则 KeyRelease 已清掉状态后，紧随其后的 Release/手绘板移动会把旧值重新锁存，表现为松开 Alt/Ctrl 后光标不恢复并污染后续笔输入。

菜单焦点或浮窗切换还可能吞掉 KeyRelease，使本地锁存值没有事件可清。Alt/Ctrl 任一激活时启动 16ms 短轮询，Windows 直接使用 `GetAsyncKeyState` 读取左右 Ctrl/VK_MENU 的当前物理状态，只有连续两次读到已松开才清除并刷新原生/画笔光标；按键仍按住时轮询保持激活，不会在多源 Hover 之间来回切换。轮询在两键都释放后立即停止。

鼠标绘画与手绘笔共用幂等笔触状态：重复 Press 不丢弃已接受的 `m_stroke`，Move/Release 以本地 stroke 或 `PaintCoreClient::drawing()` 任一为准确认活动笔触，失去抓取时也能取消客户端仍在进行的笔触。此前的本地 false/客户端 true 分裂会让后续 Press 被永久拒绝。

普通画笔开始时清除遗留常驻预览，元数据确认液化、移动或变换全部结束后也发布清除，避免旧 GPU 预览继续遮住已经提交的新笔触。

本修复不改变光标几何、绘画算法、ABI、state 或文件格式。
