# 颜色与参数共同发布链路定位（2026-10-26）

本报告保留修复前调查。后续实现与交付见 [增量发布修复验收](shared-settings-publication-fix-validation.md)。

已复现用户提出的共同路径：颜色、大小、流量、不透明度变化都会触发完整偏好广播；**正常启动即使只有一个可见文档，也会把完整偏好同步给无画布的初始服务客户端**。该客户端是磁盘偏好写入者，仍拥有 BrushLibrary；普通颜色/参数编辑不满足只允许选择差异的快速恢复条件，进入 GUI 线程整库恢复。这是可定位且已测到数百毫秒阻塞的产品路径。

本次只添加默认关闭的计时 spans、opt-in 探针和分析工具；没有改变通知、缓存失效、35ms 定时器、线程调度、产品输入或保存逻辑，没有部署优化后的应用。设计见 [调查协议](../../../brush/performance/shared-settings-publication-investigation.md)。

## 调用链与额外开销

1. 色轮 `ColorWheelItem::picked` 或画布 `CanvasItem::updatePicking` → `PaintCoreClient::setBrushColor` → `BrushLibrary::setActiveColors`。大小/流量/不透明度分别进入 `BrushLibrary::update` → `changed` → `queuePersistence`。
2. 两者均重新启动35ms单次定时器，timeout 执行 `persistRequested(snapshot())`。颜色保留有效对象缓存；普通参数调用 queuePersistence 清空完整偏好及执行目录对象缓存，故其 snapshot 在 GUI 重建全库活动行、reset 基线和工具 profiles，再编码。
3. `DocumentManager::track` 对 persistRequested 的槽同步遍历 `m_clients`，调用每个接收方的 `restoreBrushPreferences(state, source)`。`m_clients` 包括 configure 中 track 的初始客户端，不只 documents() 中的可见文档。
4. 正常 Main.qml 的 createInitialDocument=false。首个真实文档由 create 创建新的 PaintCoreClient；初始客户端继续作为应用服务/偏好写入者。一个可见文档实际有两个客户端。
5. `restoreSharedSelection` → `restoreSelectionOnly` 只接纳选中项差异。颜色变化使 colorState 不等；参数变化使 presets/profiles 不等，`expected!=root` 拒绝快路径。随后 restore(bytes) → restoreObject 的完整校验/解码，包括 reset、三个 profiles、预览复用和整库通知。原有预览复用并不免除验证成本。
6. DocumentManager 随后调用写入者 persistBrushPreferences(state)。其 synchronizeBrushCatalog 中，表达式 `state==m_brushLibrary->snapshot()` 再查询刚被完整恢复清空的服务端快照，追加一次 GUI 全库构造/编码。目录未变化的颜色编辑也会付这笔成本。实际磁盘保存另排到 BackendWorker，不把后台耗时直接加到 GUI 上。

因此参数编辑除接收方恢复外，还多出源文档的整库快照构造；颜色编辑虽在源侧保留缓存，却绕不过接收方和写入者的工作。增加可见文档会再增加接收方数量。快速拖动可暂时合并发布，慢速连续移动一旦反复超过35ms，就会在拖动中多次卡住；快速拖动则常在停止后顿一下。

## 实测与启动方式消融

产品基线 HEAD `eb0a6316268fc7c110bbba8bbaaa3c55d3cac523`。本机原生 Windows 可见 Release Main.qml 窗口1600×1000、DPR=1.5，Qt实际初始化 D3D11 / NVIDIA RTX 4060 Laptop GPU，非只根据环境变量判断。使用仓库既有 `artifacts/user-brush-repro` 的489项真实配置/资源副本，选择 round-pressure；工具隔离原值1。配置及78MB资源复制到新实验目录，用户配置不修改。

最终相同二进制 SHA256 `230b5fa1399d25382e4a619953fbf2e14e4b9878725217b064f67dbbb89ad24b`，分别运行：

- `real-normal-3`：正常启动，一个可见文档+初始服务客户端，8场景×8移动=64次，全部改变并验证最终保存及接收方值；3 passed含初始化/清理，零失败/跳过/输入异常/trace溢出。
- `real-adopt-final`：同一二进制与同一源配置，历史基准 createInitialDocument=true，把初始客户端直接当作首个文档，无额外服务接收方；同样64次，全部通过。
- `default-normal-final`：同一二进制、正常启动、默认4项，8场景64次，全部通过。

以下为每段8次移动的**最大 GUI 心跳间隔**，单位ms；8/50ms为请求的事件间等待，阻塞后实际节奏变慢。不是输入到显示时延，不是跨机器性能承诺。

| 操作 | 正常单文档，489项：快速拖动 / 收尾 | 正常单文档，489项：慢速拖动 | 采用初始客户端的单文档对照，489项：慢速拖动 | 默认4项正常单文档：慢速拖动 |
|---|---:|---:|---:|---:|
| 色轮 | 14.583 / 272.278 | 296.818 | 15.818 | 19.955 |
| 大小 | 14.565 / 348.452 | 361.289 | 96.663 | 13.163 |
| 流量 | 15.072 / 340.673 | 366.287 | 116.277 | 12.282 |
| 不透明度 | 14.032 / 353.710 | 381.731 | 101.461 | 12.178 |

