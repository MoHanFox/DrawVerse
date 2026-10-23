# 主线 C2-B — Exact Tile Cache Reuse 验收

**当前建议见末尾最新「当前预览契约验证与合并判定」验收节；下文保留的 UNRESOLVED 属于历史结论。**

## 结论与未通过边界

2026-10-23。局部快路径在受限的平面 Normal、整数整瓦片相对平移条件下有效：本轮三对独立 4K 进程的六个最终任务均从 4096 次重建变为 3596 次实际平移复用、500 次精确重建。完整正常帧 execute 为 345.133–359.347 → 7.556–8.175ms；Release→对应完整视口 GUI 接收为 364.373–386.996 → 20.507–49.906ms。这些范围来自下表全部六个值，不是跨机器阈值或整体图层移动承诺。

核心完整像素与确定性取消验证通过；但本轮既有 Qt 组合专项有一个未归因的 pendingMoved 像素失败。准确基线同顺序通过、新版单独运行通过，不能据此排除回归或宣布全部专项通过。该失败、首轮八次意外取消及全部历史异常均保留；C2-B 仅提交供审查，不创建 PR、不合并。

## 合并收尾、版本与可追溯性

| 项目 | 远端结果 / 准确提交 |
| --- | --- |
| C1 [PR #7](https://github.com/MoHanFox/DrawVerse/pull/7) | MERGED，head `22a4183c8317ad0b9e636c56e4ec1fbd57cec2df`，merge `c50279fc3ea85bdb7ebfbd72f83369981b50a131` |
| C2-A [PR #8](https://github.com/MoHanFox/DrawVerse/pull/8) | C1 合并后再处理；MERGED，head `eb3e2c2caf7c3c489cd03d7b3a269a6bdd19b757`，merge `9462f84c10b34daf97296416df41d148468d3475` |
| 本轮产品起点 / 最新 upstream/develop | `9462f84c10b34daf97296416df41d148468d3475`，tree 与 C2-A HEAD 相同 |
| C2-B 新分支 | `perf/issue-2-move-tile-cache-reuse`，从上述 develop 建立 |
| 保留既有 guarded 实现历史 | merge `f9c40b6` 保留旧 `fix/issue-2-move-tile-cache-reuse`（HEAD `8bab6d8aa12bb8ea7b2f38416d73df3e5f1d6969`）的提交；没有改写旧分支 |
| 共用计数与工具 / 产品优化 / 资格取消测试 | `0eb73bcf7789f3c235e138ae02c083855ed5c1e6` / `9338b900234c46b16cece3d2851077cc21426b12` / `173799cf5556ef8f9f422748ec0f9080acc64b49` |
| 本轮共用非纯色夹具 | `969709b`，先提交工具，再应用于独立 before 工作树 |
| 本轮 TILE_SIZE 与固定上层/编辑增量、实际 after 版本 | `1e50b39e22b8a4ddebf78153504c3dd0d56e1c11` |
| 实际 before 版本 | `c30ee4e6ac4c3b6a4de28191b333f684a8a7cd79`，合并后 develop 加共用工具/测试，没有复用快路径 |

两个调查 PR 均核对完整差异、准确 head、报告及现有规则后使用 merge commit；无阻塞检查/审查，不使用管理员绕过或直接 push develop。合并后从远端再次确认 MERGED、fetch 并核对 ancestry。正文使用 Refs，Issue #2 保持开放。GitHub connector 写入返回 integration 403 后，复用已登录的本机 gh 普通权限完成，未扩大权限。master/pre-master 未修改。

依赖历史仍是 C1→C2-A→C2-B，但前两者现在已合并；本分支包含最新合并后的 develop，不再依赖未合并的调查分支。[旧 guarded 报告](guarded-tile-translation-reuse-validation.md)保留其原始版本、表格和时间边界，不作为本轮正式对照替代品。本文件所在的收尾提交只增加设计/验收文档，测量版本以上表为准。

## 资格、精确参考与内存边界

[设计](../../tools/exact-tile-cache-reuse-design.md)和[原资格证明](../../tools/guarded-tile-translation-reuse-design.md)共同约束：缓存是完整合成 Mip，并非单层纹理。所有层必须是平面根 Normal，无组、蒙版、剪贴；恰好一个显式栅格层且无隐式白范围，其余固定贡献无显式瓦片且为透明或覆盖同尺寸画布的均匀隐式白。图层顺序、属性、完整贡献、格式、文档尺寸和采样条件保持，唯一配置变化是证书栅格层的 offset。

位移相对当前缓存配置计算，两轴必须是实际 TILE_SIZE=64 的整数倍；没有场景号、红色、4096 或 (384,128) 产品特例。旧项构造时必须有对应证书；所有显式源的 Weak 分配身份、数量、顺序及有效 opacity 一致，COW 编辑令旧身份失效。不是比较 layer ID 或一个未受保护的裸指针。旧/新均完整位于画布内、同格式同 LOD 且 LOD≤TILE_SIZE.ilog2()，才有相同局部源窗口和采样相位。部分边缘、画布外来源、新露出、淘汰/缺项、非对齐或资格不明均按原路径精确重建。非均匀固定背景/上层直接拒绝，不能将其旧贡献搬走。

独立参考为未改变的 CpuRenderer 精确路径（零缓存预算，无可平移旧项），在同一份移动后 snapshot 和 request 下逐元素比较整个输出；sRGB 逐字节、LinearRgba32F 按现有严格相等断言，不调整算法或放宽误差。数学上的平移等价不是测试替代品。

原 configuration 检查、预算及淘汰保持。映射移交已有 Entry/Mip Arc，不复制 Mip 像素、不预热全图、不强持有源 Tile 像素。独立新表避免原地覆盖和键碰撞；坐标用 i64 checked 运算与 u32 checked 转换。取消在资格阶段保留旧表，移交至提交前留下空表/旧配置，提交后只保留已证明的新表；不发布混合配置或取消帧，下一次精确渲染仍正确。

retained_bytes 沿用原预算的**逻辑保留量**口径，逐项包含 Entry/Mip/source 向量，所有取消检查也核对求和。x64 Entry 新增 24B 元数据，4096 项增加 98,304B（96KiB），256 项增加 6144B；下表报告实际求和。96MiB 保留预算未提高。临时映射最多以旧表条目数申请新表，条目数受原预算约束；旧/新桶短暂并存，Entry 被移动而非另保留一份源图，取消会释放局部表。HashMap 分配器桶/空余容量与实际峰值 RSS **未测，且不属于原 retained_bytes 口径**；不能将逻辑保留量说成进程总内存，或声称峰值总量只有 96MiB/内存零开销。

产品改动限 paint-render 现有缓存。paint-ffi session 的增量仅把现有诊断数据追加到默认关闭的任务记录，没有公开 ABI、state/格式/依赖/锁文件变化。16ms 输入合并、线程池、优先级、预热/取消策略、常驻资格、纹理上限、LOD/采样/正常帧质量以及笔触/历史均未修改。

## 构建、工具一致性与运行条件

before 工作树 `E:/Git/DrawVerse-c2b-exact-before`，after `E:/Git/DrawVerse-local-build`；各自独立 CMake build 与 Cargo target，保留旧对照树。复用 Windows 10 / MSVC 19.43 / SDK 22621 / Rust 1.99.1 / Qt 6.8.3 MSVC x64 / CMake 4.4.4 / Ninja 1.12.1 / Python 3.10.8，没有升级。

| 二进制/工具 | SHA256 |
| --- | --- |
| before qt_layer_move_baseline.exe | `817c5da8fa584fdfc32ec681acb36f3d7e52ac3c22ed3999457754921c174def` |
| after qt_layer_move_baseline.exe | `0ac874094d66a89d25468feb01b6d27c0c8dfafeadc9f23479bd21f1fe7b825a` |
| 两侧 runner，同字节 | `68ad96f06d2a6149b944bdb713664867dcff3b7df77aebe8ccf06f4a81fb57a2` |
| 两侧 C2-A 分析器，同字节 | `71661f4145e8c18acb1ac8eb6287f90b38041166d4534cb74b88ddafd91eae40` |

宿主、FFI 诊断的 Git 规范化文本也逐文件一致，Qt 运行 DLL 摘要一致，见 `shared-tool-provenance.json`。invocation 中旧 `product_baseline=b948dcc...` 是 C1 历史起点，本轮应使用上述准确 before/after SHA 及源码摘要。tracked_diff 均为空摘要；新增文档不影响二进制。

两侧均 Release 原生可见窗口。全部 16 个空闲确认轮进程日志实际创建 D3D11 QRhi，AMD Radeon(TM) Graphics（1002:1638）后明确 using this adapter，没有切独显。1024×720 窗口、656² 视口、DPR=1、logical DPI=96、LQ156M1JW26；4K zoom=0.16015625，小 S1=0.640625。4K 是文档不是窗口尺寸。

S2 满幅 4096² Normal 红色栅格加隐式白底，实际非空范围 [0,0,4096,4096]；S3 保持原 PassThrough 组内层和有遮挡的链接蒙版，未偷换为 Normal 隔离组。pattern 在 S2 满幅上用合法笔触画六个不同颜色的圆斑，半径=尺寸×0.075，x=0.2/0.5/0.8、y=0.2/0.7，RGB=(20+xIndex×25,80+yIndex×60,220−xIndex×35)。核心测试另覆盖透明/半透明细节，不拿纯红性能夹具代替像素证明。

每进程 I=立即进入工具，实际 assets_ready=false/source=0/upload batches=0，走原有未预备交互路径；W=随后确实准备并上传，source=4/upload batches=1，整段保持同源且无新增源上传。不是磁盘冷缓存实验，也没有人为推迟准备。对齐 (384,128)，非对齐 (385,129)，无吸附/取整修改。

输入为 Qt 合成鼠标经真实窗口/CanvasItem 的 GUI QTimer 轨迹，120 Move、目标 8ms；这是**事件循环驱动的受控轨迹，不是外部恒定125Hz输入**。没有系统输入发生器或向其他应用注入操作。I/W 真实节奏如下。相对延后从 Press 返回后的名义计划计算，可有轻微负值；同步 sendEvent 与计划延后单列。

计时/任务/帧归属沿用 C1/C2-A：单调时钟，精确 session/task/view/generation/request/revision/frame 和 COMMIT sequence，GUI 接纳正常完整视口不是物理显示完成。Rust/C++ 各自持续时间；绝对关系仅通过准确同步调用包含区间校准，28 个开启诊断的成功段均得到 bounded 区间，不直接相减两个时钟。execute 包含 setup 与 compose/LOD/blit，资格/映射又在 setup 内，不能重复相加。原先嵌套合成/LOD/blit 未进一步拆开。心跳只是事件循环代理。完成时间戳后才计算整个正常 QImage 数据 SHA256；计时区间不截图、不逐条同步写盘。

## 正确性与专项结果

| 项目 | 本轮结果与准确口径 |
| --- | --- |
| paint-render Release lib/cache/translation | **17 个函数通过**：lib 2、cache 7、translation 8；0 fail/ignore |
| 确定性取消 | 一个 lib 函数内六个时序：资格前/后、第三项映射、提交前/后、第三项重建，均 Cancelled 后再次正常渲染完整像素一致；不计为六个函数 |
| 新增边界 | 固定非均匀上层拒绝、移动后真实绘画/undo/redo、单独 fill 修改完整像素；实际 TILE_SIZE，不依赖 literal 64 资格 |
| 既有核心覆盖 | 333×269 非均匀/透明图案、正负/零/连续/返回、(383,127)/(1,0)、部分边缘/画布外、源编辑、冷热/0/512/25000B 预算、层增删/顺序/可见性/opacity/fill/混合、组及链接/固定蒙版；格式 sRGB/linear，代表 LOD 0/2/6；既有 cache 另含高 LOD/分页/COW |
| paint-ffi 四项 | **复用旧准确 guarded 实现已通过证据**：最终 barrier/单历史、完整帧恢复、旧 view/generation/request/revision/frame 拒绝、原子帧复制；本轮未重复运行，相关产品路径无变化 |
| 本轮 D3D11 三函数组合 | **12 功能数据行 pass、1 fail**；Qt Totals 14 pass/1 fail 含 init/cleanup 两项，不能说 14 功能通过 |
| 有限失败对照 | 准确合并后未优化 before 同顺序：13 功能行 pass；after 该函数单独：1 功能行 pass。原 fail 保留为未归因，见下节 |
| 离线分析 | 7 测试 pass；首次从根目录按模块名调用造成 import 路径错误，改在 tools 目录调用后通过，错误日志保留，没有产品修复 |
| fmt / 严格 Clippy / 构建 | fmt、paint-render Release lib `-D warnings` 通过；两侧相关目标、after 完整 drawverse_runtime 与生成 C 头检查通过 |
| 空闲确认轮原生 | 16 独立 PID、32 committed、每段120实际 Move、最终位移准确、history_delta=1、undo/redo/cancel 检查通过；16 对 I/W 完整正常视口数据摘要前后一致；0 cancel/error/timeout/进程失败/诊断溢出 |

FFI 复用函数为 `layer_move_resident_packets_absolute_updates_and_final_barrier`、`resident_move_skips_coarse_cpu_frames_and_restores_full_resolution`、`viewport_replacement_generation_and_disabled_slots_reject_old_frames`、`frame_copy_capacity_stride_padding_format_and_stale_ids_are_atomic`，日志在上一级历史证据根目录。没有将不同版本历史测试改写成本轮重跑。

## 异常：两轮分开保留

首轮计划 16 个进程：8 成功、8 失败；实际有 27 段（19 committed、8 unexpected_cancel），另外 5 个 W 未执行，不能记为已发满轨迹。八个取消均有同画布/窗口/设备、时间/sequence 对应的 `move.cancel.ApplicationDeactivate` 及 kind=6 CANCEL 回执，最终位移 (0,0)、history_delta=0。失败不是提交完成，未继续空等90秒，也没有靠忽略失焦或抢焦点绕过。无 product_error/timeout/崩溃。失败位置为：

| 场景 | 版本/段 | 实际 Move 数 |
| --- | --- | ---: |
| pattern | after I | 64 |
| small | before W / after W | 23 / 23 |
| aligned-1 | after W | 4 |
| aligned-2 | before I | 108 |
| aligned-3 | before I / after I | 74 / 37 |
| nonaligned | after I | 14 |

这些事件说明取消入口，不能定位哪个外部程序或用户操作引发了失活。用户明确回复桌面可空闲约两分钟后，另建 `idle-confirmed/` 固定执行同一套16进程计划，全部完成；**不是覆盖首轮失败或将成功补跑替换失败**，也不是已证明原取消由用户导致。两轮各自 order、invocation、raw/core、summary、日志和分析完整保留。

首轮所有实际段的收尾摘要如下；数值为 committed 的 Release→GUI ms，“取消(n)”保留实际输入数，“未执行”不计完成：

| 场景/段 | before | after |
| --- | --- | --- |
| aligned-1 I | 384.145 | 16.555 |
| aligned-1 W | 380.997 | 取消(4) |
| aligned-2 I | 取消(108) | 38.606 |
| aligned-2 W | 未执行 | 50.536 |
| aligned-3 I | 取消(74) | 取消(37) |
| aligned-3 W | 未执行 | 未执行 |
| pattern I | 394.646 | 取消(64) |
| pattern W | 413.551 | 未执行 |
| small I | 54.059 | 42.855 |
| small W | 取消(23) | 取消(23) |
| nonaligned I | 424.050 | 取消(14) |
| nonaligned W | 425.688 | 未执行 |
| group-fallback I | 698.208 | 715.154 |
| group-fallback W | 696.119 | 693.426 |
| diagnostics-off I | 377.201 | 45.665 |
| diagnostics-off W | 385.582 | 34.262 |

本轮 Qt 组合专项失败在 `ui/tests/transform_tests.cpp:595`：`!(pendingMoved=capture()).isNull() && white(pendingMoved,{100,1080}) && red(pendingMoved,{800,1080})`。它位于**未准备移动的按住预览、最终 COMMIT 之前**，不同于 C1 历史后续 moved3 采样断言。原日志没有失败时的具体像素值/取消来源，不能从位置推断原因。限定一次准确 before 同三函数顺序、一轮 after 该函数单独：均通过；未改夹具/断言，没有无限重复到绿。结论是**有限复测未再出现，原失败未归因**，并未证明无回归，也未声称 Qt/环境背锅。待审查/下一步只能补该状态的有界证据，不在本轮顺手修预览策略。

日志：`native-regressions/test.txt`、`native-baseline-control/test.txt`、`native-optimized-standalone/test.txt`。三函数为 prewarmed4kResidentMoveReusesFullQualityTexturesAndRestoresFullFrame、complex4kLayerMoveRestoresFullFramesAndHistory、compoundLayerMovesKeepResidentSources，完全未修改测试源码。before/after transform EXE 摘要分别 `272095d710b60b09826c0bbf162634bff25578a8e7ebef30359205872896034a` / `406e21a0c98774fd2fb81ff121b8251326aaa2e1f3b204fcafed16ca55052c47`。

C1 原像素失败/两次历史取消、C2-A 探索抓图崩溃继续在原报告和证据中保留；本轮没有宣称修复。此次没有出现新的进程崩溃。

## 空闲确认轮：全部每次前后数据

下列箭头均 before→after，S2 aligned-1/2/3 前后交替、每侧三独立进程；其余每侧一进程有限代表性对照。I 与 W 不混合。每段尾部/最终任务 N=1，因此直接列值，不对三次首次操作夸大 p95。后两个表的分布来自120个移动样本或119个间隔、各自实际心跳数，不是尾部的分位数。

| 场景/段 | Release→GUI ms | execute ms | compose/LOD/blit ms（内含） |
| --- | ---: | ---: | ---: |
| aligned-1 I | 376.699 → 42.612 | 350.989 → 8.175 | 348.849 → 5.770 |
| aligned-1 W | 386.996 → 20.507 | 354.560 → 7.556 | 352.572 → 5.122 |
| aligned-2 I | 379.523 → 24.567 | 351.324 → 7.922 | 349.030 → 5.562 |
| aligned-2 W | 381.265 → 49.906 | 346.332 → 7.849 | 344.302 → 5.347 |
| aligned-3 I | 385.017 → 23.633 | 359.347 → 7.679 | 357.257 → 5.518 |
| aligned-3 W | 364.373 → 31.136 | 345.133 → 7.848 | 343.030 → 5.747 |
| pattern I | 371.020 → 31.528 | 347.052 → 7.660 | 344.758 → 5.467 |
| pattern W | 363.864 → 33.270 | 346.672 → 7.671 | 344.622 → 5.297 |
| small I | 48.926 → 42.362 | 31.976 → 12.489 | 31.746 → 12.004 |
| small W | 43.191 → 48.892 | 31.680 → 12.855 | 30.515 → 12.149 |
| nonaligned I | 430.236 → 400.994 | 409.126 → 391.758 | 406.896 → 389.385 |
| nonaligned W | 426.993 → 429.417 | 411.440 → 413.185 | 408.865 → 410.696 |
| group-fallback I | 713.407 → 691.072 | 688.313 → 665.150 | 686.176 → 663.161 |
| group-fallback W | 675.582 → 680.758 | 658.503 → 653.754 | 656.436 → 651.678 |
| diagnostics-off I | 380.499 → 35.795 | 未测 → 未测 | 未测 → 未测 |
| diagnostics-off W | 360.881 → 34.796 | 未测 → 未测 | 未测 → 未测 |

| 场景/段 | Press µs | 首 Move ms | 持续移动 ms | 间隔 p50/p95/max ms | 最终节奏延后 ms |
| --- | ---: | ---: | ---: | --- | ---: |
| aligned-1 I | 129.300 → 105.000 | 8.402 → 8.412 | 951.012 → 951.938 | 7.999/8.493/8.984 → 8.001/8.453/8.637 | -0.716 → 0.244 |
| aligned-1 W | 93.900 → 113.500 | 78.017 → 88.535 | 1218.204 → 1282.274 | 12.505/17.060/33.345 → 13.466/16.847/30.377 | 336.127 → 410.696 |
| aligned-2 I | 108.900 → 102.000 | 8.604 → 7.889 | 951.173 → 951.791 | 7.999/8.446/8.887 → 7.992/8.603/8.939 | -0.331 → -0.422 |
| aligned-2 W | 96.100 → 95.600 | 70.805 → 104.372 | 1187.133 → 1191.812 | 11.928/16.747/33.244 → 11.757/16.790/33.409 | 297.842 → 336.089 |
| aligned-3 I | 111.000 → 110.900 | 7.433 → 7.540 | 952.748 → 952.715 | 7.995/8.608/9.057 → 8.001/8.492/8.622 | 0.070 → 0.144 |
| aligned-3 W | 82.700 → 114.000 | 102.499 → 90.051 | 1314.913 → 1270.555 | 13.065/17.235/44.018 → 12.751/16.782/33.057 | 457.329 → 400.491 |
| pattern I | 122.300 → 103.700 | 8.409 → 8.685 | 951.467 → 951.448 | 7.998/8.236/8.581 → 8.000/8.489/8.851 | -0.247 → 0.030 |
| pattern W | 91.700 → 95.900 | 106.438 → 96.910 | 1281.807 → 1216.180 | 13.557/17.177/33.345 → 12.995/16.726/33.096 | 428.154 → 352.994 |
| small I | 95.600 → 120.200 | 7.399 → 7.999 | 952.795 → 952.787 | 8.008/8.293/9.024 → 8.002/8.585/8.743 | 0.098 → 0.665 |
| small W | 87.400 → 89.600 | 124.703 → 93.750 | 1273.906 → 1314.975 | 13.138/16.910/33.471 → 13.467/16.740/32.586 | 438.521 → 448.635 |
| nonaligned I | 101.200 → 111.700 | 7.523 → 8.919 | 952.936 → 952.077 | 7.999/8.994/9.037 → 8.009/9.075/9.425 | 0.358 → 0.885 |
| nonaligned W | 99.900 → 85.800 | 70.829 → 97.619 | 1219.684 → 1222.397 | 12.357/17.528/31.687 → 12.130/16.936/33.200 | 330.413 → 359.930 |
| group-fallback I | 101.900 → 119.700 | 8.546 → 8.316 | 951.423 → 951.170 | 7.997/8.982/9.096 → 7.992/8.527/9.027 | -0.133 → -0.634 |
| group-fallback W | 94.100 → 98.300 | 79.190 → 104.671 | 1211.872 → 1311.397 | 11.852/16.949/31.520 → 13.504/16.907/33.405 | 330.968 → 455.970 |
| diagnostics-off I | 140.600 → 110.200 | 7.425 → 8.570 | 952.709 → 951.182 | 8.006/8.783/10.242 → 7.998/8.760/9.225 | -0.006 → -0.358 |
| diagnostics-off W | 84.600 → 83.100 | 70.772 → 92.227 | 1289.608 → 1223.153 | 14.022/16.759/33.278 → 10.771/16.755/47.358 | 400.296 → 355.296 |

| 场景/段 | sendEvent n | 同步 p50/p95/max µs | 心跳 n | 心跳 p50/p95/max ms |
| --- | --- | --- | --- | --- |
| aligned-1 I | 120 → 120 | 26.400/43.500/61.100 → 31.500/43.800/60.900 | 166 → 125 | 8.000/8.457/8.986 → 8.009/8.422/8.636 |
| aligned-1 W | 120 → 120 | 20.400/40.700/56.900 → 20.400/37.600/90.100 | 169 → 124 | 8.029/16.826/78.040 → 12.940/16.843/88.603 |
| aligned-2 I | 120 → 120 | 26.300/36.400/67.900 → 28.300/40.300/57.700 | 167 → 123 | 7.999/8.388/8.886 → 8.003/8.493/8.984 |
| aligned-2 W | 120 → 120 | 17.200/36.500/162.200 → 24.300/42.900/57.400 | 166 → 124 | 8.015/16.731/70.887 → 12.042/16.829/104.393 |
| aligned-3 I | 120 → 120 | 29.500/40.100/64.900 → 24.800/39.200/62.600 | 168 → 122 | 7.995/8.471/9.002 → 7.997/8.454/8.572 |
| aligned-3 W | 120 → 120 | 20.200/43.100/70.300 → 21.900/42.700/92.900 | 166 → 123 | 8.036/16.753/102.567 → 12.768/16.869/90.077 |
| pattern I | 120 → 120 | 29.500/43.200/47.700 → 26.700/39.200/61.900 | 166 → 123 | 7.998/8.212/8.641 → 7.999/8.536/8.850 |
| pattern W | 120 → 120 | 20.100/39.300/62.000 → 20.900/38.800/51.000 | 167 → 123 | 8.287/16.838/106.455 → 12.996/16.736/96.930 |
| small I | 120 → 120 | 24.700/38.700/63.000 → 27.300/42.500/49.400 | 126 → 125 | 8.000/8.321/9.010 → 8.001/8.586/8.724 |
| small W | 120 → 120 | 21.900/43.500/72.900 → 26.700/43.200/55.600 | 131 → 130 | 12.493/16.910/124.802 → 12.869/16.799/93.780 |
| nonaligned I | 120 → 120 | 23.300/35.300/54.100 → 27.500/41.100/60.400 | 173 → 170 | 8.002/8.964/9.001 → 7.995/9.037/9.424 |
| nonaligned W | 120 → 120 | 21.200/40.800/54.800 → 23.500/44.100/64.300 | 172 → 174 | 8.035/17.045/70.919 → 8.022/16.748/97.638 |
| group-fallback I | 120 → 120 | 26.400/42.100/92.000 → 27.200/46.400/56.600 | 209 → 206 | 8.005/8.697/9.098 → 7.999/8.436/9.116 |
| group-fallback W | 120 → 120 | 19.300/35.100/44.900 → 24.100/42.000/68.800 | 203 → 210 | 8.008/16.723/79.213 → 8.057/16.717/104.697 |
| diagnostics-off I | 120 → 120 | 31.700/46.500/73.400 → 23.100/33.400/88.600 | 167 → 124 | 8.007/8.543/10.304 → 7.993/8.582/9.231 |
| diagnostics-off W | 120 → 120 | 20.700/38.700/49.400 → 23.000/42.200/468.300 | 168 → 125 | 8.074/16.709/70.787 → 10.772/16.782/92.299 |

| 场景/段 | 资格/映射 ms（后） | 命中/复用/重建（后） | reason（后） | 缓存项（前→后） | retained_bytes（前→后） |
| --- | ---: | --- | --- | --- | --- |
| aligned-1 I | 1.641 | 0/3596/500 | 1 | 4096 → 4096 | 5529792 → 5628096 |
| aligned-1 W | 1.544 | 0/3596/500 | 1 | 4096 → 4096 | 5529792 → 5628096 |
| aligned-2 I | 1.593 | 0/3596/500 | 1 | 4096 → 4096 | 5529792 → 5628096 |
| aligned-2 W | 1.666 | 0/3596/500 | 1 | 4096 → 4096 | 5529792 → 5628096 |
| aligned-3 I | 1.557 | 0/3596/500 | 1 | 4096 → 4096 | 5529792 → 5628096 |
| aligned-3 W | 1.464 | 0/3596/500 | 1 | 4096 → 4096 | 5529792 → 5628096 |
| pattern I | 1.599 | 0/3596/500 | 1 | 4096 → 4096 | 5529792 → 5628096 |
| pattern W | 1.622 | 0/3596/500 | 1 | 4096 → 4096 | 5529792 → 5628096 |
| small I | 0.129 | 0/140/116 | 1 | 256 → 256 | 5259456 → 5265600 |
| small W | 0.623 | 0/140/116 | 1 | 256 → 256 | 5259456 → 5265600 |
| nonaligned I | 1.544 | 0/0/4096 | 6 | 4096 → 4096 | 5698576 → 5796880 |
| nonaligned W | 1.432 | 0/0/4096 | 6 | 4096 → 4096 | 5698576 → 5796880 |
| group-fallback I | 1.462 | 0/0/4096 | 4 | 4096 → 4096 | 5529984 → 5628288 |
| group-fallback W | 1.510 | 0/0/4096 | 4 | 4096 → 4096 | 5529984 → 5628288 |

所有开启诊断的 before 最终任务原坐标 hit=0、平移复用=0；大图4096次重建，小图256次。after aligned/pattern 3596=58×62 项复用、新露出500项重建；小图140=10×14项复用、116项重建。原坐标 hits 与首次消费 translated 条目分开计数，remapped 候选本轮等于实际 translations，不将复用写成普通命中。非对齐 reason=6、S3 reason=4，均0复用4096重建；没有扩大 S3 支持范围。对应正常帧 source_tiles=4096（小图256），本轮目标任务无分页读取/写入。

资格/映射是 setup 内部合计；reason=6/4 时也包含清理原旧缓存，不是纯资格计算。before 的新增 translation_ns=0/reason=0 表示旧实现未分解，不是证明原清理零开销。诊断关闭段没有核心阶段或缓存计数，用“未测”，不能补零。两侧关闭诊断各一进程只作少量扰动观察，不声称诊断零开销。

小图 execute 降低，但 W 的 GUI 收尾 43.191→48.892ms，不能说小图整体更快。非对齐 I 有波动性较快而 W 略慢，S3 同样混合波动；有限对照没有证明它们优化，也没有稳定退化的因果证据。BEGIN 后首 Move 的 W 仍约71–107ms，持续发送约1.19–1.31s、明显偏离名义8ms节奏；原先的常驻开始延迟未修。I 持续约951–953ms，产品没有把被省掉的数百毫秒工作搬到按下或拖动的新阶段；但不能由这些小样本宣称所有阶段绝无扰动。

## 复现命令与证据

在已激活的 MSVC x64/既有 Qt PATH 下，独立构建目录执行（无环境升级）：

```powershell
cargo test --manifest-path core/Cargo.toml -p paint-render --release --locked --lib --test cache --test translation
cargo fmt --manifest-path core/Cargo.toml --package paint-render --package paint-ffi -- --check
cargo clippy --manifest-path core/Cargo.toml -p paint-render --release --locked --lib -- -D warnings
Push-Location tools
python -m unittest test_measure_layer_move test_analyze_move_completion
Pop-Location
cmake --build build --config Release --target qt_layer_move_baseline qt_transform_tests drawverse_runtime

# 每条命令使用不存在的新输出目录；before 在独立工作树用自己的 EXE/Cargo target
python tools/measure_layer_move.py build/bin/Release/qt_layer_move_baseline.exe artifacts/issue-2-main-c2b/new-s2 --scenes 2 --repeats 1 --delta 384 128 --trace --core-trace
python tools/measure_layer_move.py build/bin/Release/qt_layer_move_baseline.exe artifacts/issue-2-main-c2b/new-pattern --scenes 2 --repeats 1 --pattern --delta 384 128 --trace --core-trace
python tools/measure_layer_move.py build/bin/Release/qt_layer_move_baseline.exe artifacts/issue-2-main-c2b/new-nonaligned --scenes 2 --repeats 1 --delta 385 129 --trace --core-trace
# S1/S3 使用 --scenes 1 / 3；关闭诊断不传 --trace --core-trace
python tools/analyze_move_completion.py artifacts/issue-2-main-c2b/new-s2 artifacts/issue-2-main-c2b/new-s2-analysis
```

本轮证据根目录为 `artifacts/issue-2-main-c2b/exact-merged-20261023/`（忽略，不提交）。首轮 `all-runs.json`/`analysis/` 与后轮 `idle-confirmed/analysis/verified-runs.json`、`tables.md` 分开。`run_exact_pairs.py`/`order-*.json` 保留实际命令、固定顺序及每次退出码；每次 invocation 保留 SHA/参数/DLL 摘要，raw/core 保存全部输入、准确任务/帧、终态和原始有界记录。可用本地 `audit_idle.py` 重算核对32终态/16PID/完整像素配对/实际适配器/就绪状态/溢出。未重跑全量检查、旧矩阵或新增 CI。

## 完整 Release 交付与停止

新运行包：`E:/Git/DrawVerse-local-build/artifacts/issue-2-main-c2b/exact-merged-20261023/runtime/`，1348 文件，含 drawverse.exe、所需 Qt DLL/QML/platforms、LICENSE、BUILD.json、SHA256.json 和既有隔离启动脚本；不是一个缺 DLL 的 EXE。保留所有旧 B1/B2/guarded 包。沿用本机已安装的 MSVC runtime，没有验证任意干净机器的可移植安装。主程序 SHA256：`419b58f724242412f5a491bb2f77026d55e3dcec188a1e7cc2720331a31795f8`，产品提交 `1e50b39e...`。

```powershell
powershell -ExecutionPolicy Bypass -File 'E:/Git/DrawVerse-local-build/artifacts/issue-2-main-c2b/exact-merged-20261023/runtime/Start-DrawVerse.ps1'
```

启动脚本在包内新建 exclusive session/profile/temp，不覆盖用户 DrawVerse.ini、工作区或绘画。默认关闭诊断。本轮移除 SDK PATH/QT plugin 环境后使用包内依赖进行隔离 `--preview` 冒烟：退出0，生成157385B预览，检查三条实际笔迹；这是既有预览契约冒烟，不是完整人工功能验收或延迟测量。

已证实：受限资格下真正减少合成/LOD工作、独立精确参考完整像素一致、原正常帧与历史屏障保持。未支持：S3组/链接蒙版、非对齐任意拖动、非均匀固定背景/上层等；均回退。未验收：学生真实作品/笔刷、实体数位板真实手感、肉眼顺滑、物理输入到显示延迟和峰值RSS。不会据此称“图层移动整体不卡了”。本轮最需要补的审查证据是上述未归因 pendingMoved 失败时的准确预览/输入状态，而不是继续扩大快路径；不自动实施新的优化。

## C2-B-R 合并前审查（2026-10-23，追加记录）

**合并建议：UNRESOLVED。** 完整缓存审查未发现需要修改的资格/取消/像素正确性漏洞。本轮在准确未优化 before 捕获了一次目标失败，可解释为测试目标输入已被后续非计划 MouseMove 改变；还修正了测试仅凭两点颜色不能核对目标位移的观察缺口。但历史 after pendingMoved 失败没有这些字段，不能将新原因套用到历史失败，也不能仅以第二轮通过改为 READY。没有发现可确证的缓存新回归，故不标 BLOCKED；仍不建议现在合并。

### 审查范围和版本

起点 `55cc2ea89f46f879ef28b2c14fd1b1776028061e`，fetch origin/upstream 后均与已知锚点一致，upstream/develop 仍为 `9462f84c10b34daf97296416df41d148468d3475`。审查的是 **9462f84c..55cc2ea 的全部产品差异**，含 merge 保留的 guarded 实现，而非最后一项文档提交。原三份完整日志及目标函数全部读取，关键输出另核对；没有重写旧分支、重做合并或操作上游工作区。

| 本轮提交 | 内容 |
| --- | --- |
| `b17d590ba36469be4c8f21beb2981de3d57a246f` | 目标测试的默认关闭现场记录；PaintCoreClient 增加 GUI 线程按需只读 moveObservation，未进入正常调用路径 |
| `b4017d146894ca868b704750637da010028d7031` | 测试增加已接纳目标上下文核对与确定性反例；区分“出现过像素匹配”和“整项断言通过” |
| 对照 `db2108f57f8714208245f777a56e250c83710f40` / `41e749d70fccca64d8507673fa6d620da81e51d0` | 在原独立 before `c30ee4e...` 上分别应用完全相同两项观测/测试增量；没有产品复用快路径 |

本节所在收尾提交只追加文档。缓存/呈现算法、Rust 源码与原 after `1e50b39e...` 相同，原 before 算法仍为准确合并后版本；本轮没有更改 C ABI、锁文件、配置格式、依赖、取消/预热/线程/输入合并或质量。两侧仍各自独立 CMake build/Cargo target，复用 SDK。逐次 invocation 记录准确源码、二进制摘要；每轮两侧的三个变更文件 Git 规范化摘要逐一一致。

### 设计承诺与实现/证据逐项核对

| 审查项 | 实际代码位置与结论 | 已有精确验证 |
| --- | --- | --- |
| 完整合成贡献的资格 | `paint-render/src/translation.rs::certificate` 遍历**所有层**，拒绝 group/mask/clipped/parent!=0/非Normal；唯一显式栅格且 white_extent=0，固定层只允许透明或 offset=0、white_extent=文档尺寸的隐式白；不是由 S2 夹具保证 | patterned_signed_moves... 的白/透明/非均匀背景组合；fixed_nonuniform_upper_layer... 的固定上层反例；groups_linked_and_fixed_masks... |
| 层配置完整性 | `lib.rs::LayerConfiguration` / `render` 编码顺序、ID、blend、offset、seed、parent/group/mask/clip/white/visible/opacity/fill；`Cache::reconfigure` 只接受恰好一个差异，克隆旧项仅替换 offset 后必须完全相等，并匹配证书层 | non_aligned_special_stacks...、limited_budget_and_layer_order...、cold_evicted_edited...；链接状态组合本身已被资格拒绝，不是漏过蒙版贡献 |
| 证书时效与文档身份 | `Cache::mip` 仅在实际构造 Entry 时盖 translation_layer；原坐标命中不重新盖证。当前 certificate 只用于候选，旧项还须有同证；同尺寸/同layer ID并不足以命中 | coarse_lod_and_same_size_document_replacement...；源编辑、冷/受限缓存和后续绘画/undo/redo测试 |
| 不可变源与固定贡献 | `reconfigure` / `mip` 比较所有显式 Source 的长度、顺序、Weak分配身份及有效opacity。Weak保留旧分配身份避免地址ABA；`paint-core/src/pixels/tile.rs::Tile::edit_private` 在修改前 Arc::make_mut，使旧 Weak 失效。固定无显式瓦片的贡献由配置/均匀资格约束 | weak_cache_identity_detects_mid_stroke_writes_without_retaining_pixels；edited_source_phase...、actual_translation_counts... |
| 对齐/相位/边界 | `reconfigure` 相对**当前**configuration，用 i64差值和实际TILE_SIZE取模；`full` 以u64检查旧/新完整瓦片，同维度/level/format、LOD≤TILE_SIZE.ilog2；目标加法/转换checked，缺项/新露出/部分边缘/负向越界拒绝 | 两格式/LOD0、2、6，非64文档、非零原相位、平移视口、(383,127)/(1,0)、正负/返回；独立 CpuRenderer 完整输出严格相等 |
| 原精确回退 | 资格错误清空旧项并更新configuration；原坐标源不符则移除该项，执行原合成/LOD/blit；不改变offset或像素参考 | 非均匀背景/上层、特殊混合、组和两类蒙版、不对齐的实际计数/像素一致 |
| 映射与资源 | `reconfigure` 使用独立新表， checked平移是单射，保留level/format且不原地覆盖；移动 Entry/Mip Arc，不增加源图强引用 | limited_budget...、原坐标hit/translation/miss生命周期断言 |
| 取消和再次使用 | qualification前/后不动旧表；take之后至commit前取消留下空表/旧配置，commit后仅有效新表；Mutex串行持有，重建取消不返回部分帧 | translation_tests.rs 一个函数六个确定性时序，随后连续/返回正常渲染逐元素比对；不是靠随机重跑 |
| 预算/逻辑计账 | `Entry::bytes` 包含新增Entry元数据；`prune`/`reconfigure`重新求和，移除/插入增减；保留预算不提高。临时表按原条目数有界，源仅Weak，Mip无像素复制 | retained_bytes求和/受限预算断言；没有新增峰值RSS实验，也不将逻辑量当进程总内存 |

相关 `lib.rs` 入口：Cache::prune（189）、Cache::mip（206）、Entry盖证/计账（403起）、CachedCpuRenderer/CpuRenderer::render（504/517）、普通render（588，configuration/资格从699起）；`translation.rs` certificate（6）、full（65）、reconfigure（70）。行号对应本节缓存代码未变的版本，函数名是定位主依据。未发现具体新增核心覆盖缺口，因此复用原17函数和六取消时序，不重跑整包或性能矩阵，也不为覆盖率扩大资格。

**调用范围不是“仅在COMMIT后”。** `paint-ffi/src/engine/session.rs::Engine` 为各view持有 CachedCpuRenderer，正常初始帧、设置后的正常帧、撤销恢复、提交/取消后的正常帧都可进入2143/2209的普通renderer；任意这样的调用都必须满足同一资格。`Engine::prepare_layer_move`（967起）调用 `liquify_api::move_preview_assets_cancellable`；当前 sampled preparation 使用 `render_sampled_preview`，不使用该瓦片缓存。`document/liquify_api.rs::encode_preview_assets` 的非sampled支路也会新建 CachedCpuRenderer，但新实例没有旧项，不能产生平移复用。

对本目标**实际按住阶段**：session.rs 2124起在 index<2、preview_layer=0、layer_move_active 时选择 `render_interactive`→`render_sampled_preview`，不调用普通缓存。不能只依据代码的预期，已以本轮各次 BEGIN/UPDATE 的 sequence/generation 对应 task.enqueue(value=1)、view0/1及task身份核对；没有跨时钟相减。其他视口/在途旧正常任务仍可能并行调用普通renderer，不能笼统说“按住期间整个进程不可能运行缓存”。夹具自身为 PassThrough组+链接蒙版，普通支路也不满足平移资格；after 的相应正常帧记录 reason=4、translations=0，初次/未变配置则按cold/unchanged分类。此证据缩小本轮失败的疑点，不能反推缺现场的历史失败一定与优化无关。

### 原断言到底保证什么

夹具是3840×2160白底上满幅红层，加链接黑色蒙版斑（2000.5,1080.5，半径90），再放入PassThrough组，选中子层。原生窗口1024×720、canvas同尺寸、DPR=1、zoom=0.25、document origin=(32,90)。从文档(1200,1080)→(1584,1080)，实际窗口坐标(332,360)→(428,360)，独立期望整数位移=(384,0)。

白点文档(100,1080)→local/image(57,360)，预期来自新露出白底；红点(800,1080)→(232,360)，预期来自移动后源x=416的红色区域，远离蒙版斑。原阈值 white: r/g/b>230；red: r>180且g/b<70，原整体AND、坐标映射及阈值全部保留。

`layerMoveActive()` 在本地 BEGIN 排队后立即为true，不保证BEGIN/UPDATE已接纳，不保证对应帧到GUI，更不保证异步grab已使用那一帧。原QTRY像素条件会继续重试，但只看两点不能证明目标384已接纳；且颜色在不相关位移/较旧抓图上也可能成立。这是观测契约缺口，不是已证明的产品停更。

### 新现场：一次有证据的新失败，不能覆盖旧失败

首轮严格执行 before组合→after组合→before单独→after单独各一次。before组合目标失败，其余三次目标通过；组合中其他12功能行仍通过。现场记录显示：

- BEGIN sequence=13，未准备；随后测试计划的Move应为(428,360)，但同窗口/画布紧接着收到未在该测试轨迹中请求的Move，例如(228,293)。按原逆变换：((228−332)/0.25,(293−360)/0.25)=**(−416,−268)**；after-original-mouse-move checkpoint已是该值。
- `move.submit` sequence=14 的UPDATE值确为(−416,−268)，核心同sequence提交/接纳/应用，有对应interactive任务。之后仍有连续变化的输入、位移和新revision/frame，不是证据显示一直卡在同一预览。source=0 / device=1 本身不能区分 Qt合成和OS事件，因此只称“非计划输入”，未判定物理设备、源程序或用户操作。
- 同一张首次失败图为1024×720、QImage format=18，两个采样均在界内、RGBA都是(255,0,0,255)：white=false、red=true，空图=false。白点不满足本来就要求的384目标。两点不是分别来自不同截图。
- 记录的抓图#49曾出现 white=(255,255,255,255)、red=(255,0,0,255)，旧像素AND为true；同时GUI请求delta=(1880,56)、dirty=true，已接纳owner offset=(1848,72)。**不能由此说这张图恰按1880渲染**：其精确GPU帧身份未测，但足以反驳“两点匹配即证明当前目标384正确”。最后#51为white/white，红点也失败。
- 无产品错误、无空图或grab超时；本次观察到的目标阶段没有 ApplicationDeactivate/WindowDeactivate/Ungrab 取消，不能标D。结束时断言失败、move仍active，未执行该测试后面的正常Escape/COMMIT部分，不能把该目标功能行记通过。

分类：本轮这一次是 **B：输入夹具/观察条件不满足（非计划输入）**，并且发生在未优化基线；没有证明基线产品缺陷C，也不能因此排除历史A。历史after失败仍为 **E：现场字段不足、继续未归因**。C1 moved3失败、C2-A探索抓图崩溃、原八次ApplicationDeactivate各保持原边界，未合并归因。

### 局部测试修正与可控反例

没有修改产品缓存或预览策略。`PendingMoveEvidence::acceptedContext` 在**原颜色AND之外增加更强核对**：预期generation、owner、undoDepth和逆变换得到的实际目标delta；active且非starting/closing/dirty；UPDATE已完成（completedSequence≥sequence）；owner元数据offset吻合；GUI view0帧generation/revision与当前状态一致。没有增加超时/sleep、跳过按住阶段或屏蔽失焦/真实鼠标。输入被改变时仍会失败，不将它吞掉或改skip。

确定性回归 `pendingMoveObservationRejectsUnrelatedState`（1功能函数）以记录的反例值检查旧颜色匹配不能替代目标上下文：合法状态接受；wrong dx/dy、starting/closing/dirty/inactive、未完成sequence、换generation、改变history、产品错误、旧frame revision、错误owner offset共12项拒绝。原两个RGBA标准仍在同一张真实grab上执行；初始红/红或空图仍不能通过，GUI帧停在旧revision也不能通过。新条件只是GUI端的必要关联，**不是grab的精确render-frame回执或全GPU像素证明**；稀疏颜色点与渲染侧帧归属的局限保留，不能拿最终getter称肉眼无停更。

新增记录 `ui/tests/PendingMoveEvidence.h` 仅在 `DRAWVERSE_PENDING_MOVE_EVIDENCE` 指向已有新空输出目录时启用；最多256 captures/checkpoints/input行，各自溢出计数，原InteractionTrace仍有自身容量/溢出。仅观察本应用目标窗口/画布的鼠标与生命周期事件，不记录文本或系统键盘。截图来自原capture调用，没有额外截图；最多持有first/last failure及success三张图，结束后才写JSON/PNG。request、ready信号、wait返回、断言状态分别记录，均为GUI线程状态，明确不将ready时的getter认作图像帧身份。

每次grab串行，QSharedPointer在原5000ms等待内保持结果；ready回调有QObject上下文并由scope guard在返回前断开，所有捕获栈引用都仍有效；不留重叠grab或退出后悬空回调。QPointer检测对象存活，observer先于window/client析构，失败返回也执行RAII导出。未跨线程读取GUI对象。首轮字段 `pending_succeeded` 只是“至少一次像素AND匹配”，**不表示Qt断言通过**（before组合就是反例）；第二提交改为any_pixel_match与显式pending_assertion_passed分开，统计始终以Qt结果/通过检查点为准，没有错误计作通过。

### 有限对照结果与保留证据

第一轮出现了上述可检验线索，故只追加第二轮，验证更强的目标状态核对；仍各组一次，不再进行第三轮。每轮顺序固定为before组合、after组合、before单独、after单独；表按目录名列出。全部日志实际创建D3D11并使用AMD 1002:1638，DPR=1/DPI=96；应用请求auto、报告实际启用wintab，本轮输入为Qt合成鼠标，不能将后端选择当实体驱动验收。没有切GPU、请求用户暂停工作或额外抢焦点。8进程退出码为第一轮before组合1、其余0；无崩溃、失活/失抓意外取消、进程超时或诊断溢出，成功函数内原定Escape取消保持。进程时长、QTRY/抓图等待不是处理/显示延迟。

| 轮次/版本/方式 | 测试 HEAD | EXE SHA256 | 功能 pass/fail | 抓图次数 | 目标断言 |
| --- | --- | --- | ---: | ---: | --- |
| round-1-after-combo | `b17d590ba36469be4c8f21beb2981de3d57a246f` | `71210ae02f993dcc834a7378b43dd8d9f24cef00daebfa249e2a40f9eb22c838` | 13/0 | 7 | 通过 |
| round-1-after-single | `b17d590ba36469be4c8f21beb2981de3d57a246f` | `71210ae02f993dcc834a7378b43dd8d9f24cef00daebfa249e2a40f9eb22c838` | 1/0 | 6 | 通过 |
| round-1-before-combo | `db2108f57f8714208245f777a56e250c83710f40` | `4e64d786c82ea025c197dcd86d4ff6b4d40a730dd1dbec57b8e051d8e48fccd4` | 12/1 | 47 | 失败，保留现场 |
| round-1-before-single | `db2108f57f8714208245f777a56e250c83710f40` | `4e64d786c82ea025c197dcd86d4ff6b4d40a730dd1dbec57b8e051d8e48fccd4` | 1/0 | 6 | 通过 |
| round-2-after-combo | `b4017d146894ca868b704750637da010028d7031` | `cc439e1cf5aba66d4fc95aacd3cb1fefcd34254c13bb3a01af1db3f73e85c068` | 13/0 | 5 | 通过 |
| round-2-after-single | `b4017d146894ca868b704750637da010028d7031` | `cc439e1cf5aba66d4fc95aacd3cb1fefcd34254c13bb3a01af1db3f73e85c068` | 1/0 | 6 | 通过 |
| round-2-before-combo | `41e749d70fccca64d8507673fa6d620da81e51d0` | `8762681d62ce3f431cc8d8d38a07d30cb4685c5bb86f0a6c4eb0df497c2d0124` | 13/0 | 6 | 通过 |
| round-2-before-single | `41e749d70fccca64d8507673fa6d620da81e51d0` | `8762681d62ce3f431cc8d8d38a07d30cb4685c5bb86f0a6c4eb0df497c2d0124` | 1/0 | 6 | 通过 |

组合为3功能函数/13功能数据行；单独为1功能行。Qt每进程init/cleanup各一项已扣除。首轮27功能行执行通过/1失败；第二轮28通过/0失败；这是重复执行数，不是55个独立新用例。无窗口确定性回归另1函数通过，12反例不算12功能函数。未修改核心，复用原17函数/六取消时序、原FFI专项与原测量表，不重跑核心/性能全矩阵。只增量构建两侧qt_transform_tests和C头检查，均通过；没有环境/依赖升级，也没有新性能结论。

证据独立放 `artifacts/issue-2-main-c2b-review/`，忽略且不提交：各round目录含invocation（准确HEAD/EXE/源码摘要）、test.txt/qt.log、core.json、pending-evidence.json及三种原图；round-1/2-order.json保留实际PID/退出码。`verified-review.json`核对相同观测源、实际后端、每段任务路径和功能结果；`review-product-diff.patch`保存完整审查差异，`evidence-sha256.json`记录现场文件摘要。原exact-merged及全部旧失败未覆盖。

启用方式（只对这一个专项，输出目录需新建；不设置该变量即关闭现场记录）：

```powershell
# 已有 MSVC/Qt 环境，使用各工作树自己的build和Cargo target
cmake --build build --config Release --target qt_transform_tests -j 2
$reviewOut = Join-Path $PWD 'artifacts/issue-2-main-c2b-review/new-run'
New-Item -ItemType Directory -Path $reviewOut -ErrorAction Stop
$env:DRAWVERSE_PENDING_MOVE_EVIDENCE=$reviewOut
$env:DRAWVERSE_MOVE_COMPLETION_TRACE=Join-Path $reviewOut 'core.json'
$env:QT_QPA_PLATFORM='windows'; $env:QT_QUICK_BACKEND='rhi'; $env:QSG_RHI_BACKEND='d3d11'
& ./build/bin/Release/qt_transform_tests.exe prewarmed4kResidentMoveReusesFullQualityTexturesAndRestoresFullFrame -o "$reviewOut/test.txt,txt"
Remove-Item Env:DRAWVERSE_PENDING_MOVE_EVIDENCE,Env:DRAWVERSE_MOVE_COMPLETION_TRACE
# 无窗口确定性状态关联回归
& ./build/bin/Release/qt_transform_tests.exe pendingMoveObservationRejectsUnrelatedState
```

未更新运行包：只有测试、按需只读观测和文档改变，原完整Release包/产品SHA `1e50b39e...` 及二进制摘要继续保留。未创建PR、合并、关闭Issue或开展新优化/实体/RSS实验。合并仍缺的具体证据是：历史失败是否在**无非计划输入、目标384已接纳、无合法取消**时仍出现，以及该失败grab实际采用的渲染帧/路径；若再次发生，必须用现场记录分离这些条件，不以更多通过样本代替。当前保留UNRESOLVED，不要求用户现在配合设备或暂停工作。


## 当前预览契约验证与合并判定（2026-10-23，独立追加）

**当前建议：READY FOR REVIEW。** 当前 after 在有效计划输入、稳定目标状态及同一次 grab 的 render 采用身份关联下，通过未准备移动的按住预览和原三函数回归。缓存审查沿用上一节，没有新增阻塞或本轮有效场景的产品失败。此建议供人工审查决定，不代表已合并、整个 Issue 或实体体验已验收；本轮未创建 PR。

### 三种状态分别记录

| 状态 | 本轮结论及证据边界 |
| --- | --- |
| 历史 after `native-regressions/test.txt` 的 pendingMoved 失败 | **unresolved**：缺输入/抓图现场，未还原，不推定为本轮发现的任一原因，也不改写成已修复 |
| 上节 before 的非计划输入失败 | 继续按已有记录解释：计划384被其他 MouseMove改成例如(-416,-268)，场景前提不满足；不能据此认定产品像素错误，更不能代替历史after根因 |
| 当前受控验收 | 修正后 before单独、after单独、after三函数组合各一次通过；目标384实际已接纳并稳定显示，具体身份见下表 |

上一节 UNRESOLVED 是上一轮任务的历史建议。本轮按新的验收要求，将历史根因与当前合并建议分开；没有要求证明历史从未存在产品错误，也没有依靠缺失历史现场跳过当前验收。C1历史取消/像素失败、C2-A探索抓图崩溃及最初失活取消仍全部保留原边界。

### 版本、构建和输入协议

审查起点为 `7fb875c3a7c580b9eff20b0219bf1dbe35c63e59`，工作区干净；fetch origin/upstream 后没有相关新提交，upstream/develop仍为 `9462f84c10b34daf97296416df41d148468d3475`。没有重新审查全部缓存、扩大资格或改动Rust/cache、绘画、预热、取消、线程、采样/质量策略。

| 提交 | 内容 |
| --- | --- |
| `a263536acc44a30e2936492bb47cbabb462996df` | 计划窗口事件、稳定grab协议、默认关闭的单槽render采用观测、确定性反例与设计追加 |
| `447c4020b150a78601e2090224e227e9c450a96f` | 仅修正测试把viewChanged通知误认实际视图变化的条件，并补对照断言；当前已验证代码 |
| before `010e3bb8defa82b74e8895766a29cf70522b6a25` → `42c1d7ce7cf564ce0ce028dae956981f959da4f4` | 在原独立before工作树准确应用相同两项测试/观测增量，不含缓存优化；未重写原历史 |

当前before仍基于原 `c30ee4e...` 产品，after缓存算法仍为原 `1e50b39e...`；各自独立CMake build/Cargo target。修正后的五个公共C++/测试文件的Git规范化SHA256全部相同，各次invocation保留准确HEAD、EXE及源码摘要。SDK/依赖未升级，C头检查和两侧Release增量构建通过；没有改ABI/锁文件/配置格式。

已实际阅读本机 Qt 6.8.3 `include/QtTest/qtestmouse.h`：QWindow mouseEvent经qt_handleMouseEvent后会调用`qApp->processEvents()`（126行）。因此“在QTest助手调用期间收到”不是来源证明。本测试改用已有基准采用的`QCoreApplication::sendEvent(QWindow*, QMouseEvent*)`：每个独立事件对象有计划编号，保存目标窗口、顺序、坐标、按键、modifier及返回accept状态；应用级过滤器只有在目标窗口入口、指针与当前登记的那个事件完全一致时才标为planned。Canvas的后续交付只记delivery，不按source/device或时间猜来源，不重复算作独立计划输入。

目标夹具仍为3840×2160、满幅红色子层+链接蒙版+PassThrough组，窗口/Canvas 1024×720；DPR=1、DPI=96、zoom=0.25、document origin=(32,90)。计划Press=(332,360)、Move=(428,360)，独立逆映射期望=(384,0)，未吸附/改变产品坐标。后续准备后Press/Move/Move/Release同样显式登记为3/4/5/6号事件。事件经真实QWindow和Canvas正常路由；不移动OS光标，不向其他应用注入事件，没有要求用户停工或额外抢焦点。

观察期间其他窗口入口鼠标/笔/滚轮/键事件和生命周期继续正常传播。目标阶段收到非计划输入或实际文档/视图/窗口身份变化即锁存前提失效，不补发目标位置覆盖干扰；不记录文本内容。输入代数、GUI内容代数及render身份代数独立记录，变化后恢复也不会回退代数。

### 同一次grab的证明规则

`PendingMoveEvidence::associated` 在原12项状态条件之外要求：

1. BEGIN实际sequence与之后完成的目标UPDATE sequence独立记录，不推算“+1”；UPDATE、owner=2、generation=3、delta=(384,0)、undoDepth=5和view0帧revision均吻合。
2. 请求之前目标Canvas节点已经采用对应GUI imageKey（或resident source/revision）。`CanvasPresentationObservation`只在`CanvasItem::updatePaintNode`的Qt同步阶段发布**成功采用**身份，失败/删除路径发布invalid。GUI线程在该同步阶段被Qt阻塞；观测使用与现有节点设置相同的image/region/documentRect快照，不在异步render回调查询client的sequence/frame元数据。单槽互斥保护，没有GPU对象跨线程，没有额外刷新/定时器，普通产品默认无observer、无逐帧锁和分配。
3. 对指定Canvas串行grab，独立编号绑定该结果对象的ready回调；保留request/ready/complete/assertion四份状态。四处的操作、GUI内容与输入代数、目标Canvas/窗口、view/frame/request/revision/imageKey、render采用身份/代数、DPR及视图必须一致。render曾换图后恢复也会因代数变化拒绝。
4. 返回图像cacheKey必须是该次grab实际取得的结果；采样坐标冻结在请求上下文。原white/red标准、两个点及整体AND不变，两点来自同一张图。请求/ready回调和持有结果均在原5000ms完成边界内，作用域退出断开回调；超时后不再启动下一次grab，没有重叠或销毁后的栈回调。

这是**已采用目标节点在稳定区间内、同Canvas的item grab关联**，不是将最新getter重命名为回执，也没有用任意frameSwapped。它不测硬件扫描输出、物理输入到显示延迟或肉眼停更。实际成功均走normal/interactive图像节点，resident采用字段的条件保留但本次目标不以未发生的resident路径冒充验证。

### 确定性辨别力与一次协议误判

保留`pendingMoveObservationRejectsUnrelatedState`的1功能函数/12个状态反例；新增`pendingMoveGrabAssociationRejectsStaleAndInterruptedEvidence`为1功能函数，含13个关联拒绝断言：旧render imageKey、旧revision、请求端其他sequence及整次记录属于另一sequence、输入/内容/计划代数变化、取消标志、render ABA、换generation/zoom、其他Canvas及错误ready编号。另验证实际单槽发布器的ABA代数、失败invalid，以及新revision/imageKey/sequence已正确接管时能通过。JSON条件测试不是原生渲染替代品，下列原生运行补足这一点。最终两个功能函数通过，init/cleanup不计数。

第一次before单独运行（`before-single/`，尚用a263536协议）失败并保留：只有已登记的Press/Move，没有非计划窗口输入，目标最终是384且两点颜色匹配，但记录器已把同步`viewChanged`通知锁存为scenario_invalid。源码 `CanvasItem::setClient` 的stateChanged连接会在refresh后发viewChanged（约142行），这不代表zoom/origin改变；现场在Press前后均为0.25/(32,90)。这是本轮**测试协议的假拒绝**，不是桌面干扰，也不是产品像素错误。

447c402将判定改为比较实际视图身份（含generation、Canvas/client/window、zoom/origin/size/DPR），不是忽略信号或放宽产品像素条件；增加“revision通知但视图相同”与“实际zoom变化”的确定性对照。这个明确新线索只追加一次必要before验证，after两次此前未运行。首个失败不删除、不当通过：保留256条抓图断言记录及49条溢出计数，trace无溢出；修正后以下三次均无溢出。没有第三/第四轮盲目重跑，没有加sleep、扩大超时、改变采样点/颜色/取消或跳过按住预览。

### 原生结果（当前协议）

所有运行均为Windows原生可见Release。Qt日志实际创建D3D11 QRhi并选用AMD Radeon(TM) Graphics（1002:1638）；未切换独显或软件后端。Qt输入后端请求auto、实际报告wintab；本轮为Qt合成鼠标，不等于实体手绘板驱动验证。

| 运行/实际HEAD | 功能pass/fail | 按住记录/成功grab编号 | EXE SHA256 |
| --- | ---: | --- | --- |
| 首次before `010e3bb8...`，协议误判保留 | 0/1 | 256保留/无成功，49溢出 | `cddda1a80cd47bde8618fe5da9acef5a7822c2376a1fe5c131191704dea8f5e2` |
| 必要before对照 `42c1d7ce...`，目标单独 | 1/0 | 6 / #8 | `7a21811bedf84e636ee6bcf03e3d5c7db1b5112c7fbac3f91ea3c1e47d5726cc` |
| after `447c4020...`，目标单独 | 1/0 | 7 / #9 | `ed8a2b351f75a4c90c7af988f70551a688e606474109c132e132a24bd4786b03` |
| after `447c4020...`，原三函数组合 | 13/0 | 7 / #9 | 同上 |

三个成功grab均：BEGIN=13、目标UPDATE=14、generation=3、revision=7、owner2 offset=(384,0)、history=5；GUI frame为view0/request4/frame5/revision7。抓图前节点已采用相同imageKey、region=(0,0,3840,2160)，包含两个采样点；request→ready→complete→assertion期间各身份与代数全部一致。白点文档(100,1080)→image(57,360)，RGBA=(255,255,255,255)；红点(800,1080)→(232,360)，RGBA=(255,0,0,255)。两点同图、原阈值通过。

| 成功运行 | GUI/节点imageKey | GUI内容代数 / 输入代数 / render代数 | grab结果imageKey |
| --- | --- | --- | --- |
| before单独 | 73014444033 | 6 / 2 / 3 | 90194313218 |
| after单独 | 81604378625 | 7 / 2 / 3 | 98784247810 |
| after组合 | 90194313217 | 7 / 2 / 3 | 98784247810 |

图像键是各进程本地身份，不做跨进程相等推论；grab结果键与被采用源键不同是正常的独立结果对象。每次完整字段保存在pending-evidence.json，`verify_current.py`逐一断言四阶段一致、同一grab/Canvas及两侧公共源码摘要相同；摘要保存在`verified-current.json`，不仅摘录完成时getter。

修正后的三次进程退出0、无skip、无意外取消/错误/超时/崩溃或诊断溢出；原定Escape回执及history不变仍通过。后续准备纹理复用、松手单历史/完整帧接管/撤销，和组合的复杂蒙版路径、撤销/重做均由原断言实际执行。功能成功数为1+1+13的重复执行，不是15个新增独立函数。没有新的有效场景产品失败；首次测试协议错误与旧历史失败分别保留。

### 证据、命令与剩余边界

新证据位于忽略目录 `artifacts/issue-2-main-c2b-current-acceptance/`，未提交：

- `before-single/`：首次协议假拒绝，含原图、输入、状态、trace、test.txt、invocation与溢出记录。
- `view-identity-before-single/`、`view-identity-after-single/`、`view-identity-after-combo/`：当前协议三次运行，含同样完整字段及成功图。
- `native-order.json` / `view-identity-native-order.json`保留四次实际进程/退出码；`verified-current.json`、`evidence-sha256.json`、公共patch、构建及确定性日志独立保留。

复现仍用两侧各自build（已有MSVC/Qt环境），先运行两个确定性函数，再用新输出目录启用该测试的本地批量导出：

```powershell
cmake --build build --config Release --target qt_transform_tests -j 2
./build/bin/Release/qt_transform_tests.exe pendingMoveObservationRejectsUnrelatedState pendingMoveGrabAssociationRejectsStaleAndInterruptedEvidence
# 输出目录必须新建且不含pending-evidence.json；Qt/配置隔离同本轮invocation
$env:DRAWVERSE_PENDING_MOVE_EVIDENCE='<新输出目录>'
$env:DRAWVERSE_MOVE_COMPLETION_TRACE='<新输出目录>/core.json'
$env:QT_QPA_PLATFORM='windows'; $env:QT_QUICK_BACKEND='rhi'; $env:QSG_RHI_BACKEND='d3d11'
./build/bin/Release/qt_transform_tests.exe prewarmed4kResidentMoveReusesFullQualityTexturesAndRestoresFullFrame
# 组合在同一命令后追加：complex4kLayerMoveRestoresFullFramesAndHistory compoundLayerMovesKeepResidentSources
Remove-Item Env:DRAWVERSE_PENDING_MOVE_EVIDENCE,Env:DRAWVERSE_MOVE_COMPLETION_TRACE
```

准确隔离路径、参数、实际环境与二进制摘要以各invocation和qt.log为准。没有设置记录变量时不写现场文件；Canvas单槽观察只由这项测试显式启用，产品普通启动默认关闭。没有重新制作运行包：缓存/呈现策略未变，原包保留；原17项核心/确定性取消、性能数据仍按原测量SHA引用，没有重新跑矩阵或提出新的收益数字。

**当前合并建议：READY FOR REVIEW；历史根因状态：unresolved。** 当前测试会拒绝旧节点/旧revision、错误操作及被改变又恢复的输入；已取得真实窗口中目标按住像素及后续专项通过证据。剩余风险是缺现场的历史after失败和原先未归因异常仍不能排除；它们不被改写成已修复。真实作品、实体手绘板手感、肉眼稳定性和物理显示延迟仍未验证。此处停止，不自动PR/合并、不关闭Issue，不进入任何新优化。
