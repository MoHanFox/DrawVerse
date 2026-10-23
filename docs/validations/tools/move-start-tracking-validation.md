# 主线 C3 — Move Start & Tracking Responsiveness

2026-10-23。本轮交付**有界调查与可执行探针，没有产品优化**。已在两个有效
原生进程中定位 W 起拖的一段主要等待；后续直接 W 受到非计划输入干扰，按任务
边界停止原生重复。持续跟随、直接 W 的有效对照及 Main.qml 代表性验证尚未完成，
不以工具已经实现冒充这些验证已经通过。

## 版本、环境和证据

- 实际产品基线、fetch 后 upstream/develop：
  `9b36e2f80f414ddee062eb87fb6ed429359cae17`，已含 C2-B PR #9。
- 分支：`perf/issue-2-move-start-tracking`。测量/工具提交见末尾版本记录；没有修复提交。
- 产品算法与策略未变：C2-B 资格、16ms 合并、worker 8/16ms 轮询、BEGIN/sequence/
  关闭屏障、取消、预热、采样和画质全部保持。`PaintCoreClient.cpp` 只有默认关闭
  trace 增量；`core/`、依赖、锁文件、ABI、格式没有改动。
- 原生 Release / Qt 6.8.3 / MSVC 2022；复用英文入口、既有 SDK 和 build/Cargo target。
  Qt 实际日志：**threaded render loop / D3D11 / AMD Radeon(TM) Graphics
  (1002:1638)**，vsync animation driver 16.67ms，max frame latency 2，软件设备偏好 0，
  pipeline cache collection 0。没有选择 NVIDIA 独显。
- `LQ156M1JW26`，逻辑 DPI 96、DPR 1，原生窗口 1024×720，fit zoom 0.17578125。
  S2：4096² 文档，隐式白底加独立 Normal 满幅红栅格层，非空范围 `[0,0,4096,4096]`。
  精确 layers/viewport/source 信息见每进程 raw。4K 指文档而非窗口。
- 沿用 C1 测量环境 `QSG_RHI_DISABLE_DISK_CACHE=1`，隔离 QML cache。**这是测量条件，
  不是正式启动的缓存配置，也没有当作产品修复。**
- 新证据根：`artifacts/issue-2-main-c3/`，未提交。每目录独占创建；invocation 保存
  HEAD、完整源码逐文件 SHA256、EXE SHA256、探针模式；不覆盖历史目录。

两个有效 v2 进程的 EXE SHA256 均为
`4c047ff6d7b99edc8429eb83e58b7f6b5635ae185a16bcfaba4372d9d7e7c05c`。
原生测量发生在基线 HEAD 上的未提交 v2 观测工作树，**不能写成最终工具提交已经
重新通过原生验证**。`measurement-source-v2/` 保留六个构建/宿主/观测增量的准确源码，
`verification.json` 对照 invocation 字节摘要全部相同；其余产品文件由基线和 invocation
摘要约束。最终工具又补了更严格的常驻关联反例、失败保留和溢出计数，完成增量构建
与确定性验证，没有追加原生运行。缓存产品始终是同一个基线。

## 协议与执行方法

设计：[`move-start-tracking-design.md`](../../tools/move-start-tracking-design.md)。

新增按需 `qt_move_start_tracking`（EXCLUDE_FROM_ALL，不注册 CTest/CI）；复用
`PendingMoveEvidence` 的**具体活事件对象**登记和窗口正常路由。Press 后立即发送
非零 `(384,0)` Move，再返回事件循环；原生日志没有用零位移 BEGIN 充当首次位移。
Press 与 Move 均为 Qt 合成鼠标输入，不是 OS 注入、设备采样或实体笔。

`--tracking` 提供独立 GUI 8ms QTimer 轨迹：前48点沿 x 以4文档像素前进；暂停12个
目标周期；接着36点沿 y 前进；最后36点转向到 `(384,0)`，共120个 Move。
保留计划时刻、实际 dispatch、同步返回和绝对位移，不追赶式补发。它是**事件循环
驱动的轨迹**，不是外部恒定125Hz。本轮未取得该模式的原生数据，不给其 p95 或收益。

