# Windows Ink入点后的悬停光标

2026-10-15，用户反馈开启驱动Windows Ink后，从外部滑入会同时显示系统光标和笔尖轮廓；直接入点后绘画位置正确，但松笔悬停轮廓回到入点。优先修复。

代码证据：Windows Ink路径送TabletMove并允许系统兼容鼠标消息；Qt Quick的frameSynchronousHover使用lastMousePosition生成timestamp0/默认鼠标设备的Hover。该位置可能仅由初始Enter更新，而Canvas应用过滤器消费TabletMove，不更新Quick缓存。旧Canvas在不落笔时无条件接受默认鼠标Hover，覆盖数位笔位置/几何并释放原生窗口光标；落笔时的m_tablet保护解释了绘画正常。此为与反馈吻合的代码路径，尚不是该实体驱动的事件录制。

最后实际输入来源为数位笔时，忽略非数位笔HoverEnter/Move/Leave，保留最后真实Tablet/数位笔Hover位置与设备姿态。真正鼠标Move/Press（非合成且设备Mouse）在非数位笔笔触期间先释放笔所有权，随后正常鼠标Hover接管；笔触期间伴随鼠标不抢占。离开感应范围、窗口/画布离开、失活、隐藏/禁用/换窗/销毁沿既有释放路径。笔离开后仍保留输入来源记录，防止旧Hover复活轮廓；真实鼠标Move/Press解除记录。未出现过笔输入时鼠标Hover不受限制。

Quick在Hover派发后还会按缓存位置更新原生光标；忽略旧Hover时用单个可合并的零延迟任务在派发返回后重申笔窗口光标，任务再次校验所有者/窗口/可见性，释放时取消。任务不发绘画或UI重绘通知，避免帧反馈循环。无设备坐标轮询，无定时伪造输入，不改变绘画算法、ABI或格式。

接管时先保存原窗口光标，再切换Item笔尖光标；否则Quick同步解析鼠标目标可能提前改写要恢复的值。回归精确核对应用过滤阶段恢复原光标，随后真实鼠标目标自身引起的Quick光标解析单独处理；窗口离开/隐藏/离开感应范围必须不再保持Blank，不把正常CrossCursor解析误判为恢复失败。

验收：先复现真实Quick窗口Enter缓存/后续TabletMove与帧同步旧鼠标Hover、直接进入/松笔/不同位置及默认鼠标HoverLeave；笔触期间伴随鼠标隔离；真实窗口鼠标接管、系统光标恢复、笔离开/窗口和浮窗隔离；已有动态笔尖/复合光标回归。Windows Release，环境缩放1/2最小相关Qt测试，不宣称实体数位板已认证。

上游依据：[Qt Windows pointer handler](https://github.com/qt/qtbase/blob/6.8/src/plugins/platforms/windows/qwindowspointerhandler.cpp)与[Quick delivery agent](https://github.com/qt/qtdeclarative/blob/6.8/src/quick/util/qquickdeliveryagent.cpp)。
