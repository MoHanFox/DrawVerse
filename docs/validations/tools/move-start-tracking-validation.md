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

## C3 Shader Artifact Check & Precompile Comparison（2026-10-23）

本节为后续独立实验，不改写上文 measurement-source-v2 的身份和结论。
**结论：支持将共享移动材质的 HLSL 编译移到构建期，以减少本机首次图形使用的长等待；
持续跟随没有一致改善，C3、实体手感及物理显示延迟没有整体验收。**

### 版本、产物与构建边界

- 起点 `58ef133b838febe23b5d17d17ea0619b8ecce443`，产品基线仍为
  `9b36e2f80f414ddee062eb87fb6ed429359cae17`；核查时 upstream/develop 未更新。
- 固定共用探针/分析器：`2f472dbd65b25484bce9ae5242ba65f704c3bebd`。
  before 独立工作树 `E:/Git/DrawVerse-c3-shader-before` 在该提交，使用自己的 build
  和 Cargo target；after 构建修改为 `6e6e35f0f097d9c5003315597476269b06ab2295`。
  两侧 Rust 均独立链接，不借用旁边工作树的静态库。
- Main.qml 缓存隔离的测试修订另固定于 `1c867bce1d616324472bd5c5167b8f4ab86964b8`。
  仅向 before 叠加完全相同的两份测试/runner 文件；before HEAD 仍为 2f472db，
  不能称为干净 checkout。`isolated-main-identities.json` 证明两侧测试源码 SHA256
  均为 `5b83e4690b65ac7fbd5825a4327d4af9ead2c972c295f345703b3efb68ae9e95`，
  runner 均为 `7667c76c4cc4e4e9a45abc3655e4346bea171dd553426dd4dda47a392511caee`。
  此修订只改变 Main 窗口暴露前的缓存位置；冷条件的三对使用原固定探针，不混成新源码测量。
- 新证据根目录：`artifacts/issue-2-main-c3-shader/`。各 invocation 记录实际 HEAD、
  全部源码摘要和 EXE 摘要。首次探针 EXE before/after 分别为
  `7f73e952688962177439d129e4d42d1302d2116845c48f8a2e8b54199e8f0135` /
  `91542265a0dc0c4ec47276e669517b02daaa12400400fe423129f8f1e827c9ea`；
  Main 隔离修订后为 `310480cd01e01ab8436ca12222a63cb98852a38cee6d0e9837efbc484fa9aa0c` /
  `dbaf2a031443d4c35ebe9a9a11f329b242fe58cca81d0b3a7c419b5771e41e03`。

qsb 实测为 **6.8.3 / QSB_VERSION 9 / Standard**。原五个目标为 SPIR-V 100、
GLSL 300 es / 330、HLSL 50 源码、MSL 12，**没有 DXBC**。
本机 ShaderTools 宏把 Windows `PRECOMPILE` 转成 qsb `-c`；实际 Ninja 命令保存于
`commands-before.txt` / `commands-after.txt`，没有添加 `-g`、`-O` 或改变目标列表。
现有 SDK fxc 为 10.0.22621.0/x64（帮助标识 Direct3D Shader Compiler 10.1）。

| 实际资源 | before SHA256（大小） | after SHA256（大小） |
|---|---|---|
| liquify.vert.qsb | 43805311f4c23ed8b73e34ad7a2ccc67ba55509e179493adf05cdf8f49e340b7（1441） | c0df27bee7609a02e88c65da7bdbea213ac60887dce03eaaadddde52e7f732a5（1622） |
| liquify.frag.qsb | f60e23977a193c6f643669496e4b5ff97e31ae3226aba6e95dd044a182e27fbe（5340） | 3fcd81d67807de47e65dd8b813dd0281e98f23acc36126646f09c4193c005d37（6871） |

