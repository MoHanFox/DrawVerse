# 主线 C2-B — Guarded Tile Translation Reuse 验收

## 版本、依赖与构建

2026-10-23，本轮分支 `fix/issue-2-move-tile-cache-reuse` 从准确的 C2-A HEAD `eb3e2c2caf7c3c489cd03d7b3a269a6bdd19b757` 建立。依赖仍未合并：C1 `22a4183c8317ad0b9e636c56e4ec1fbd57cec2df` → C2-A → C2-B；没有从缺少测量工具的 upstream/develop 开始，没有重写前两分支历史。

| 用途 | 提交 |
| --- | --- |
| 共用工具、基线可通过的完整像素测试、设计 | `0eb73bcf7789f3c235e138ae02c083855ed5c1e6` |
| 局部产品优化，仅 paint-render 缓存实现 | `9338b900234c46b16cece3d2851077cc21426b12` |
| 资格拒绝断言与确定性取消测试；实际测量 HEAD | `173799cf5556ef8f9f422748ec0f9080acc64b49` |

本验收收尾只增加文档。修复前工作树 `E:/Git/DrawVerse-c2b-before` 固定在 0eb73bc，即准确 C2-A 产品及诊断代码加两侧共用测试/观测字段；它没有产品快路径。修复后使用既有英文入口 `E:/Git/DrawVerse-local-build`。两者各有独立 CMake build、Cargo target 和二进制，只有 SDK/依赖下载复用；保留对照工作树、历史运行包与所有数据。

共用 runner SHA256 `858c2320b6365007b2659550f47f0268db8a80297bfcf62256522b73e6d7177a`，分析器 `71661f4145e8c18acb1ac8eb6287f90b38041166d4534cb74b88ddafd91eae40`。两侧 runner/分析器字节相同，宿主与 FFI 观测代码的 Git 规范化文本摘要相同，见 `shared-tool-provenance.json`。每个 invocation 保存准确 HEAD、逐文件摘要及空工作区差异摘要；其中沿用的 `product_baseline=b948dcc...` 是 C1 历史产品起点，**本轮前后版本应以上述准确 HEAD 和源码摘要为准**，不能误称前版本没有 C2-A 观测。

| qt_layer_move_baseline.exe | SHA256 |
| --- | --- |
| 修复前 | `a4d2be8764bc117ea5891eb4dc860f7d5b21019ce37774b443e2dd8d7bb24d79` |
| 修复后 | `ea567ba538b7db99e755ec5039f23ad19c31ca25616331133941223ed9bdfe27` |

Windows 10，Release，MSVC 19.43 / Windows SDK 22621 / Rust 1.99.1 / Qt 6.8.3 / CMake 4.4.4 / Ninja 1.12.1 / Python 3.10.8。未升级工具链、依赖或锁文件。每次原生可见窗口日志均实际创建 D3D11 QRhi，并在 AMD Radeon(TM) Graphics（1002:1638）后记录 `using this adapter`，不是只检查环境变量，也没有切换 NVIDIA 独显。窗口 1024×720、视口 656×656、DPR=1、logical DPI=96、屏幕 LQ156M1JW26、缩放 0.16015625。

## 资格、等价理由与反例

[设计](../../tools/guarded-tile-translation-reuse-design.md)中的第一版证书只接受：所有层为平面根层、Normal、没有组/蒙版/剪贴；恰好一个有显式瓦片的栅格层，且该层没有隐式白范围。其余层必须无显式瓦片，为透明默认层或 offset=0、white_extent 等于文档尺寸的隐式白层。固定贡献可以位于移动层上下，但配置与顺序必须不变。因此半透明内容下的背景贡献逐像素相同；有坐标细节的静态背景直接拒绝，不会把旧背景一起搬走。

`cache.configuration` 完整保留。只有一层配置不同，且唯一差异是该证书栅格层的 offset；ID、顺序、可见性、opacity、fill、blend、seed、parent/group/mask/clip/white 均保持。位移相对**当前缓存配置**计算，不相对初始文档或固定测试目标；两轴必须为 64 的整数倍。每项还必须带有在其实际构造时写入的旧证书，不能用当前资格给旧项补证。

