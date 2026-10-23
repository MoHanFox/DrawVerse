# 主线 A · Brush Parameter Responsiveness 验收

2026-10-24。实际产品基线为 C3 PR #10 的 merge commit
`a7fb6773cce94769a07bc7a1a6bf274bc7088ca3`；分支
`perf/issue-2-brush-parameter-responsiveness`。本轮只对应 Issue⑤。

## 修改前证据与范围

默认4/固定256预设规则复用 A：反复选择 round-pressure，再用合法 saveCopy 创建
issue2a-NNN 名称，恰好256项，最后选择 round-pressure。真实 Main.qml、1280×800、
512×384画布、实际 D3D11 AMD 1002:1638、DPR1、DPI96，不使用实体手绘板。
控件的窗口事件逐个登记实际对象，发现非计划输入/窗口失活则停止，不吞真实输入。

小样本 `pilot-256-v2`：大小、流量、不透明度各同值/离散/连续3次，共27次输入，
各独立目标与实际保存终值一致；另 pressureOpacity 真实开关一次改变成功。
三类离散同步约1–2ms；每次偏好对象构造约3.5–3.6ms后，目录又构造同样256行
约1.6ms。光标刷新约0.03–0.06ms。异步准备仍发生，不能跳过。
这些少量样本不用于稳定p95或真实资源结论。

首次默认库试跑 `pilot-default` 保留失败：流量轨迹恰落 .605 的 .01 量化中点，
oracle 的浮点舍入方向与控件位置运算不同。测试修订为内部位置略高于中点
（+0.00001），起点.38/终点.83保持，仍独立计算期望；不改产品或阈值。
同时使测试 teardown 的 Hide 不被误标为运行期间干扰，并避免失败后补发未执行终点。
这不解释任何历史学生问题。

新增精确参考测试在未优化代码中失败：
`catalogReusesCurrentPreferenceRowsWithoutChangingState` 观测到额外
`catalogSnapshot.presetRows=1`，期望0；完整目录像素无关，比较完整JSON和原冷路径。
原始证据 `artifacts/issue-2-brush-parameters/duplicate-before.txt`。
这证明重复构造，不证明学生卡顿的全部根因。

设计与独立参考边界见 [设计](../../../brush/performance/brush-parameter-responsiveness-design.md)。
后续测量与最终验收在本记录追加，历史数据保持。

## 最终版本与实现（2026-10-24）

