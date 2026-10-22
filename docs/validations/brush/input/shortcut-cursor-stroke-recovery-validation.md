# 快捷键光标与鼠标笔触恢复验收

## 2026-10-22：第二点UI的快捷键通知修复

物理按键轮询此前用实时getter比较前后可见性，两次均读到新键态；Alt前后原生光标同为Blank时没有其他变化可触发通知，QML轮廓/十字可能保持旧值。现统一为最近一次光标刷新采样的快捷键快照，轮询先比较旧显示再发布新显示，所有getter及原生光标使用相同快照，笔尖姿态读取保持独立。

新增 `physicalShortcutsUpdateQmlWithoutPointerOrKeyEvents`：真实Windows窗口中用SendInput改变物理Alt/Ctrl，应用事件过滤器阻止KeyPress/KeyRelease/ShortcutOverride进入画布，指针位置不动，分别在鼠标及笔悬停所有权下验证QML绑定的画笔/吸色可见性与窗口原生光标及时切换，按住Alt跨多个16ms周期保持。600ms为测试超时上限，不是响应延迟测量。既有无modifier TabletMove、丢失KeyRelease恢复与鼠标/手绘笔重复Press同时通过。

软件与D3D11各4个功能用例（含初始化/清理各6 passed、0 failed），日志 `artifacts/shortcut-qml-software.txt`、`artifacts/shortcut-qml-d3d11.txt`。Release应用部署和测试目标构建、生成头检查通过；标准 `build/bin/Release/drawverse.exe` 已更新。未改ABI1.80/state45/手势判定或文件格式，未跑全量测试；物理键盘注入与合成笔悬停不能替代实体数位板连续压力/移动验收。

2026-10-22，Windows 11 / Qt 6.8.3 / Release / 原生 windows 平台与 software Quick 后端。

- `shortcutCursorSurvivesModifierlessTabletMoves`：先只用鼠标验证 Alt/Ctrl，收到无 modifier 的 MouseButtonRelease 时立即恢复画笔光标，不依赖手绘板；再验证无 modifier 的 TabletMove 不会冲掉已按下状态，KeyRelease 后再移动仍保持画笔状态且不得重新锁存旧 modifier。另注入只有 KeyPress、没有 KeyRelease 的 Alt/Ctrl，Win32 物理键态短轮询必须清除工具光标；窗口原生 `cursor()` 与 Item 状态一致（Ctrl 时 `SizeAllCursor`，松开后 `BlankCursor`），修复前两者会长期不同步。
- 快捷键轮询路径与笔尖姿态路径已分离，轮询不会反复调用姿态读取；现有真实 Tablet Hover/Press/Move/Release 与 Caps Lock 精确光标用例继续通过，窗口原生光标断言保持。
- `duplicateMousePressStillEndsOnRelease`：真实窗口画布上重复 MouseButtonPress 后仍保持 drawing，Move/Release 正常结束，只提交一条笔触历史。
- 原 `duplicateTabletPressStillEndsOnRelease` 与笔刷/液化 proximity loss 两行继续通过。
- `layerMoveFollowsBeforeReleaseAndCancelsWithSingleHistory` 继续通过，覆盖 Ctrl 临时拾取移动、松手单历史、Esc 取消及移动结束后显示刷新。
- 定向 TransformTests 含初始化/清理共 `7 passed, 0 failed`，日志 `artifacts/tablet-cursor-tool-conflict.txt`；UiTests 相关共 `3 passed, 0 failed`，日志 `artifacts/ctrl-move-tool-conflict.txt`。
- 鼠标长绘停刷候选已从显示与输入两层收口：普通笔触开始和互动工具状态结束都会清除遗留常驻预览；本地/客户端笔触状态分裂可由下一次 Press/Release 恢复。未用实体笔做长时间连续压力与悬停验收。

ABI1.79/state45 保持不变，未运行无关全量测试。

## 2026-10-22：元数据回执竞态导致永久停更

代码检查发现 `BackendWorker::publish` 在最后一次 `paint_session_committed_strokes` 读取前设置 `metadataPending`。该读取按精确 publication 查询，actor 在两次读取之间发布即可返回 BUSY；失败路径没有发送 metadata 信号，GUI 无法清除标记，后续 `poll` 在 publish 处返回，连像素读取也不再执行。现仅在全部读取成功后、发送信号前置位，BUSY 与错误路径均保持可重试。

新增 `continuousStrokesKeepPublishingFramesAndAllowLaterEdits`：真实 Windows 窗口中连续24笔、每笔104次移动并让事件循环与后台并行运行，确认历史增长、帧修订追上以及末笔真实红色显示；液化和自由变换取消后再画第25笔，确认新像素可见。并发覆盖不保证每次命中最后两次查询之间的竞态，未引入生产故障注入接口。

最小专项：新增用例、快捷键、鼠标重复Press、数位笔重复Press、笔刷/液化异常离笔及4K预热移动，软件与D3D11各7个功能用例（含初始化/清理为9 passed、0 failed），日志 `artifacts/publication-retry-software.txt`、`artifacts/publication-retry-d3d11.txt`。4K用例同时检查未预热输入、同源纹理复用、单历史、取消和最终完整帧；抓图耗时不作为实体输入延迟。

补查 `layerMoveFollowsBeforeReleaseAndCancelsWithSingleHistory` 时，旧测试在D3D11首拖强制要求常驻包，未等待预热，与ABI1.80未就绪立即走完整CPU帧的契约冲突。断言改为等待有效网格或完整帧；保留真实像素随动、原位清除、单历史、取消、空白Ctrl拾取、手绘笔和复杂混合检查。预热后的强制网格/纹理复用仍由4K专项验收。

更新断言后，该UI用例软件和D3D11各1个功能通过（含初始化/清理各3 passed、0 failed）；日志 `artifacts/publication-retry-ctrl-move-software.txt`、`artifacts/publication-retry-ctrl-move-d3d11.txt`。两后端合计各8个功能用例通过。

Release应用与专项测试目标构建、生成头一致检查通过；标准交付 `build/bin/Release/drawverse.exe` 已更新。没有改动Rust、ABI1.80/state45或文件格式，未运行全量测试；实体数位板持续压力/悬停与移动手感仍需实机确认。