正常单文档慢速段，颜色发布8次、服务端目录重建8次；大小5/5、流量6/6、不透明度7/7。快速段各合并一次发布与一次服务端重建。颜色原始HSV结果按既有HexArgb的8位精度核对接收方；四项实际INI与最终模型一致。

正常启动489项慢速段分层最大方法耗时：

| 操作 | 广播槽（包含恢复与调用写入者） | 接收方restoreBrushPreferences | 写入者synchronizeBrushCatalog | snapshotObject单次重建最大 |
|---|---:|---:|---:|---:|
| 色轮 | 287.333 | 210.165 | 86.626 | 75.738 |
| 大小 | 271.299 | 184.540 | 90.587 | 72.461 |
| 流量 | 270.879 | 175.687 | 95.631 | 80.338 |
| 不透明度 | 285.481 | 194.840 | 98.451 | 78.391 |

这些为嵌套span或不同次调用的最大值，**不可相加**。正常参数段每次发布出现两个 snapshotObject 重建（源文档+写入者），颜色段每次一个（写入者）。色轮 paint 最大约2.9–3.0ms、三角色图重建约2.1ms；输入派发最大约1.2–1.3ms（色轮）、4.6–4.8ms（三参数），无法解释数百毫秒的GUI阻塞。上述整库工作直接发生在GUI线程并与长心跳对应，后台保存不是这段阻塞的归属。

较早 `real-2` 使用采用初始客户端的1/2/3可见文档矩阵，24场景192移动全部通过。489项慢速色轮广播最大0.001/288.875/545.736ms；最大GUI间隔14.597/295.962/553.044ms。它说明恢复成本随接收方数量增长，但单文档栏不代表正常启动。`default-1` 同样24场景通过，4项的对应广播最大仅0.001/1.088/2.066ms。

## 测量边界与保留的失败

- 初版 `real-1` 在心跳计时内轮询INI，QSettings读取可与后台写入相互等待，因此其心跳不用于结论；另因HSV内部精度与HexArgb恢复精度不同触发颜色相等断言失败。最终改为计时区间结束后才读INI，颜色按原持久化通道精度比较。没有改产品行为或保存精度。
- `real-normal-1`、`real-normal-2` 检测到非计划窗口输入并失败退出，完整日志/报告保留，不计完整通过。第二次报告保留一条非计划MouseMove。最终 `real-normal-3` 无非计划输入，通过同一断言。
- 快速与慢速仅各8次，最终成对启动方式对照各一个进程；不声称p95稳定分布或通用提升倍数。配置是仓库保存副本，不能证明与用户此刻运行的全部偏好逐字节相同；未测实体手绘板或物理显示时延。
- 原有A/B/参数基准默认采用初始客户端，且多为合成小库，不能用它们的单文档结果否定这里正常启动的隐藏接收方。旧报告保留，本次没有重标历史数据。

## 复现与下一步

在已有MSVC/Qt开发环境中构建同一个opt-in目标（本次Rust Release、生成头检查、Qt编译/链接均通过）：

```powershell
cmake --build build --target qt_brush_interaction_baseline -j 2
python tools/measure_shared_settings.py build/bin/Release/qt_brush_interaction_baseline.exe artifacts/shared-settings-new-normal --settings artifacts/user-brush-repro/DrawVerse.ini --documents 1
python tools/measure_shared_settings.py build/bin/Release/qt_brush_interaction_baseline.exe artifacts/shared-settings-new-adopt --settings artifacts/user-brush-repro/DrawVerse.ini --documents 1 --adopt-initial
python tools/measure_shared_settings.py build/bin/Release/qt_brush_interaction_baseline.exe artifacts/shared-settings-new-default --documents 1
python tools/analyze_shared_settings.py artifacts/shared-settings-new-normal/report.json artifacts/shared-settings-new-normal/analysis.json
```

输出目录必须不存在；runner记录HEAD、二进制及工具摘要、源配置摘要。最终证据在忽略目录 `artifacts/shared-settings-20261026/`，每次含report/test/qt日志、隔离INI与资源；最终三次另含invocation.json及analysis.json。Python编译检查和git diff --check通过；未跑全量check，没有功能优化或新应用交付。

修复应先收窄已验证模型之间的颜色/单预设参数同步，避免为这些差异完整恢复全库；同时消除参数编辑的全库偏好对象重建和服务写入者重复构造。服务客户端承担持久化职责，不能直接删除或跳过最终保存。保持立即起笔捕获最终参数、工具隔离、原子无效输入拒绝与外部完整恢复；增加正常欢迎页启动的回归入口。延长防抖仅改变卡顿时机，不能消除已定位的同步工作。