```powershell
# 已激活既有 MSVC/Ninja 环境
cmake --build build --config Release --target qt_move_start_tracking qt_transform_tests -j 2
python tools/measure_move_start_tracking.py build/bin/Release/qt_move_start_tracking.exe artifacts/issue-2-main-c3/new-iw --trace
python tools/measure_move_start_tracking.py build/bin/Release/qt_move_start_tracking.exe artifacts/issue-2-main-c3/new-direct-w --trace --direct-w
python tools/measure_move_start_tracking.py build/bin/Release/qt_move_start_tracking.exe artifacts/issue-2-main-c3/new-tracking --trace --tracking
python tools/analyze_move_start_tracking.py artifacts/issue-2-main-c3 artifacts/issue-2-main-c3/new-analysis
python -m unittest discover -s tools -p test_analyze_move_start_tracking.py -v
```

默认执行 I→提交→撤销→关闭移动工具→重新准备 W；`--direct-w` 在新进程直接准备 W。
W 只证明资产存在且上传批次已记录，**不证明 GPU fence 完成或首次管线已经完成**。
I 的三次 Press 均没有准备资产，实际走 CPU interactive frame；没有伪称 GPU 冷资源路径。

同一时间轴只使用 C++ steady_clock。Rust 命令以 session/generation/sequence 精确关联；
Rust `command.submit` 被同次 C++ BEGIN/submit scope 包含，取多条包含区间交集估计
offset。有效 v2 行不确定宽度 8.2–16.9µs；不直接相减原始时钟。核心真正 metadata
publication-ready 时刻没有新增埋点，保持**未测**；worker 首次精确读取是可用性的上界。

worker 元数据 scope 带 completed_sequence/generation/revision，并单记 emit；GUI 入口
位于回调最开始。常驻路径按 generation/revision/source 关联 GUI 与节点，普通帧按
generation/revision/view/request/frame/imageKey 关联。节点记录在 `updatePaintNode`
同步阶段产生，只含不可变 identity/time/thread；没有 render 回调跨线程读 GUI 对象。
before/afterRendering 和 frameSwapped 只记录线程时间；**不称 GPU 完成或物理显示延迟**。

计时至目标非零姿态节点采用。随后才串行抓图，沿用同图 `(100,1080)` 白、`(800,1080)`
红的原阈值和稳定 input/context/render epoch 协议。常驻路径允许无效期 CPU 图保持旧
revision，但必须有当前 resident 的精确 source/revision 且节点确实采用它；旧正常节点
不能借 GUI 中存在的新 resident 取得认证。原 pendingMoved 默认 CPU 协议不变。
抓图耗时、QTRY 等待和 hold→Release 的正确性观察时间均不计入起拖处理耗时。

## 每次原生运行（全部保留）

| 目录 | 模式 | 结果、输入和异常 |
|---|---|---|
| `explore-iw-1` | 探索 v1，I→W | 进程 exit 1。I committed；W 在抓图期 scenario_invalid，不能算通过。v1 错把旧 CPU frame revision 当作 resident 抓图的必需前提，且没导出入口明细；既无有效 W 像素认证，也不能补写成产品像素错误/某种取消。 |
| `protocol-v2-iw-1` | v2，I→W | exit 0；两段均 committed，按住目标、同图颜色/关联、最终正常帧节点接管、单历史和撤销完成。每段 Press/Move/Release 各一次。 |
| `protocol-v2-direct-w-1` | **实际仍是 I→W** | exit 0；同上。目录名字来自调用标记错误，invocation 的 direct_w=false 才是真实模式，不当作直接 W。 |
| `protocol-v2-direct-only-1` | v2，新进程直接 W | exit 1；Press/计划 Move 已执行，之后收到未登记的窗口 MouseMove，绝对 delta 变为 `(-1978,-184)`。终态 invalid_input_or_lifecycle；没有 Release、没有 COMMIT 回执、没有目标像素通过。完整入口记录保留，不归咎驱动或 Qt。此后停止原生重复。 |

