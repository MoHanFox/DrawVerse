# Issue #2-B — Color-only State Invalidation

## 版本、边界与状态依赖

A 的 [PR #3](https://github.com/MoHanFox/DrawVerse/pull/3) 已正常 merge commit 合并，执行后重新 fetch 确认 upstream/develop=`ead85b36eb76df345427407abaad99165d24622b`。B 分支 `fix/issue-2b-color-state-invalidation` 从该提交创建；没有继续在 A 分支添加产品修改，也没有修改 master/pre-master 或关闭 [Issue #2](https://github.com/MoHanFox/DrawVerse/issues/2)。

- 修复前产品：上述 A merge；叠加双方共用测量探针提交 `91322cea0065732bee83be00f29fa3c3849b06f4`，before 二进制编译 revision 为该探针提交。
- 修复后产品/测量二进制：`f83a2de0a53563ca01571be2db1e578fdcfe312d`。产品差异仅 `BrushLibrary::setActiveColors` 的失效边界，另含专项测试和统一离线分析器。后续测试/文档收尾不改变该产品代码或测量二进制。
- 两侧 runner、Qt harness、记录器文件 SHA256 完全一致；每个 invocation 记录同一个 tool revision=`91322ce…`、实际 product revision、Git HEAD 和二进制摘要。14 次启动时源码树差异摘要均为空，旧报告没有作为本轮 before 数据。
- before exe SHA256：`f30539d520f5a41c307666c805aeeaeb89acdd1f7f6d6cc7ca84347ef515eafa`；after：`4eb55abf39a4274d3de97518de922576127040ab14171d3da14a708e0b4662b5`。

颜色存于 BrushLibrary 的全局 foreground/background 及偏好顶层 `colorState`；完整偏好另含预设、resetState、工具隔离 profiles、cursor/recentColors。执行目录只含 version/selected/presets/organization；工具 profiles 不含活动颜色。参数、目录结构、工具切换等仍通过既有路径使缓存失效。修复仅对**已有有效对象缓存**局部更新 colorState，清除编码偏好，保留执行目录；缓存为空时保持为空，让下一次 snapshot 正常全量重建。没有用“最近是颜色修改”替代已有脏状态。

`activeColorsChanged` 的顺序、35ms timer、完整 persistRequested、PaintCoreClient 校验/目录比较/后台保存均保留。客户端同步取得活动颜色，普通笔触起笔捕获新色；DocumentManager 仍在提交时广播完整偏好、保持各文档工具上下文且只有一个磁盘写入者。接收端不满足既有快速校验时仍完整恢复，未优化跨文档恢复。湿混等既有载色策略没有改成“所有工具立即使用新色”。详见对应 [设计](../../../brush/performance/color-state-invalidation-design.md)。

## 同条件方法与命令

2026-10-23，本机 Windows 10 19045 / Ryzen 7 5800H，复用 Qt 6.8.3 MSVC2022 x64、MSVC 19.43.34809、SDK 10.0.22621.0、Rust 1.99.1、CMake 4.4.4、Ninja 1.12.1、Python 3.10.8。Release、Windows 可见窗口1280×800，画布512×384，zoom=1，逻辑DPI=96、DPR=1，面板/当前 round-pressure 与原 A 工具一致。所有进程均在实际初始化日志选择 AMD Radeon(TM) Graphics / vendor1002 / device1638 / flags0、Prefer software device=0，运行接口为 D3D11；没有仅依据环境变量断言，也没有软件全矩阵。

before 独立工作树 `E:/Git/DrawVerse-issue2b-before`，after 复用 Fork 英文入口。各自独立 build 和 core/target，Qt SDK/下载缓存共用，两个目录保留供审阅，未覆盖二进制或清理旧数据。未升级/安装工具。

只测 A0/A1/A2，首次1次和后续50次分离。A0请求16ms间隔；A1/A2为匹配的125ms离散节奏（实际间隔见表），真实 CanvasItem/QML 窗口合成鼠标吸色；生成红蓝色块和绘制/撤销/重做冒烟在计时外。默认4预设；256档通过 saveCopy 从 round-pressure 生成252个副本，固定名字 issue2a-004…255，每次选回原预设，UUID由合法接口生成，规模/参数规则固定而非逐字节相同库。它不代表学生真实资源。

每档3组 before/after，顺序为 before-1/after-1、after-2/before-2、before-3/after-3；最后 after-256-off、before-256-off 各1次无诊断对照。每次独立进程/隔离 INI、workspace、TEMP/TMP 和 QML 缓存；图形磁盘缓存关闭。raw 保留所有样本和失败字段，最终实际 INI 颜色核对独立在计时/trace 结束后完成。最终均为前景红、背景白。

复现时先激活既有 MSVC、CMake、Ninja 进程环境；使用新的英文路径和不存在的输出目录，Qt DLL 路径加入进程 PATH：

```powershell
$tool='91322cea0065732bee83be00f29fa3c3849b06f4'
$productBefore='ead85b36eb76df345427407abaad99165d24622b'
$productAfter='f83a2de0a53563ca01571be2db1e578fdcfe312d'
$qt=(Resolve-Path .tools/qt/6.8.3/msvc2022_64).Path
$env:PATH="$qt/bin;"+$env:PATH
git worktree add --detach ../issue2b-before-new $tool
git worktree add --detach ../issue2b-after-new $productAfter
foreach($side in 'before','after') {
  cmake -S "../issue2b-$side-new" -B "../issue2b-$side-new/build" -G Ninja -DDRAWVERSE_BUILD_UI=ON -DCMAKE_BUILD_TYPE=Release "-DCMAKE_PREFIX_PATH=$qt"
  cmake --build "../issue2b-$side-new/build" --target qt_brush_interaction_baseline -j 2
}
# 对 presets=0、256 和 repeat=1..3，按上述顺序交替调用；以下是一次 before 示例
python ../issue2b-before-new/tools/measure_brush_interaction.py ../issue2b-before-new/build/bin/Release/qt_brush_interaction_baseline.exe artifacts/issue-2b/new-pairs/before-4-1 --a2-only --trace --presets 0 --count 51 --repeats 1 --product-revision $productBefore --tool-revision $tool
# after 同样参数，换 executable/output/product-revision；256关闭对照各去掉 --trace
python tools/analyze_color_interaction.py artifacts/issue-2b/new-pairs
```

本轮逐次启动脚本和顺序为 `artifacts/issue-2b/run-paired.ps1`、`paired/run-order.json`；未提交本机绝对路径脚本。离线分析器对双方同一套原始字段计算，queue_us 使用 point.value 而非零长度 point span。内部 span 用单调时钟，GUI 与后台线程分别记录，operation 捕获关联保存请求；不相加嵌套/并行耗时。心跳是事件循环代理，收尾为观察时间，均不代表输入/显示延迟。

## 每次运行摘要

14进程×3场景×51=2,142次尝试；全部完成，失败/超时/trace溢出0。诊断关闭组 trace 为空，所有运行最终颜色实际保存一致。以下耗时均毫秒；内部事件统计含首次，共51条，输入统计单独列首次与后50条。

| 版本/库/重复 | A1首次 ms | A1后50次 p50/p95/max ms | A2首次 ms | A2后50次 p50/p95/max ms |
|---|---:|---|---:|---|
| before-4-1 | 3.965 | 0.615/0.741/0.860 | 1.531 | 1.557/1.970/4.445 |
| after-4-1 | 4.970 | 0.612/0.816/0.997 | 1.585 | 1.541/1.859/3.935 |
| before-4-2 | 5.016 | 0.619/0.787/0.791 | 1.799 | 1.536/1.859/3.972 |
| after-4-2 | 4.896 | 0.620/0.712/0.851 | 1.595 | 1.543/1.833/3.954 |
| before-4-3 | 5.260 | 0.624/0.789/0.818 | 1.707 | 1.561/1.855/4.074 |
| after-4-3 | 3.132 | 0.617/0.795/0.818 | 1.437 | 1.558/1.998/4.253 |
| before-256-1 | 4.803 | 0.610/0.711/0.768 | 1.563 | 1.714/2.198/4.232 |
| after-256-1 | 5.068 | 0.625/0.818/0.869 | 1.628 | 1.611/1.952/4.585 |
| before-256-2 | 4.979 | 0.612/0.774/0.846 | 1.719 | 1.746/2.039/4.400 |
| after-256-2 | 4.950 | 0.621/0.803/0.960 | 1.503 | 1.546/1.851/3.802 |
| before-256-3 | 4.795 | 0.618/0.865/0.871 | 1.600 | 1.774/2.280/4.481 |
| after-256-3 | 4.899 | 0.622/0.812/0.880 | 1.663 | 1.587/2.027/4.710 |
| before-256-off | 4.895 | 0.622/0.804/0.839 | 1.852 | 1.776/2.287/4.340 |
| after-256-off | 5.279 | 0.623/0.790/0.939 | 1.808 | 1.547/1.962/4.411 |

每个输入场景尝试/完成均51/51，A1实际变更0，A2实际变更51；每行首次n=1，不计算首次p95。

| 运行 | snapshotObject调用/重建/复用 | 对象重建或复用 p50/p95/max ms | 目录同步 p50/p95/max ms | 目录重建/复用 | 新准备票据 | 后台保存 p50/p95/max ms |
|---|---|---|---|---|---:|---|
| before-4-1 | 51/51/0 | 0.141/0.164/0.178 | 0.054/0.068/0.070 | 51/0 | 0 | 3.101/3.419/3.601 |
| after-4-1 | 51/0/51 | 0.023/0.038/0.050 | 0.002/0.003/0.019 | 0/51 | 0 | 3.163/3.403/3.744 |
| before-4-2 | 51/51/0 | 0.141/0.168/0.179 | 0.052/0.062/0.066 | 51/0 | 0 | 3.180/3.482/3.644 |
| after-4-2 | 51/0/51 | 0.022/0.039/0.042 | 0.002/0.003/0.004 | 0/51 | 0 | 3.155/3.469/3.581 |
| before-4-3 | 51/51/0 | 0.137/0.169/0.186 | 0.052/0.061/0.063 | 51/0 | 0 | 3.201/3.409/3.516 |
| after-4-3 | 51/0/51 | 0.022/0.035/0.041 | 0.002/0.003/0.005 | 0/51 | 0 | 3.108/3.489/3.537 |
| before-256-1 | 51/51/0 | 3.576/3.726/3.982 | 1.690/1.758/1.794 | 51/0 | 0 | 7.553/8.060/8.296 |
| after-256-1 | 51/0/51 | 0.024/0.031/0.040 | 0.010/0.015/0.033 | 0/51 | 0 | 7.550/7.995/8.978 |
| before-256-2 | 51/51/0 | 3.489/3.643/4.112 | 1.658/1.730/1.757 | 51/0 | 0 | 7.612/8.402/9.341 |
| after-256-2 | 51/0/51 | 0.024/0.037/0.058 | 0.010/0.020/0.021 | 0/51 | 0 | 7.655/8.127/8.463 |
| before-256-3 | 51/51/0 | 3.515/3.678/4.056 | 1.645/1.696/2.023 | 51/0 | 0 | 7.605/8.003/8.047 |
| after-256-3 | 51/0/51 | 0.023/0.036/0.037 | 0.009/0.015/0.047 | 0/51 | 0 | 7.658/8.124/8.455 |

有样本的耗时项各n=51，未走的路径n=0；重建与复用按真实路径统计，目录复用指完整编码字节复用。颜色缓存保留观察：before每组0/51，after每组51/51。诊断关闭组不收集内部事件，不能把事件数0解释为没有执行。

| 运行 | A0心跳 n; p50/p95/max ms | A1心跳 n; p50/p95/max ms | A2心跳 n; p50/p95/max ms | A1/A2实际间隔 p50/p95/max ms | A2收尾观察 ms |
|---|---|---|---|---|---:|
| after-256-1 | 95; 10.07/17.00/17.80 | 634; 10.82/11.40/18.31 | 668; 10.99/13.88/19.15 | 126.78/127.43/127.59; 137.15/148.29/149.10 | 136.31 |
| after-256-2 | 95; 10.22/17.03/18.00 | 635; 10.88/11.30/17.00 | 658; 10.99/13.91/18.01 | 126.76/127.48/127.66; 137.22/148.39/149.05 | 125.78 |
| after-256-3 | 97; 10.06/17.02/17.99 | 631; 10.92/11.31/18.04 | 653; 10.99/13.89/17.03 | 126.88/127.42/127.45; 136.30/148.26/148.60 | 135.67 |
| after-256-off | 93; 10.06/16.88/17.54 | 631; 10.86/11.25/18.50 | 645; 10.99/14.02/17.04 | 126.89/127.38/127.58; 137.02/148.07/148.55 | 126.00 |
| after-4-1 | 94; 10.17/17.08/17.99 | 633; 10.97/11.32/17.97 | 631; 10.99/14.15/18.00 | 126.85/127.44/127.57; 136.43/146.60/148.04 | 145.95 |
| after-4-2 | 98; 10.04/16.99/17.84 | 621; 10.93/11.29/18.02 | 640; 10.99/14.02/17.00 | 126.88/127.29/127.52; 127.44/138.36/149.49 | 147.64 |
| after-4-3 | 95; 10.06/16.99/17.50 | 638; 10.82/11.34/18.02 | 664; 10.98/13.52/17.34 | 126.78/127.59/127.75; 136.68/147.83/148.54 | 125.76 |
| before-256-1 | 91; 10.38/17.01/18.05 | 629; 10.93/11.26/17.99 | 660; 10.98/14.09/19.01 | 126.83/127.52/127.57; 137.55/147.55/149.70 | 135.46 |
| before-256-2 | 97; 10.07/16.90/18.06 | 635; 10.90/11.21/17.75 | 663; 10.90/14.78/17.92 | 126.99/127.51/127.85; 137.43/146.72/149.54 | 134.60 |
| before-256-3 | 96; 10.05/16.62/17.05 | 633; 10.87/11.35/18.08 | 669; 10.94/13.90/17.18 | 126.99/127.59/127.66; 137.37/149.13/160.71 | 146.75 |
| before-256-off | 94; 10.07/17.01/18.03 | 633; 10.97/11.27/16.97 | 657; 10.88/14.00/17.99 | 126.91/127.59/127.76; 136.36/147.90/148.67 | 137.07 |
| before-4-1 | 92; 10.09/17.02/18.00 | 632; 10.85/11.33/18.88 | 642; 10.99/14.02/18.02 | 126.97/127.59/127.63; 136.32/147.82/159.59 | 136.39 |
| before-4-2 | 93; 10.57/16.99/18.03 | 623; 10.95/11.28/17.03 | 643; 10.99/14.03/17.32 | 126.71/127.47/127.63; 136.85/147.76/148.09 | 125.69 |
| before-4-3 | 92; 10.11/16.97/17.01 | 634; 10.90/11.35/17.95 | 651; 10.99/13.92/16.86 | 126.87/127.40/127.53; 136.77/138.49/148.44 | 136.43 |

## 正确性与结论

- 新 `qt_color_state_tests` 最终7 passed/0 failed/0 skipped（含初始化/清理）：真实 QML 前后景色块同步更新、真实色块点击交换；同色 no-op、缓存快照真实 JSON/目录等价；冷/热缓存以及颜色在参数/切换/新预设/工具隔离前后的同窗口交错恢复；在 pending=true、无中间事件循环等待时起笔的蓝/红真实像素；实际保存并重新打开后绿色/黄色及 radius=13/flow=.41；真实 DocumentManager 非写入文档改色、单写入者保存/广播、各文档半径3/7及工具上下文保持。旧产品运行新断言时，缓存测试因3次颜色导致3次对象重建而失败，其他原始正确性测试通过；修复后不放宽断言。
- 既有 `activeAndRecentColorsPersistAcrossRestartAndStayOutOfPresets`、`colorDynamicSettingsUseCapturedForegroundBackgroundAndRoundTrip` 通过（同进程 totals4 passed）；颜色动态专项验证起笔后改色不会改变已捕获笔触的真实像素与恢复。两个双载色专项明确 QSKIP“双色绘制已删除”，没有记作通过或恢复该能力。
- 既有 `sharedBrushPreferencesRetainEachDocumentsIsolatedToolWithoutFeedback` 3 passed；`wetSettingsPageManualInkAndPreviewStaySeparateFromCanvas` 3 passed（含初始化/清理）。湿混实际像素证实：载入红色后改选蓝色仍画红，清空/重新载色后画蓝，撤销/重做和预览隔离保持。
- 运行命令：`cmake --build build --target qt_color_state_tests qt_ui_tests qt_workflow_tests -j 2`，在隔离 TEMP/TMP、QML缓存和 `QT_QPA_PLATFORM=windows` 下分别执行上述明确函数名；未运行全量 tools/check.py。14个测量进程各自还完成计时外绘画/撤销/重做/退出。新增 QML 测试早期有 resize 尚未布局的越界点击警告；最终夹具先等待控件坐标落入所属窗口后再改色，最终7 passed且无该警告，未改产品布局。

**证据支持的改善**：在有效缓存下，颜色变化不再引发不必要的全库对象重建/执行目录重建。256档每次 A2 的 snapshotObject 方法仍调用51次，实际重建51→0、复用0→51；重建路径p95为3.643–3.726ms，复用路径p95为.031–.037ms。目录同步p95为1.696–1.758→.015–.020ms，两侧新 Rust 准备票据都为0；不是“消除每次吸色的 Rust 编译”。

**没有夸大的部分**：256档同步输入p95由2.039–2.280降至1.851–2.027ms，但幅度远小于后续工作变化，max有两组上升；默认4项同步p95重叠，第三组after略高，不能声称普遍同步延迟改善。必要偏好编码仍执行，256档 encodeSnapshotObject p95由.259–.268变为.336–.484ms，未改善；后台保存p50仍约7.55–7.66ms，GUI心跳没有明确整体改善。保存请求队列p95范围.315–.733ms，与GUI并行，不归入输入同步时间。没有将任何这些数据称为屏幕呈现延迟。

256关闭诊断对照的A2 p95：before2.287、after1.962ms；与各自开诊断组大体接近，单次关闭对照不足以分离纯埋点扰动，不声称零开销。关闭组仍有共同测试观察成本。原 A 表格不替换成本轮统计。

未提供学生真实笔刷资源、实体手绘板；没有真实库外推、跨机器统计或输入到显示关联测量。参数性能、方向抖动、WinTab/Ink、图层/GPU架构均不在本修复范围。仅能声称上述颜色失效边界改进，不能声称学生报告的全部卡顿、调参或轮廓抖动已修复。后续优先在合法真实笔刷资源下验证残余卡顿；不扩大本次实现。

## 证据与停止

忽略目录 `artifacts/issue-2b/paired/` 含每次 invocation/summary.json、summary.csv、run-1/raw.json、test.txt、qt.log、计时外 window.png，以及合并 analysis.json/analysis.csv、run-order.json。构建日志、regression-before.txt、correctness-ui-final/color-state.txt、existing-specialties/、wet-specialty/ 均在 `artifacts/issue-2b/`。没有覆盖 issue-2a 数据、提交配置/真实资源/二进制，未改配置格式、state版本、Rust ABI、依赖或锁文件。

B 仅提交并推送 Fork 分支留待审查，不创建/合并 B PR，不关闭 Issue；本阶段到此停止。