after 将 HLSL 50 替换为 DXBC 50。实际提取的 reflection、SPIR-V、两个 GLSL、
MSL **逐字节相同**，另外三个 shader 的完整包也相同，见
`artifact-after/non-d3d-equivalence.json`。shader 源码、精度、混合和采样逻辑未改。
只拆分现有 shader 资源组以限定两个文件，原 alias 未变，没有重复注册。
`LiquifyPreviewNode.cpp::Shader` 仍绑定 `:/shaders/liquify.vert.qsb` 和
`:/shaders/liquify.frag.qsb`；移动、液化和常驻变换共用该材质，其他自定义 shader 未预编译。
QFile 从测试程序同一 URL 导出的包与构建输出一致，完整序列也唯一存在于正式 EXE
和测试 EXE 中；不是用旁边未链接的 .qsb 代替验证。包中只有 Standard 变体；未直接
hook D3DCompile，不能仅凭资源 URL 宣称观测到了某次编译函数调用。

Windows 默认 `DRAWVERSE_PRECOMPILE_MOVE_SHADERS=ON`，需要 Developer Shell 的 SDK
fxc；找不到时配置明确失败，可显式 OFF 退回原 HLSL 路径。非 Windows 不启用
PRECOMPILE，其他目标保持。构建后还验证实际 DXBC，防止 qsb 外部编译软失败。
拿真实 before 包要求 DXBC 的负测试按预期失败（`negative-hlsl-check.txt`）。
最终用户只需部署包，不需要安装 fxc。