- C3 已正常合并 [PR #10](https://github.com/MoHanFox/DrawVerse/pull/10)，准确 head
  `dc5138b5e895bbd48b49c17d88e01076a6747694`，merge/base 为上述 a7fb677；
  构建前提和OFF回退已进入常规说明，Issue原进度评论仅③补充本轮边界，Issue保持开放。
- 共用窗口探针/runner/初版分析器：`0859a85ea6b5c294b1103f5b9969d0033a4a62ad`。
  before独立工作树 `E:/Git/DrawVerse-parameters-before`，自己的build和core/target，
  HEAD0859a85，无未提交修改；产品是a7fb677加相同默认关闭诊断。
  after仅增加 `38f8df2522e570ce92ef0afbf84b58e820474a29` 的11行产品修改。
- 正式12进程中，runner/Qt探针/InteractionTrace/参数辅助头摘要全部相同，
  见 `identities-threaded.json`。before/after探针EXE分别为
  `32da618f41ba0561f21a97d8b8d61f55b312116a650835b7787f9dfc4a9c3d59` /
  `05b76bc1550efaf8d14ebf9ffd98b1181deab05fb84d50d7e50bc549daa73c25`。
  CMake/Ninja Release、Qt6.8.3/MSVC x64，原Rust与SDK复用。实际原生可见Main.qml，
  D3D11 AMD Radeon Graphics 1002:1638、DPR1/DPI96、1280×800、
  512×384画布、zoom1，各invocation和Qt初始化日志保留。
- 离线分析收尾 `8b3147026fb70382bae01bd8c6b1a624caf81984` 按实际input.dispatch
  线程区分GUI与并行后台对象构造，对双方相同原始记录重新离线分析，不重测输入。
  保留初版all-thread字段/目录，当前表格使用GUI字段。after-256-1同值段曾重叠一个
  worker线程4行冷目录构造，operation=-1；不能把它归给同值操作或混入GUI热点。
- 最终探针 `7285590c784b4cd3878ef6da0188b1e2525b92ee` 仅修失败记账：
  未尝试的收尾不标成超时，已超时输入不再被收尾重复计数。成功路径和产品未变。
  单独重建后3场景×3次诊断关闭冒烟通过，见final-tool-smoke；不将此新工具身份
  改写进上述正式数据或重新声称测了性能分布。

**唯一产品修改**是从本对象仍有效的偏好对象复用活动预设行及其版本/组织。
目录对象仍生成并编码，不把“catalogSnapshot调用”当作“全库行重建”。
`queuePersistence`、35ms、防抖到期、参数通知、目录失效/准备、保存与广播均未改。
冷缓存仍用旧循环。该对象的全部赋值入口已经核对：

|入口|保持正确性的边界|
|---|---|
|snapshotObject|活动presetRow与原目录同一实现、同一版本计算；reset/其他工具版本分开|
|queuePersistence / restoreObject|参数、结构或整体恢复清除对象，不能复用旧行|
|restoreSelectionOnly|只接受与自己已验证对象精确一致的状态，当前选择单独更新|
|prepare/applyPreparedPreferences|候选先完整恢复/验证并生成快照与目录，原子采用同一组对象|
|BrushToolIsolation::setToolContext|原有活动行/版本投影或清空路径保留，不借用其他工具参数|
|setActiveColors|只修colorState，活动目录行未变；此前参数脏状态不会被颜色覆盖|

没有新增缓存框架、ABI、版本、依赖或锁文件变更。没有更改绘画算法、回执、
预热、画质、线程或图层缓存资格。

### 同条件正式对照

各库按 before1→after1→before2→after2→before3→after3 交替。每行51次；首次n=1单列，正常段n=50，单位ms。实际变化分别为同值0、离散51、连续50；每行51/51完成，失败/超时0，终值均正确。

|库/进程|场景|首次同步|后50次同步p50/p95/max|
|---|---|---:|---|
|before-4-1|radius-same|0.605|0.518/0.677/0.689|
|before-4-1|radius-discrete|1.855|2.110/2.720/4.838|
|before-4-1|radius-drag|0.469|1.530/2.126/4.346|
|after-4-1|radius-same|0.730|0.507/0.650/0.697|
|after-4-1|radius-discrete|1.739|2.095/2.596/5.941|
|after-4-1|radius-drag|0.443|1.503/1.824/4.253|
|before-4-2|radius-same|0.687|0.541/0.693/0.717|
|before-4-2|radius-discrete|1.788|2.111/2.781/6.456|
|before-4-2|radius-drag|0.462|1.515/1.725/4.837|
|after-4-2|radius-same|0.640|0.623/0.777/0.852|
|after-4-2|radius-discrete|1.749|2.060/2.652/4.852|
|after-4-2|radius-drag|0.540|1.558/2.144/4.574|
|before-4-3|radius-same|0.693|0.528/0.686/0.720|
|before-4-3|radius-discrete|1.659|2.121/2.881/4.880|
|before-4-3|radius-drag|0.481|1.439/1.750/4.536|
|after-4-3|radius-same|0.594|0.577/0.750/0.780|
|after-4-3|radius-discrete|1.723|2.089/3.101/4.625|
|after-4-3|radius-drag|0.432|1.446/1.752/4.241|
|before-256-1|radius-same|0.585|0.534/0.687/0.735|
|before-256-1|radius-discrete|1.729|1.976/2.471/4.523|
|before-256-1|radius-drag|0.466|1.481/1.713/4.084|
|after-256-1|radius-same|0.585|0.518/0.648/0.697|
|after-256-1|radius-discrete|1.914|1.970/2.725/5.254|
|after-256-1|radius-drag|0.468|1.492/1.760/4.338|
|before-256-2|radius-same|0.666|0.572/0.709/0.897|
|before-256-2|radius-discrete|1.766|1.971/2.346/5.716|
|before-256-2|radius-drag|0.485|1.486/1.825/4.425|
|after-256-2|radius-same|0.602|0.562/0.654/0.754|
|after-256-2|radius-discrete|1.736|1.940/2.655/4.304|
|after-256-2|radius-drag|0.443|1.468/1.659/4.040|
|before-256-3|radius-same|0.689|0.573/0.764/0.779|
|before-256-3|radius-discrete|1.940|1.979/2.591/4.717|
|before-256-3|radius-drag|0.470|1.494/1.762/4.771|
|after-256-3|radius-same|0.584|0.578/0.736/0.758|
|after-256-3|radius-discrete|1.793|1.951/2.155/4.781|
|after-256-3|radius-drag|0.451|1.446/1.700/4.338|

离散后的GUI工作（ms）。下列span嵌套，**不能相加**；目录对象仍构造/编码，减少的是其内部全库预设行构造。准备是准确票据请求→worker完成观察，不是纯Rust执行或显示延迟。每行51个请求全部匹配。

|库/进程|偏好对象重建p50/p95/max|目录对象p50/p95/max|额外序列化行/复用行|目录准备观察p50/p95/max|
|---|---|---|---:|---|
|before-4-1|0.143/0.159/0.180|0.051/0.058/0.077|204/0|0.464/0.711/1.391|
|after-4-1|0.141/0.163/0.174|0.015/0.017/0.029|0/204|0.468/0.682/1.149|
|before-4-2|0.142/0.151/0.171|0.052/0.059/0.062|204/0|0.480/0.675/1.388|
|after-4-2|0.141/0.163/0.183|0.015/0.016/0.021|0/204|0.477/0.642/0.720|
|before-4-3|0.141/0.167/0.177|0.051/0.058/0.062|204/0|0.475/0.650/2.248|
|after-4-3|0.138/0.149/0.166|0.015/0.017/0.018|0/204|0.498/0.656/0.764|
|before-256-1|3.609/3.738/3.779|1.736/1.837/2.011|13056/0|13.106/21.489/21.607|
|after-256-1|3.625/3.768/4.087|0.065/0.082/0.096|0/13056|15.298/22.388/35.133|
|before-256-2|3.567/3.756/4.167|1.714/2.245/3.719|13056/0|15.188/21.498/22.260|
|after-256-2|3.574/3.745/3.817|0.063/0.078/0.089|0/13056|13.877/20.902/21.875|
|before-256-3|3.539/3.734/5.334|1.736/1.825/2.490|13056/0|16.343/22.179/22.881|
|after-256-3|3.593/3.807/3.837|0.066/0.082/0.088|0/13056|14.576/22.325/22.988|

实际节奏和收尾观察（ms）。连续段是GUI事件循环驱动，目标8ms；离散段允许保存结束，实际间隔包含测试等待。同值目标8ms，全部原始节奏仍保留。收尾观察包含既有80ms信号安静判定和INI读取，**不当作产品完成/显示延迟**。

|库/进程|离散间隔p50/p95/max|连续间隔p50/p95/max|离散心跳p50/p95/max|连续心跳p50/p95/max|Release同步|拖动收尾观察|
|---|---|---|---|---|---:|---:|
|before-4-1|132.402/144.162/144.327|10.006/10.800/12.973|10.989/11.669/13.995|10.057/11.029/12.848|0.394|117.525|
|after-4-1|132.242/142.941/144.889|10.014/10.525/14.005|10.996/12.880/13.996|10.135/11.130/12.858|0.458|119.602|
|before-4-2|132.703/143.821/144.291|10.002/10.746/14.007|10.993/12.832/13.984|10.091/11.017/13.281|0.440|129.673|
|after-4-2|133.154/143.332/144.437|10.007/11.102/14.024|10.994/12.492/13.985|10.241/11.165/13.206|0.528|118.949|
|before-4-3|133.656/143.203/143.724|10.007/10.509/13.547|10.995/12.153/13.798|10.098/11.007/13.156|0.431|128.899|
|after-4-3|132.393/144.087/144.473|10.010/10.657/14.006|10.990/12.523/13.329|10.117/11.004/12.963|0.466|117.124|
|before-256-1|140.226/150.347/151.873|10.007/10.625/12.845|10.995/17.999/22.008|10.069/11.017/19.028|0.463|139.117|
|after-256-1|138.668/149.678/151.307|10.006/10.707/14.018|10.990/16.023/18.003|10.102/11.028/18.030|0.425|139.514|
|before-256-2|140.620/153.373/154.173|10.007/10.571/14.008|10.989/17.922/22.023|10.070/11.133/19.030|0.388|127.898|
|after-256-2|138.963/149.701/150.222|10.013/10.461/13.857|10.994/16.015/17.970|10.182/11.142/18.012|0.466|139.273|
|before-256-3|140.226/151.109/153.074|10.003/10.345/13.025|10.991/17.973/21.021|10.069/11.002/20.005|0.477|129.839|
|after-256-3|139.031/150.826/152.095|10.007/10.733/13.630|10.993/16.117/17.282|10.143/11.095/17.965|0.452|127.768|

### 计数、连续终值与解释

正式12进程共1,836次逻辑操作，实际变更1,212次，全部完成；窗口层登记3,180条
具体计划事件（包含计时外控件定位），无非计划输入/失活、产品错误、失败、超时、
崩溃或记录溢出。原始Press/Move/Release均保留，不把一次点击等同于单条窗口事件。

每进程同值段0次settingsChanged/brushChanged/persistRequested，GUI无新目录准备；
离散段51次变更，各51次通知/保存及51个匹配准备票据。连续段首点是已保存的.38，
后50点改变大小，各50次settingsChanged/brushChanged，停止后一次保存及一次准备；
两侧计数相同。大小此前保存3.5→起点13.5→独立目标308.5，model/QML control/INI均
达到308.5；未只检查两个getter相等。事件节奏实测约10ms中位数，不能称恒定125Hz。

256库每轮离散目录额外构造51×256=13,056行→0，复用13,056行；GUI目录对象p95
1.824–2.245→0.078–0.082ms。偏好对象仍每次完整重建，约3.5–3.8ms中位数。
同步输入约2ms，没有一致改善；连续末尾的总观察段和Release同步也没有一致改善。
准备、保存仍发生，没有将其跳过或拖到起笔才处理。心跳p95在256离散段约18→16ms，
但包含测试观察和INI轮询，不能写成输入到显示延迟改善或学生体验已经修复。
完整后台保存span与准确准备票据见analysis-threaded，不与GUI嵌套区间相加。

### 有限对照与正确性

- 256库诊断关闭before/after各1进程、每场景51次，全部完成。离散同步p50/p95/max
  为1.949/2.263/5.614→1.968/2.637/4.759ms；连续为
  1.580/2.022/4.497→1.450/1.726/4.261ms；离散心跳p95
  17.991→16.010ms。关闭组trace为空，仍有共同的输入/状态/保存观察开销，
  不声称零扰动，也不据单对断言纯诊断开销。
- 修复后controls-after-256再次执行流量/不透明度同值、离散和连续各3次，
  加大小共27次全部完成，pressureOpacity真实开关1次改变成功。
  连续终值各.83，独立oracle、模型、控件与实际INI一致；与pilot-256-v2的
  相同控件路径前后相符。这是有限功能对照，不构成三对参数性能矩阵，
  不外推所有压感开关或实体压力输入。
- `qt_color_state_tests` 4个功能函数通过：新增目录复用/冷精确参考（3种隔离×7状态，
  21次完整JSON/字节对照，冷路径构造断言、真实复用断言）、mixedDirtyStatesRoundTrip、
  beforeDebouncePixelsSaveAndReopen、documentBroadcastRetainsIsolatedToolsAndSingleWriter。
  原循环在新增“额外构造应为0”断言失败；修复后相同条件通过，不将优化特征断言
  的失败称作原产品像素错误。
- `qt_ui_tests` 3个功能函数通过：brushParameterEditsKeepRowsAndImmediateStroke
  （防抖前下一笔实际alpha=.125与行/预览保持）、resetBaselineSurvivesNativePreferencesReopenAndCapturedStroke
  （真实像素验证在途笔触保留已捕获大小、下一笔采用重置大小、保存重开）、
  batchedBrushPreferencesFlushFromActiveDocumentOnCreateAndQuit（创建文档/退出时最后参数保存）。
- `qt_workflow_tests` 3个功能函数通过：sharedBrushPreferencesRetainEachDocumentsIsolatedToolWithoutFeedback、
  brushToolIsolationRoundtripNativeSyncAndInvalidStateAreAtomic、
  partialToolSwitchIgnoresStaleInactiveSharedParameters。保留不同文档工具上下文、
  原子无效输入拒绝及不同工具参数恢复。总计**10个功能函数**，不计init/cleanup；
  本轮没有新增取消/准备策略，旧准备结果的票据检查保持原实现。正式连续变化后
  保存/广播没有恢复旧终值；没有声称人为控制过全部异步乱序。
- Python runner准入2项、线程/票据分析2项通过；增量Release构建与严格生成头检查通过。
  不跑全量check、不重跑A/B/C矩阵，不涉及实体手绘板验证。

### 命令、证据与运行包

先沿用现有Developer Shell、Ninja与Qt进程环境；双方各自构建，无共享Rust产物：

```powershell
cmake --build build --config Release --target qt_brush_interaction_baseline qt_color_state_tests -j2
# below: use the same 0859a85 probe/runner on both products; one fresh directory per process
python tools/measure_brush_interaction.py <exe> <fresh-directory> --parameters --trace --count 51 --repeats 1 --presets 256 --product-revision <product-sha> --tool-revision 0859a85ea6b5c294b1103f5b9969d0033a4a62ad
# omit --presets for default4, omit --trace for the limited off comparison
python tools/measure_brush_interaction.py <exe> <fresh-control-directory> --parameters --parameter-controls --trace --count 3 --repeats 1
python tools/analyze_brush_parameters.py artifacts/issue-2-brush-parameters <fresh-analysis-directory>
python tools/test_measure_brush_interaction.py
python tools/test_analyze_brush_parameters.py
```

原始证据根 `artifacts/issue-2-brush-parameters/`：before/after-4-{1,2,3}、
before/after-256-{1,2,3}、off-before/after-256、pilot-default（失败保留）、
pilot-256-v2、controls-after-256、final-tool-smoke。每进程保留invocation/摘要、
raw、test.txt/qt.log与隔离配置。分析有analysis-main、analysis-final、analysis-threaded
三份独立目录；末者为当前线程口径，旧数据没有被覆盖。全部表格/身份仍可独立审阅，
不只依赖本机日志。像素与保存恢复专项在独立正确性运行，不混入输入同步计时。

完整Release在 `artifacts/issue-2-brush-parameters/release-38f8df2/`：
包含Qt DLL/QML/plugins、LICENSE、BUILD.txt、SHA256.json和start-isolated.ps1，
约79MB/1,346文件（含清单），保留原包。应用EXE SHA256：
`407e6eac4362b66178b689aa79e1dcd436c6e253cde834323b246b9bb9a49cfe`。
先增量drawverse_runtime，再复用相同C3 SDK部署依赖，只替换新应用；launcher按次创建
隔离profile和QML/pipeline cache，不覆盖原用户配置。运行用户仍无需fxc。

移除SDK PATH后，Python显式等待正式EXE的隔离--preview入口，绘制/输出PNG/退出0，
见package-smoke-waited-v2/process.json。早先PowerShell GUI启动没有可靠等待退出，
仅作启动记录；另一Python脚本首次在启动进程前因环境键大小写出错，空目录保留，
不计为产品失败或成功。未把包冒烟写成新增性能样本。

### 结论与未覆盖

**已确认**：当前GUI目录重复构造可以由自己的有效偏好表示替代；完整目录、像素、
保存/恢复和工具隔离专项通过。必要准备与保存没有减少，未新增尾部终值恢复。
**测量支持**：256库的这段GUI目录工作明显减少；同步输入、连续拖动及整体收尾
没有一致改善。剩余完整偏好对象构造仍是本机可见GUI成本，本轮不继续实施第二项优化。
**未证明**：学生实际卡顿的因果链、真实图像资源/复杂笔刷、多文档数量增长下的耗时、
实体设备和物理显示延迟。仅pressureOpacity开关有限对照，不等同于所有压感设置验收。
历史A/C调查的未归因异常保持原记录，本轮不重新调查或声称修复。

产品只有一项局部优化，可提交Fork审查；不创建/合并调参PR，不关闭Issue或继续扩大性能工程。