旧坐标 c、平移瓦片数 d、旧/新层 offset o/o' 满足 `(c+d)*64-o' = c*64-o`。逐项核对新位置所有显式源引用的数量、顺序、有效 opacity 和不可变 Tile 的 Weak 身份，不只比较移动层 ID。现有 Arc COW 编辑会使旧 Weak 身份失效；这保留源编辑后的精确重建。均匀固定层没有漏比的显式源。

只转移实际存在的缓存项，不强持有源像素。旧/新坐标都须是同尺寸文档内的完整 64² 瓦片；i64 检查加减、u32 检查转换，负向越界、部分边缘、新露出及无旧 Mip 的画布外来源均重建。同 LOD、同格式且 LOD≤6，64 平移保持 2^LOD 采样相位和聚合范围；视口采样使用原 blit。LOD>6、资格不明、不同尺寸、淘汰或冷缓存保留原路径。源已有非对齐 offset 后再移动 64 也按相对差值核对，而非要求绝对 offset 对齐。

映射使用独立新表，检查过的平移为单射，不原地覆盖旧键。取消前/中/后规则：资格阶段尚未更改旧表；移交旧表后至提交前取消，留下空表及旧 configuration；提交后取消，只留下已证明有效的新配置项。重建沿用原取消检查，不返回部分帧。下一次正常渲染仍全像素正确。

反例/不支持：两层实际栅格、非均匀固定背景、组（包括 S3 的 PassThrough）、链接或固定蒙版、特殊混合、非整瓦片位移、同时改 opacity/结构等；不为覆盖这些条件扩大架构。无新的图层缓存、GPU 路径、ABI、格式、质量策略、取整、输入/线程调度或历史语义变化。

## 正确性与生命周期专项

| 验证 | 实际结果 |
| --- | --- |
| paint-render lib + cache + translation，Release | 16 个测试函数通过：lib 2、既有 cache 7、新 translation 7；无失败/忽略 |
| 确定性取消 | 上述一个 lib 函数内覆盖资格前/后、第三个映射项、提交前/后、第三个重建项共 6 个受控时序；全部返回 Cancelled，随后多次正常移动全像素一致；不将循环六项计作六个函数 |
| paint-ffi async_session 定向 | 4 个功能函数通过；过滤项不计通过 |
| 原生可见 D3D11 既有 transform 专项 | 3 个功能函数、13 个功能数据行通过；Qt 总数 15 包含 init/cleanup 两项，不计功能用例 |
| 既有离线测量分析 | 7 个轻量测试通过 |
| fmt / paint-render Release lib 严格 Clippy / 两侧增量构建 | 通过；生成 C 头检查通过 |
| 本轮原生前后比较 | 14 个独立进程、28 段 committed，每段 120 个实际 Move；0 取消/错误/超时/异常退出，记录溢出 0 |

核心参考仍为未改动的 CpuRenderer 精确渲染，逐元素比较**完整输出** LinearRgba32F 和 SrgbRgba8Premultiplied，不只采样颜色点或计数。333×269 确定性图案包含不同瓦片颜色/细节、透明和半透明边缘、真实画布外笔迹，代表 LOD 为 0/2/6。覆盖白/透明背景、非均匀静态背景反例、正负/零/连续/返回移动、(383,127)/(1,0)、非 64 文档边缘、裁切和视口平移、源编辑、冷缓存/0/512/25000 字节受限预算、增删/顺序/可见性/opacity/fill/Multiply、组和链接/未链接蒙版。既有 cache 用例另覆盖高 LOD、格式切换、同尺寸文档替换、缺页与 Weak COW 身份。

FFI 定向复用：`layer_move_resident_packets_absolute_updates_and_final_barrier`、`resident_move_skips_coarse_cpu_frames_and_restores_full_resolution`、`viewport_replacement_generation_and_disabled_slots_reject_old_frames`、`frame_copy_capacity_stride_padding_format_and_stale_ids_are_atomic`。验证最终提交、单历史/关闭屏障、正常帧恢复、旧 generation/request/frame 拒绝及原子复制，未修改这些路径。

Qt 定向复用：`prewarmed4kResidentMoveReusesFullQualityTexturesAndRestoresFullFrame`、`complex4kLayerMoveRestoresFullFramesAndHistory`、`compoundLayerMovesKeepResidentSources`。本轮这一组通过；**不据此改写 C1 历史像素失败或声称已修复**。

所有原生进程还验证最终位移、一次拖动一条历史、撤销/重做及取消后的属性/像素。新增完整正常 CPU 视口 SHA256 在完成时间戳之后计算，七对进程的 I/W 两段均前后一致。它是接纳正常帧数据的完整摘要，不是 GPU 截图或物理屏幕等价证明；复杂纹理/透明图案的逐元素等价主要由上述核心测试建立。

## 场景、输入与统计边界

沿用 C2-A 原夹具：S2 为 4096² 满幅红色 Normal 栅格层+隐式白底，非空范围 [0,0,4096,4096]；S3 是 PassThrough 组内该层+有真实遮挡的链接蒙版。源瓦片 4096，非空白工作量；S3 第一版明确不支持。对齐目标 (384,128)，非对齐 (383,127)，没有改产品坐标取整。

I=进入移动工具立即拖动，14 进程均 assets_ready=false、source=0、upload batches=0，实际走原 CPU 交互预览；W=随后资源准备并上传完成，均 source=4、upload batch=1，同段保持同源且没有额外源上传。W 是准备就绪，不是磁盘冷缓存。BEGIN 延迟仍保留在节奏数据，本轮未优化。

Qt 合成 Press→120 Move→Release，经真实 QQuickWindow/CanvasItem。GUI QTimer 目标 8ms，**是事件循环驱动轨迹，不是外部恒定 125Hz 输入**；W 明显随事件循环延后。未向其他应用注入事件。完成判据、终态和精确 view/generation/request/revision/frame 关联沿用 C2-A；不选择“下一个任意 frameSwapped”。Rust Instant、C++ steady_clock 各自记录持续时间；跨语言沿用同步调用包含区间的 offset 校准，不直接相减未经对时的时间戳。心跳只是事件循环代理，不是输入到显示延迟。

每段最终任务 N=1，表中列实际值，不给三个首次样本强行算 p95。执行包含 setup；合成/LOD/blit 为执行内嵌套区间（包括源读、合成、LOD 聚合和写出），未再次拆成互斥子成本，不能相加。`translation_ns` 是 setup 内的资格/重映射合计；不合格时包含原清理旧缓存的工作。修复前新字段为 0、reason=0 表示旧路径未分解，**不是证明资格判断或旧清理零成本**。诊断默认关闭、有界，计时区间不抓图、不逐条写盘；关闭诊断时没有核心阶段数据，不填零代替未测。

## 每次前后结果

单位 ms，箭头始终修复前→修复后。aligned-1/2/3 分别前→后交替运行；非对齐首对也是前→后，reverse 额外一对按后→前执行，保留首对而非替换。S3 和诊断关闭各一个有限对照。全部最终位移/历史/像素通过。

| 组/段 | Release→GUI 前→后 | 任务执行 前→后 | 合成/LOD/blit 前→后（嵌套） |
| --- | ---: | ---: | ---: |
| aligned-1 I | 354.111 → 16.422 | 342.302 → 8.168 | 340.260 → 5.491 |
| aligned-1 W | 383.542 → 47.485 | 340.826 → 8.928 | 338.289 → 6.585 |
| aligned-2 I | 356.980 → 17.396 | 345.824 → 8.416 | 343.347 → 5.691 |
| aligned-2 W | 358.622 → 20.919 | 346.321 → 7.458 | 344.297 → 5.170 |
| aligned-3 I | 372.016 → 20.408 | 353.934 → 7.780 | 351.938 → 5.238 |
| aligned-3 W | 359.307 → 28.509 | 344.981 → 8.325 | 342.712 → 6.037 |
| nonaligned I | 436.183 → 464.457 | 407.078 → 442.619 | 404.387 → 440.082 |
| nonaligned W | 462.426 → 463.777 | 429.849 → 439.007 | 427.384 → 436.194 |
| nonaligned-reverse I | 466.403 → 477.539 | 448.382 → 439.945 | 445.689 → 437.511 |
| nonaligned-reverse W | 456.778 → 479.250 | 443.318 → 434.565 | 440.725 → 431.986 |
| group-fallback I | 691.913 → 680.074 | 663.129 → 652.648 | 661.052 → 650.451 |
| group-fallback W | 712.090 → 681.595 | 670.743 → 658.550 | 668.906 → 656.575 |
| diagnostics-off I | 383.730 → 17.551 | 未测 | 未测 |
| diagnostics-off W | 376.019 → 45.908 | 未测 | 未测 |

S2 对齐六次最终任务，修复前均原坐标 hit=0、translation=0、miss=4096；修复后均原坐标 hit=0、实际 translation=3596、remapped=3596、miss=500、reason=1。58×62=3596 个旧完整瓦片可映射，500 个新露出项仍精确重建，未预设重建降为 0。后续同坐标读取才计原坐标 hit，首次消费平移项独立计数；核心测试验证这一计数生命周期。

对齐 S2 正常任务执行 340.826–353.934→7.458–8.928ms，嵌套合成/LOD/blit 338.289–351.938→5.170–6.585ms。Release→完整视口接收 354.111–383.542→16.422–47.485ms。命令接纳 0.007–0.021→0.008–0.010ms，队列 0.006–0.028→0.009–0.012ms；不是通过跳过提交或最终正常帧缩短。发布→worker 首读仍可达 14.199ms、end→actor 可达 8.144ms，剩余间隔未被写成渲染本身或物理显示时间。

非对齐两对均 reason=6、translation=0、miss=4096。四段 Release 收尾优化版仍较慢约 1.35–28.28ms；首对执行更慢，反序对执行差异反向，不能声称“无退化”或将差异确定归因于算法/环境。样本有限，**无非对齐收益，差异未归因**；未继续跑到最好结果或扩大产品修改。S3 两段均 reason=4、translation=0、miss=4096，数字小幅变化不作为组/蒙版优化证据。诊断关闭对齐各一次仍见明显收尾下降，但不能据此量化诊断扰动或宣称零开销。

## 资格成本与内存

| S2 对齐运行 | I 资格/映射合计 ms | W 资格/映射合计 ms |
| --- | ---: | ---: |
| 1 | 1.823 | 1.586 |
| 2 | 1.899 | 1.484 |
| 3 | 1.710 | 1.557 |

最终缓存条目两侧均 4096，保留计账 5,529,792→5,628,096 B，增加 98,304 B（96KiB，证书/状态元数据每项增加 24 B）。预算仍 96MiB；未增预算、预热或强持有所有源像素。重映射临时第二张 HashMap 的桶存储有界于旧表条目数，移动 Entry/Mip Arc，不复制 Mip/源像素；上述 retained_bytes 不含分配器余量或临时桶的峰值，**峰值 RSS 未测**，不能声称内存完全不增。

逐段缓存记录如下，箭头仍为前→后；h/t/m 分别为原坐标命中/实际平移复用/重建。资格区间只在优化版拆分，回退行的该区间包含清理旧表，不是全部新增开销。reason=1 合格、6 非对齐、4 栈不支持。

| 运行/段 | h/t/m 前→后 | 后资格/映射 ms；reason | 条目 前→后 | 保留 B 前→后 |
| --- | --- | --- | --- | --- |
| aligned-1 I | 0/0/4096 → 0/3596/500 | 1.823；1 | 4096 → 4096 | 5,529,792 → 5,628,096 |
| aligned-1 W | 0/0/4096 → 0/3596/500 | 1.586；1 | 4096 → 4096 | 5,529,792 → 5,628,096 |
| aligned-2 I | 0/0/4096 → 0/3596/500 | 1.899；1 | 4096 → 4096 | 5,529,792 → 5,628,096 |
| aligned-2 W | 0/0/4096 → 0/3596/500 | 1.484；1 | 4096 → 4096 | 5,529,792 → 5,628,096 |
| aligned-3 I | 0/0/4096 → 0/3596/500 | 1.710；1 | 4096 → 4096 | 5,529,792 → 5,628,096 |
| aligned-3 W | 0/0/4096 → 0/3596/500 | 1.557；1 | 4096 → 4096 | 5,529,792 → 5,628,096 |
| nonaligned I | 0/0/4096 → 0/0/4096 | 1.659；6 | 4096 → 4096 | 5,706,256 → 5,804,560 |
| nonaligned W | 0/0/4096 → 0/0/4096 | 1.624；6 | 4096 → 4096 | 5,706,256 → 5,804,560 |
| nonaligned-reverse I | 0/0/4096 → 0/0/4096 | 1.592；6 | 4096 → 4096 | 5,706,256 → 5,804,560 |
| nonaligned-reverse W | 0/0/4096 → 0/0/4096 | 1.505；6 | 4096 → 4096 | 5,706,256 → 5,804,560 |
| group-fallback I | 0/0/4096 → 0/0/4096 | 1.627；4 | 4096 → 4096 | 5,529,984 → 5,628,288 |
| group-fallback W | 0/0/4096 → 0/0/4096 | 1.448；4 | 4096 → 4096 | 5,529,984 → 5,628,288 |
| diagnostics-off I | 未测 | 未测 | 未测 | 未测 |
| diagnostics-off W | 未测 | 未测 | 未测 | 未测 |

## 逐进程实际输入节奏及同步开销

以下每格按 I / W，gap 三值为 p50/p95/max（每段 n=119），lateness 为相对 Press 返回后原定 8ms 时刻表的最大延后（n=120），单位 ms。持续 Move 为首至末 Move 的真实时长，不含 Press→首 Move；同步 sendEvent 耗时另表，不可代替节奏。W 中残留的开始延迟及 250–439ms 累积延后没有因本次收尾优化消失。

| 运行/版本 | I/W Press→首Move | I/W 持续Move | I；W gap p50/p95/max | I/W lateness max |
| --- | ---: | ---: | --- | ---: |
| aligned-1 before | 7.559 / 106.748 | 952.957 / 1148.128 | 7.999/8.178/8.805 ; 12.101/16.683/17.157 | 0.706 / 294.746 |
| aligned-1 after | 7.345 / 94.198 | 952.576 / 1169.796 | 8.053/8.568/8.943 ; 12.239/16.688/17.749 | 0.650 / 309.587 |
| aligned-2 before | 8.614 / 72.793 | 951.243 / 1272.049 | 7.996/8.409/8.853 ; 13.495/16.709/17.353 | 0.608 / 384.747 |
| aligned-2 after | 8.750 / 79.124 | 952.312 / 1131.075 | 8.000/8.178/8.711 ; 11.380/16.727/17.812 | 0.913 / 250.100 |
| aligned-3 before | 7.594 / 74.089 | 952.919 / 1264.450 | 8.005/8.490/8.626 ; 13.607/16.910/17.773 | 0.841 / 378.445 |
| aligned-3 after | 7.558 / 99.292 | 952.340 / 1220.892 | 8.000/8.206/8.618 ; 12.746/16.810/17.328 | 0.639 / 361.661 |
| nonaligned before | 8.553 / 70.342 | 951.783 / 1291.426 | 8.001/8.306/8.936 ; 13.828/16.712/23.928 | 1.106 / 401.662 |
| nonaligned after | 7.561 / 71.030 | 953.201 / 1266.464 | 8.002/8.520/8.644 ; 14.073/16.719/17.003 | 0.679 / 377.394 |
| nonaligned-reverse before | 7.622 / 94.099 | 952.106 / 1231.903 | 8.007/8.578/9.463 ; 12.749/16.740/17.424 | 1.062 / 365.804 |
| nonaligned-reverse after | 7.622 / 91.356 | 952.330 / 1245.364 | 8.000/8.132/8.552 ; 12.528/16.764/16.939 | 0.517 / 382.837 |
| group-fallback before | 7.723 / 117.308 | 952.026 / 1275.175 | 8.000/8.312/8.766 ; 13.679/16.773/17.004 | 0.775 / 439.238 |
| group-fallback after | 7.418 / 105.831 | 952.401 / 1230.507 | 8.001/8.452/9.127 ; 13.639/16.891/17.398 | 0.456 / 376.187 |
| diagnostics-off before | 7.191 / 76.159 | 952.949 / 1243.663 | 8.001/8.906/8.990 ; 13.431/16.834/17.041 | 0.101 / 365.958 |
| diagnostics-off after | 8.614 / 93.223 | 951.347 / 1130.849 | 8.006/8.483/9.077 ; 10.225/16.670/17.091 | 0.686 / 267.081 |

下表 I；W 分开，Move 同步单位 μs，每段 n=120；心跳单位 ms，三值均 p50/p95/max，具体样本数保留 verified-runs.json/raw.json。心跳观察跨拖动和收尾，前后观察长度不同，不用于推导相同输入延迟。

| 运行/版本 | I；W Move 同步 p50/p95/max（μs） | I；W 心跳 p50/p95/max（ms） |
| --- | --- | --- |
| aligned-1 before | 21.100/32.400/56.200 ; 14.300/31.200/56.400 | 8.000/8.131/8.769 ; 8.048/16.672/106.831 |
| aligned-1 after | 29.200/40.100/54.500 ; 16.400/32.400/48.200 | 8.040/8.550/8.948 ; 12.265/16.693/94.293 |
| aligned-2 before | 25.300/36.000/64.500 ; 15.300/29.800/40.900 | 7.994/8.508/8.947 ; 8.059/16.698/72.884 |
| aligned-2 after | 26.800/41.500/55.000 ; 16.100/33.300/54.900 | 8.000/8.177/9.276 ; 11.392/16.788/79.143 |
| aligned-3 before | 26.600/37.600/62.600 ; 17.500/34.300/63.700 | 7.997/8.438/8.588 ; 8.107/16.753/74.109 |
| aligned-3 after | 25.000/36.900/55.200 ; 16.400/34.500/55.800 | 8.002/8.185/8.588 ; 12.072/16.853/99.322 |
| nonaligned before | 23.300/38.200/57.500 ; 16.900/36.000/43.700 | 7.998/8.208/8.995 ; 8.033/16.705/70.442 |
| nonaligned after | 22.900/37.600/51.700 ; 15.900/33.400/75.300 | 7.996/8.500/8.604 ; 8.255/16.692/71.060 |
| nonaligned-reverse before | 24.000/39.800/61.900 ; 17.100/34.500/51.900 | 7.995/8.609/9.504 ; 8.022/16.698/94.187 |
| nonaligned-reverse after | 27.300/38.000/55.400 ; 16.700/34.900/42.900 | 8.002/8.450/8.589 ; 8.020/16.745/91.373 |
| group-fallback before | 26.100/42.100/57.300 ; 17.300/36.400/43.200 | 7.995/8.220/9.201 ; 8.035/16.719/117.330 |
| group-fallback after | 23.000/39.200/52.100 ; 18.500/41.200/57.200 | 8.000/8.223/9.071 ; 8.016/16.698/106.031 |
| diagnostics-off before | 20.000/32.200/50.000 ; 13.100/32.600/45.100 | 7.999/8.884/8.982 ; 8.030/16.778/76.439 |
| diagnostics-off after | 19.000/31.200/58.400 ; 13.700/29.000/40.500 | 8.004/8.495/9.096 ; 10.063/16.669/93.241 |

## 可复制命令与证据

先进入对应英文工作树；激活既有 VS x64 shell，PATH 仅当前进程加入已安装 Ninja 和 Qt bin，不改系统配置。两边分别构建自己的目标：

```powershell
& 'D:/VS/Microsoft Visual Studio/2022/Community/Common7/Tools/Launch-VsDevShell.ps1' -Arch amd64 -HostArch amd64 -SkipAutomaticLocation
$qt = 'E:/Git/DrawVerse-local-build/.tools/qt/6.8.3/msvc2022_64'
$ninja = 'D:/VS/Microsoft Visual Studio/2022/Community/Common7/IDE/CommonExtensions/Microsoft/CMake/Ninja'
$env:PATH = "$qt/bin;$ninja;$env:PATH"
$env:QT_QPA_PLATFORM_PLUGIN_PATH = "$qt/plugins/platforms"
$env:QML_IMPORT_PATH = "$qt/qml"
cmake --build build --target qt_layer_move_baseline qt_transform_tests -j2
cargo test --manifest-path core/Cargo.toml -p paint-render --release --locked --lib --test translation --test cache
cargo fmt --manifest-path core/Cargo.toml --package paint-render --package paint-ffi -- --check
cargo clippy --manifest-path core/Cargo.toml -p paint-render --release --locked --lib -- -D warnings
python -m unittest discover -s tools -p 'test_*move*.py'
```

按前/后工作树各自执行、每次使用不存在的新输出目录并交替顺序（`--repeats 1` 保证每个 invocation 单独进程，重复三对）：

```powershell
python tools/measure_layer_move.py build/bin/Release/qt_layer_move_baseline.exe artifacts/issue-2-main-c2b/reproduce-new --scenes 2 --repeats 1 --delta 384 128 --trace --core-trace
# 非对齐改 --delta 383 127；S3 改 --scenes 3；关闭诊断去掉 --trace --core-trace
python tools/analyze_move_completion.py artifacts/issue-2-main-c2b artifacts/issue-2-main-c2b/reanalysis-new
```

原始证据统一在忽略目录 `artifacts/issue-2-main-c2b/`：七组 `aligned-1/2/3`、`nonaligned`、`nonaligned-reverse`、`group-fallback`、`diagnostics-off` 各有 before/after、invocation、summary、raw/test/core 日志；原始文件未覆盖。`order-*.json`、`run_pairs.py`、`reverse_nonaligned.py` 保留实际顺序与命令；`final-analysis/attribution.json` 精确关联 28 段（关闭诊断 4 段明确 unmeasured），无进程/场景失败。`verified-runs.json` 和一次性 `verify_results.py` 复核 14 个 PID、状态、硬件、资源、全视口摘要、所有实际输入，未改写原始数据。

专项日志为 `core-final-tests.log`、`ffi-*.log`、`native-regressions/test.txt`、`offline-tool-tests.log`、`render-clippy.log` 和两侧 build 日志。初次编译的 u32 错误与修正后日志也保留；它发生在实现编译阶段，不是像素不等价、崩溃或被替换的测量失败。未运行全量 tools/check.py、无关输入用例或旧性能矩阵。

## 结论与边界

- **已确认**：对可证明的平面 Normal、均匀固定贡献、整瓦片相对平移，在原完整正常帧内复用已有合成 Mip。完整像素/生命周期、最终位移、历史及旧帧拒绝专项通过；对齐 S2 的实际重建从 4096 降为 500，而非减少画质或省略最终帧。
- **测量支持**：本机同条件三对 S2 显示主要合成/LOD/blit 与收尾下降；没有任意拖动、非对齐或复杂组/蒙版提速结论。非对齐较慢样本与未归因差异保留供审查，不隐去。
- **未验证**：学生真实作品、复杂实际栈/噪声、实体手绘板、肉眼手感与物理输入到显示延迟；不需要等待这些设备测试来证明局部像素等价，也不宣称整体移动不卡。

C1 原组合像素失败、两次历史取消、C2-A 探索时 Qt 抓图崩溃的原记录/分类继续保留；本轮未修复、未归咎环境，也没有重复到绿来取代历史。本轮没有异常退出或新正确性失败。未进入常驻 BEGIN 延迟、方向滤波或另一轮 GPU/缓存架构优化。仅提交并推送 Fork 供审查，不创建 PR、不合并、不关闭 Issue #2。