依据：[Qt PRECOMPILE 文档](https://doc.qt.io/qt-6.8/qtshadertools-build.html)、
[Qt 6.8.3 qsb](https://github.com/qt/qtshadertools/blob/v6.8.3/tools/qsb/qsb.cpp)、
[Qt 6.8.3 D3D11 实现](https://github.com/qt/qtbase/blob/v6.8.3/src/gui/rhi/qrhid3d11.cpp)。
D3D11 优先取包中 DXBC，缺失时才进入 HLSL/字节码缓存与 D3DCompile 分支；
DXBC 仍需要创建 D3D shader 和驱动相关工作，不能等同于零管线成本。

### 缓存核查与一次被排除的对照

产品 main.cpp 不设置 QQuickGraphicsConfiguration 或禁用管线缓存；只有独立
QML source preview 宿主设置 QML_DISABLE_DISK_CACHE。嵌入 QML bytecode、QML
磁盘缓存、Qt D3D11 HLSL 字节码缓存和驱动缓存是不同层。
本机 Qt 6.8.3 `QQuickGraphicsConfigurationPrivate::applyEnvironmentSettings()`
确实读取 `QSG_RHI_DISABLE_DISK_CACHE`，不是仅从变量名字推断。冷条件 raw.cache
记录 automatic_pipeline=false、显式 load/save 为空、AA_DisableShaderDiskCache=false；
Qt 初始化日志同时给出 cache collection=0。内存内 QRhi 缓存仍正常工作。

第一次 `main-before` / `main-after` 功能通过，但 Windows Known Folders 没有随
LOCALAPPDATA 重定向。before 写入了默认 **DrawVerseTests/MoveStartTracking**
管线缓存，after 有读取该文件的条件；没有足够的对称初态证据，**整对不用于性能归因**。
其原日志、266.422/156.806ms 准备、20.121/12.930ms 输入→节点值均保留，不能拿后者
证明修复。未删除或清空这个测试应用缓存，也没有改 DrawVerse 原有 ini/作品。

据此修正局部测试：实际 Main.qml 初始 visible=false，在首次暴露前给目标窗口设置
各进程独立的 pipeline load/save 路径，然后正常显示。自动缓存策略保持开启；
两侧断言文件原先不存在，并记录实际 configuration、路径和加载失败（初次无文件）日志。
合格重测为 `main-isolated-before/after`：before 结束写16988字节，after 没有产生该文件；
不能把文件缺失称为缓存禁用，实际 automatic_pipeline 和 collection 均为1。
其他未暴露控件窗 collection=0，目标窗口内 W1/W2 使用同一 QRhi。
参见 [Qt 缓存配置](https://github.com/qt/qtdeclarative/blob/v6.8.3/src/quick/items/qquickgraphicsconfiguration.cpp)
和 [Qt 缓存装载/保存](https://github.com/qt/qtdeclarative/blob/v6.8.3/src/quick/scenegraph/qsgrhisupport.cpp)。
没有清理系统/驱动缓存，没有改变全局设置。Main 对照只修测试缓存前提后补一次，未重跑冷矩阵。

### 输入、图形设备与首次结果

Release/MSVC x64、Qt 6.8.3，实际 threaded render loop、D3D11，均采用
AMD Radeon(TM) Graphics（1002:1638），逻辑DPI均96，未切换 NVIDIA。最小窗1024×720、DPR1、
fit=0.16015625；Main.qml1480×780、DPR1.1、fit=0.14599609375，正式 1.1 缩放入口。
两类窗口不混池。文档均4096²满幅 Normal 内容，保持原 S2 和 (384,0) 最终位移。

每进程直接准备 W1，Press 后立即经已登记的具体 QMouseEvent 发送非零 Move，
不等 BEGIN；撤销后同窗口 W2 使用120个点、8ms GUI QTimer、12个 timer tick 暂停、
两次转向。没有移动 OS 光标、过滤真实输入、补发漏点或调用 processEvents 稳定结果。
各进程两行的 window 和 QRhi 地址相同，source 为1→4（隔离 Main 也为1→4）：
这是同设备、同材质/Standard 变体的后续使用，**不是同一份纹理**。每个源都只创建/
上传一次；冷条件各进程 createTextures=2、uploadBatch=2，按住更新复用对应源。
上传批次是 CPU 命令记录，非 GPU fence。

三对按 before1→after1→before2→after2→before3→after3 交替。共6个合格主进程，
每段1次首操作，不把三次首次值的 p95 当作稳定分布。下表单位ms，render最大区间
覆盖准备至按住结束，起点可能在 Press 前；不得与GUI或核心区间相加。

| 进程 W1 | 准备状态观察段 | Press→Move窗口入口 | BEGIN emit→GUI | Move入口→目标节点 | 最大 render 区间 | Release→完整帧/单历史观察 |
|---|---:|---:|---:|---:|---:|---:|
| before-1 | 153.700 | 0.794 | 91.096 | 121.112 | 154.924 | 27.681 |
| after-1 | 154.052 | 1.281 | 0.269 | 16.257 | 2.122 | 30.651 |
| before-2 | 153.657 | 0.614 | 91.615 | 111.281 | 125.303 | 27.756 |
| after-2 | 153.676 | 0.504 | 0.262 | 33.403 | 2.060 | 30.429 |
| before-3 | 153.347 | 0.522 | 80.120 | 95.365 | 123.598 | 27.255 |
| after-3 | 153.670 | 0.647 | 0.379 | 12.239 | 2.026 | 27.944 |
| Main isolated before | 158.637 | 7.726 | 57.157 | 77.731 | 124.749 | 33.624 |
| Main isolated after | 158.701 | 7.160 | 0.022 | 14.739 | 1.783 | 25.319 |

核心 BEGIN 应用的 Press 后包含区间，before三次分别为0.481–0.489、0.290–0.302、
0.260–0.267ms，after为0.931–0.938、0.221–0.227、0.283–0.290ms；
Main隔离对照为0.564–0.577→0.520–0.526ms。准确 sequence 的同步FFI包含区间和核心接纳/
应用上下界保存在分析 JSON，没把 Rust Instant 与 C++ steady_clock 原始值直接相减。
原本很长的是核心应用之后、worker emit 到 GUI 回调之间。after-2 仍有33ms输入→节点，
不是所有等待都消失。冷 W1 最大图形区间由123.598–154.924ms变为2.026–2.122ms，
并且准备段约153–154ms没有增加；长区间从 Press 前开始，不能只报告其 Press 后部分。
准备数字包含既有状态等待/轮询，只是观察段，不是纯资源准备执行耗时，也不是 GPU 就绪。

关闭详细 Qt/Rust 诊断的一对仍启用必要输入/稳定抓图检查：W1 send入口→目标节点
114.416→23.203ms、准备153.234→153.615ms、收尾36.081→27.734ms。
该节点值用唯一目标 revision/source 的采用记录关联，粒度不同，单独列值不混池；
其 BEGIN/内部渲染阶段未测。不能声称零测量扰动。

### 同进程 W2 跟随结果

各行均实际发送120个 Move、最后(384,0)，正常完成。提交/采用小于120是有界最新值
合并，不能直接算丢输入。所有准确关联的采用 revision 单调，三段轨迹均有采用，
未在这些关联样本中发现旧位置回跳；原始绝对输入、回执及节点身份完整保留。

| 进程 W2 | UPDATE提交/准确采用数 | 输入→节点 p50/p95/max ms | 计划→窗口 p95 ms | Move间隔 p95 ms | 移动首尾跨度 ms | 准备/收尾 ms |
|---|---:|---:|---:|---:|---:|---:|
| before-1 | 93/93 | 15.927/18.830/24.657 | 0.412 | 8.524 | 1048.587 | 153.247/30.984 |
| after-1 | 107/107 | 15.356/23.179/24.716 | 0.355 | 8.643 | 1047.991 | 153.116/23.316 |
| before-2 | 106/106 | 11.061/19.199/20.200 | 0.328 | 8.724 | 1047.246 | 153.491/24.150 |
| after-2 | 109/109 | 9.172/16.954/18.197 | 0.441 | 8.844 | 1047.798 | 153.137/30.833 |
| before-3 | 108/108 | 13.706/20.847/22.017 | 1.124 | 8.928 | 1047.869 | 152.853/31.315 |
| after-3 | 108/108 | 15.803/23.265/24.654 | 0.352 | 8.631 | 1047.839 | 154.069/39.305 |
| Main isolated before | 106/106 | 12.098/20.145/23.487 | 0.784 | 8.602 | 1047.151 | 158.007/26.435 |
| Main isolated after | 107/107 | 14.691/22.073/23.919 | 1.006 | 8.867 | 1047.982 | 156.854/25.362 |

W2首个非零输入→节点，三对before为17.532/18.843/13.706ms，after为
14.810/16.758/15.030ms；Main为13.395→15.007ms，**没有一致改善**。
W2准备至按住结束的最大render区间，before三次为0.125/0.158/0.154ms，
after为0.167/0.171/0.148ms，均没有W1中百毫秒级的首次长段。
暂停产生约104ms的无Move间隔，是既定轨迹，不算发生器阻塞。完整分布/心跳见
`qualified-summary.json` / `analysis-with-isolated-main/analysis.json`；心跳含正确性和
收尾阶段，只是事件循环代理。所有节点采用均非 GPU完成或物理显示延迟。
Qt生命周期 pipeline creation 合计仅作辅助，未全部计入某一 shader。

### 正确性、异常及可运行包

- 6个冷主进程、2个详细诊断关闭进程、2个修正隔离的Main进程，各2段均 committed：
  合格比较20段，无非计划输入、取消、产品错误、超时或崩溃，计数无溢出。
  两个旧Main进程的4段功能结果也保留，但缓存前提不合格，未用来推断收益。
- 合格的5对进程，每对W1/W2各保存计时外同一次稳定关联的按住 grab 和最终完整帧，
  **20组完整PNG逐字节相同**，不只检查两个红白点。见 `whole-output-qualified.json`。
  原两个采样点、阈值、planned exact-event识别、target UPDATE和render/grab稳定身份
  均未放宽。保存动作发生在该段计时之外。
- before/after 各3个原生功能数据行通过（共6，不含init/cleanup）：
  `prewarmed4kResidentMoveReusesFullQualityTexturesAndRestoresFullFrame`，
  `selectedLiquifyKeepsTheMeshPathAndProtectsUnselectedPixels:feathered-lasso`，
  `transformResidentPreviewFollowsAndCompositesTransparentStack:flat`。
  覆盖未准备/已准备按住预览、Escape取消、单历史/撤销、完整帧接管与纹理复用；
  液化还检查羽化选区边缘、撤销/重做/提交/取消，变换检查透明叠层和恢复。
  见 `regressions-before/after/test.txt`，没有修改夹具或断言。
- 9项离线归因测试和1项shader目标检查测试通过；实际 HLSL 负检查按预期拒绝。
  资源导出/原resident关联确定性专项均通过。未改 Rust/cache，复用 C2-B 17项
  完整像素/取消/预算及历史证据，不重跑缓存或A/B矩阵。未测试的平台不写成通过。
- C1历史取消/像素失败、C2-A抓图崩溃、历史pendingMoved缺现场仍未归因。
  本轮全通过不改变这些历史状态，没有归咎用户、Qt或驱动。
- 完整Release部署至 `artifacts/issue-2-main-c3-shader/release-6e6e35f/`，
  含Qt DLL、QML、平台插件、LICENSE、BUILD.txt、SHA256.json和start-isolated.ps1。
  EXE SHA256为 `f36fa7933b098be82341a438ba6c310204bb9fd371258749b0b7d6b4da2cf3f3`。
  SDK路径从PATH移除后，包内正式EXE以隔离profile和原 `--preview` 入口启动、
  绘制、保存预览并正常退出0（`package-smoke/`）；这只是部署冒烟，非新增性能样本。
  windeployqt退出0；`--no-translations`仍报告翻译源路径警告，未影响Qt/QML部署和冒烟。
  旧包和工作树均保留。正式main.cpp完整启动耗时没有成对测量，不能宣称启动加速。

可复制命令（先使用现有Developer Shell，Qt SDK无需升级）：

```powershell
cmake --build build --config Release --target qt_move_start_tracking qt_transform_tests drawverse -j2
python tools/check_move_shader_artifacts.py .tools/qt/6.8.3/msvc2022_64/bin/qsb.exe build/ui/.qsb --precompiled --executable build/bin/Release/drawverse.exe
python tools/measure_move_start_tracking.py <before-exe> <fresh-before-directory> --shader-pair --trace --source-root <before-worktree>
python tools/measure_move_start_tracking.py <after-exe> <fresh-after-directory> --shader-pair --trace
# 修正后的真实QML路径：两侧都用相同最终测试源和下列选项，目录必须全新
python tools/measure_move_start_tracking.py <exe> <fresh-directory> --shader-pair --main-qml --production-cache --trace
# 去掉--trace得到有限详细诊断关闭对照；每次目录仍必须全新
python tools/analyze_move_start_tracking.py <evidence-root> <fresh-analysis-directory>
```

**因果与停止结论**：只改变两份D3D产物，完整输出一致，首次图形长段和对应GUI交付等待
同时缩短，第二次使用没有同样长段，支持运行时HLSL编译是这组首次成本的重要贡献。
没有D3DCompile/驱动的独立调用栈，不能逐毫秒分割全部成本，也不能排除剩余驱动/帧同步。
没有增加准备、定时器、线程/回执策略或提前预热来转移成本。持续跟随/收尾数值重叠，
其中after-3 W2收尾39.305ms并不优于before；未宣称全面流畅或修复所有起拖问题。
学生真实作品、实体输入、GPU fence、物理显示延迟仍未验收。
本轮到构建修复和有限证据为止，不继续改调度、缓存资格或调参路径。
