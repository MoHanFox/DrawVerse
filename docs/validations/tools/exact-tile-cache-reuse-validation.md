# 主线 C2-B — Exact Tile Cache Reuse 验收

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