总共4进程；有效 v2 只有2进程/4次拖动，不满足3次独立有效重复。
探索 I 单列，不能与不同协议混池；未执行剩余跟随、直接 W、诊断关闭及 Main.qml 原生对照。
没有超时、崩溃或新的有效输入下产品/像素失败被记录；这不等于那些未测路径通过。
两个完整成功进程每个只有1个功能函数；init/cleanup 不计功能用例。

## 起拖关键时间线

全部为相对 Press send 入口的 ms（登记的窗口 Press ingress 随后约0.005–0.009ms）。
表中首Move是 send 入口，实际窗口 ingress 分别为0.539/0.566/0.548/0.558ms。
离线分析按登记的具体事件 plan_id 对应入口记录；v2 按源码中每次send的一一串行关系
恢复 plan_id，不从时间附近找 MouseMove。计划→窗口 ingress 为0.002ms左右，
同步 send scope 另列，没有把它替代发生器延后。每段只有一个 Move，列原值，不对单样本计算有意义的
p95；完整输入和单值统计仍留在 JSON。没有 after 产品版本，**下表不是优化前后收益**。

| 进程/阶段 | 首Move dispatch | BEGIN worker读取 | BEGIN GUI入口 | 非零UPDATE提交 | UPDATE GUI元数据 | 目标节点采用 |
|---|---:|---:|---:|---:|---:|---:|
| v2-iw-1 / I | 0.537 | 7.870 | 7.967 | 7.994 | 24.142 | 173.789 |
| v2-iw-1 / W | 0.564 | 10.296 | 122.612 | 122.629 | 130.839 | 144.095 |
| v2-direct-w-1 / I | 0.546 | 15.599 | 15.760 | 15.779 | 31.728 | 165.296 |
| v2-direct-w-1 / W | 0.556 | 12.927 | 95.022 | 95.037 | 109.180 | 119.226 |

1. W BEGIN 已在约0.24–0.26ms 被核心应用。BEGIN 核心 submit→accept 为
   0.009–0.012ms，apply 为0.009ms左右；无法解释95–123ms的 GUI入口。
   worker 读取/构建元数据约0.060–0.069ms，emit→GUI 分别 **112.264ms、82.039ms**。
   UPDATE 在 GUI 得到 BEGIN 回执后才可提交，这是现存屏障；不能把它全部归为 Rust 执行。
2. W UPDATE 核心 submit→accept 0.009–0.022ms、apply 0.004–0.008ms；之后精确
   resident GUI接收在130.875/109.207ms，节点采用144.095/119.226ms。
   已到达窗口 Move→节点分别 **143.528 / 118.668ms**。节点采用仍不等于显示扫描输出。
3. 跨越 Press 的首次常驻材质 `beforeRendering→afterRendering` 区间分别
   **129.324ms（-8.068→121.256）/122.716ms（-29.017→93.699）**。
   Qt窗口生命周期日志累计 pipeline creation 分别127/121ms；GUI元数据交付和
   心跳长间隔落在同一窗口。支持“首次图形工作与 threaded loop GUI交付等待相关”，
   **没有精确到 QRhi 创建/驱动锁/Present 的独立调用栈，不能宣称整段均为 shader 编译**。
4. 每个 W source=4，createTextures 各一次（CPU scope 0.015–0.016ms）、uploadBatch
   各一次（0.942–1.400ms）。实际经过 acquire 的窗口/source 弱注册复用，按住期间未
   创建第二份同源纹理；不是仅凭相同 source ID 就断言复用。上传 scope 是CPU记录，非fence。
5. I 没有 resident，当前首非零画面确实是 CPU interactive task。精确
   view/generation/request/revision/frame 连接到 frame6：入队65.377/66.595ms，
   开始96.931/93.632ms，执行58.188/50.547ms，发布161.588/153.204ms，
   GUI接收168.873/159.635ms。这里有任务等待及CPU执行，不能将 W 的首管线解释
   直接套给 I。未进一步分解 UPDATE→任务快照间隙，也不新增第二套调查。

所有区间可能嵌套/重叠，不相加。W 的前置资源创建已从 Press 前开始，不能把这部分
忽略后声称“准备免费”。heartbeat 为整个观察段代理：有效 I 的 p50约8.06–8.10ms、
max12.01–12.17ms；W p50约10.55–10.92ms、max94.78–122.40ms，包含正确性/grab/收尾期，
不是输入到显示延迟。离线 `analysis-3/analysis.json` 保留阶段与命令身份。

## 确认、推断和未测

**已确认**：两个有效进程的原生窗口中首个非零输入在0.567ms以内已经送达；W 较长等待发生在
worker发出元数据之后、GUI入口之前，核心 BEGIN 已完成；首次零位移 resident 与目标
非零 revision 分开采用。两次W正确性通过，不存在用 BEGIN 零预览充数。

**有测量支持的候选**：首次 resident 材质图形工作（含 pipeline creation）与这段
GUI不可交付区间重叠；同源纹理重复创建、Rust BEGIN执行不是这两条链的主要成本。
尚不能断言 Qt或驱动有缺陷，也不能断言提前无回执提交UPDATE会消除这段成本。

**未验证**：持续拖动、暂停/转向的每次采用；直接 W 与 I→undo→W 的顺序差异；
三次独立有效重复；同进程第二次材料使用的独立对照；诊断关闭扰动；真实 Main.qml；
实体手感、学生资源、GPU fence及物理显示延迟。旧8ms发生器报告只作线索，本轮不拿它
当合并后 before，也不能定量宣称旧首Move延后中有多少来自发生器。

## 修改与验证边界

- 没有具备充分因果证据的局部产品修复。没有证据证明删除回执屏障可消除已观察到的 render/
  GUI等待，且可能破坏早Release/Cancel顺序；预先完成同一管线工作可能只是转移成本。
  因而未改变上述策略，没有前后收益分布，也无需独立优化 before 工作树或新运行包。
- C++默认关闭观测仅补 worker metadata、GUI入口和resident generation；现有 observer
  可显式请求最多4096条 identity history，线程安全、有丢弃计数，默认一槽不分配历史。
  正式 Canvas 没有 observer，不增加计时器、刷新或记录磁盘I/O。
- runner 独占输出、配置隔离，保留进程 exit/输入/未完成；分析器拒绝多义 sequence、
  旧source/generation/revision、重复绝对值的猜测性输入匹配和不一致时钟区间。
- Release增量构建 `qt_move_start_tracking`、`qt_transform_tests` 通过，生成头check通过。
  最终新增1个功能函数检查 resident关联与旧CPU/旧GPU/ABA反例、容量溢出；原2个功能
  函数（原12项状态与13项关联反例）通过。确定性函数不打开原生渲染窗口，不冒充D3D11验收。
- Python离线8个测试通过，覆盖准确事件入口/身份、独立时钟、不完整失败、重复输入、旧source/
  generation/revision以及发生器延后与同步处理的分离。无默认CI/跨机器阈值。
- 核心/C2-B产品未改，复用 C2-B 报告的17项完整像素、取消/预算及提交/历史证据；
  不重跑性能矩阵。不把本轮少量运行写成全面压测或所有早关闭场景的新验收。
- C1历史取消/像素失败、C2-A抓图崩溃和历史pendingMoved未归因仍按原报告保留，
  与本次确实记录的非计划 MouseMove 分开，不重新归因。

**一个具体下一步**：在可取得有效窗口输入时，用同一即时非零探针完成“新进程直接W”
与“同进程第二次W”的小对照，并在实际 Main.qml 确认首材质渲染长区间是否仍阻塞GUI。
在此之前不实施 pipeline预热、回执放行或调度改动。用户无需现在暂停工作或连接数位板。

## 版本记录

工具、设计与确定性回归提交：`3ebbd70c8604c5fabcb4908f9903bf8f96709e6f`。
测量源码身份以各 invocation 和 `measurement-source-v2/verification.json` 为准；这份
最终工具提交包含测量后的验收强化，不能替换原始测量源码摘要。最终构建/确定性结果
见 `final-build.log`、`deterministic-final/`；工具与本报告分开提交。没有产品修复，也没有
新运行包；旧包、对照工作树和全部旧证据保持。
