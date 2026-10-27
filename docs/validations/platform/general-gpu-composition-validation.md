# 通用 GPU 合成验收计划与阶段记录

日期：2026-10-27。对应[设计](../../platform/general-gpu-composition-design.md)与[实施提示词](../../planning/general-gpu-composition-prompt.md)。

## 当前状态

G0已交付基线、真实Qt同设备三通道直显试验及选型（第9–10节）；G1已交付不可变计划/有界传输（第11–12节）；G2已接实际Canvas的有限Normal/自身蒙版增量GPU合成及CPU后备（第13–14节）；G3有限范围本地移动/自由变换、交接与资源复用已实现，性能及剩余回归仍部分完成（第15–17节）。默认显示保持，G4–G5未执行。第1–7节为总体验收要求，第8节保留初稿检查历史，不代表运行结果。没有新增实体数位板、物理显示或跨平台通过结论。

| 阶段 | 状态 | 必须补充的证据 |
| --- | --- | --- |
| G0 | 基线/试验/选型完成，性能差距明确 | 第10节；设备真实故障、物理显示和其他API未测 |
| G1 | 计划/传输纵向链完成，默认显示保持 | 第12节；真实GPU资源与窗口epoch绑定仍待G2 |
| G2 | 有限支持范围纵向完成，显式启用 | 第14节；其他API/完整大图/剩余笔刷矩阵与故障恢复仍待 |
| G3 | 有限范围实现完成，性能/完整回归部分完成 | 第15–17节；准确版本配对、保留失败及剩余门槛逐批记录 |
| G4 | 未执行 | 组/蒙版/剪贴/混合的支持矩阵与后备矩阵 |
| G5 | 未执行 | 稳定性、默认启用、Release和相关回归 |

## 1. 记录与基线

每批记录源码 HEAD/工作区差异摘要、二进制摘要、构建类型、Qt/驱动/GPU、实际窗口后端、DPR/刷新率、文档/视口/图层/瓦片规模、后端支持判定与后备原因。before/after 使用相同夹具、输入轨迹、窗口与观测设置，独立隔离配置和暂存目录；不同源码或二进制不能合并成同一基线。

复用 InteractionTrace、CanvasPresentationObservation 和已有测量脚本；只为新边界增加必要、有界、默认关闭的字段。原始产物建议放忽略目录 `artifacts/general-gpu-composition/<stage>/<run>/`，报告记录相对位置与摘要，不提交构建输出。保留诊断关闭对照和溢出数；溢出/超时/输入前提失效不能当通过。

G0 在测量前固定夹具、CPU/GPU误差口径、缓存/上传/工作预算及性能目标，并记录依据。初始热资源交互目标：已确认正确目标且资源就绪后，首个有效Move至对应内容被节点采用，P95不超过两个实测刷新周期；这只是工程目标，尚未达成。冷启必须报告总耗时与准备分段，并在G0结束前为后续阶段登记预算，不能看过after再放宽。GPU完成或物理屏幕延迟没有可靠手段就标未测。

## 2. 时间与画面身份

分别观察：

| 标记 | 含义 |
| --- | --- |
| t0 | 实际输入到达目标画布；进入操作和首次Move分别记录 |
| t1 | 小命令成功入队，带sequence |
| t2 | 核心确认目标/会话身份，可用时记录Actor开始/结束 |
| t3 | 计划与所需源数据准备完毕 |
| t4 | Qt worker读取/解码/GUI接纳完成，分别保留可用边界 |
| t5 | 上传资源更新已记录/提交，不能称GPU已完成 |
| t6 | 渲染节点采用了对应几何与有效内容身份 |
| t7 | 若可可靠关联，窗口提交包含该内容的帧 |
| t8 | 独立设备实测屏幕/笔到光子；无设备则未测 |

关联至少包括文档generation、窗口/device epoch、view/request、plan/publication、revision、源内容身份、操作sequence和本地几何序号。按实际路径补字段，不能用单一revision替代未提交的几何。任意frameSwapped、当下getter值、控制框移动或一条ready回调不能证明对应图像已变化。

性能阶段不抓图、不逐事件写盘；结束后导出。像素正确性使用独立阶段，同一Canvas的request/ready/完成区间冻结内容与观察身份，保留原联合像素断言。合成事件发生器记录计划/实际发送间隔、首Move延后与累计延后；Qt注入不能称实体笔输入。

每个主性能场景至少三次独立进程；每次分别报告首次、预热后和持续更新的样本数、P50/P95/max、实际输入节奏与失败。资源首次初始化、CPU OS热页、GPU热缓存和真实冷盘分别标注，不把第一次进程运行自动称为冷盘。

## 3. 必须通过的功能矩阵

| 类别 | 最小场景 | 断言 |
| --- | --- | --- |
| 基础合成 | 多层Normal、透明/半透明、opacity/fill、可见性、负偏移/画布边界 | 对照CPU，透明像素与显示转换正确 |
| 隐式底色 | 白底、透明底、擦除过的显式透明瓦片 | 不把显式透明当缺页或恢复白色 |
| 蒙版 | 空/有效、黑白灰覆盖、显隐/密度、链接/固定、自身/祖先 | 复用项目反向隐藏量语义；不支持必须后备 |
| 组 | Normal隔离、PassThrough、嵌套、低opacity/fill、有蒙版 | 不错误展平；穿透对backdrop影响一致 |
| 剪贴 | 单/多剪贴层、普通/组基底、嵌套与重排 | alpha、链边界和背景依赖一致 |
| 混合 | 所有现有模式逐行登记，包含非分离模式与Dissolve | 支持/后备明确；坐标和种子稳定 |
| LOD与导航 | 100%、放大、极小缩放、旋转、DPR1/2及已有高DPI条件 | 遵守先合成再LOD；像素边缘/采样相位可对照 |
| 笔触 | 画笔/橡皮/涂抹、湿混和选区限制、连续24笔后再画 | 无丢失/重复笔触；增量瓦片与CPU结果一致 |
| 移动 | 单/多选、组内、Ctrl拾取不同目标、固定/祖先有效蒙版 | 正确目标；首帧与持续跟随分开；未支持后备 |
| 自由变换 | 平移/缩放/旋转/透视/Warp、选区/羽化、源往返 | 非法几何保留合法结果；折叠覆盖和最终六种插值保持 |
| 交接 | 立即释放、确认/取消、Undo/Redo、延迟主视图/仅概览就绪 | 不闪旧帧，不提前释放预览；单历史事务 |
| 多文档窗口 | 文档切换、同文档浮窗、跨窗口/DPI/隐藏恢复/关闭 | 无跨窗口GPU对象误用、旧文档回包或泄漏 |
| 输出一致 | 显示、CPU吸色、缩略图、导出、保存重开 | 文档真值与既有持久化保持；预览近似明确 |

首批夹具覆盖 1024²、4096²满幅及非纯色/半透明细节，增加大尺寸稀疏文档，包含现有 C1/C2 夹具以便有限对照。不能只用单红层证明复杂图层支持，也不能用巨大空白画布证明密集绘画性能。

核心最终像素、历史、取消和文件结果沿用原严格比较。新增GPU显示测试同时记录线性浮点误差和转换后RGBA误差；G0先定义对应场景的容差与依据，阈值敏感模式单列。不能只看稀疏采样点：小夹具全图对照，大夹具保存完整对照摘要、误差分布和差异图。截图色彩转换与参考输出必须处于相同空间。

## 4. 增量与资源证据

- 无变化的连续重绘不重复上传相同源瓦片，不推进业务revision，不持续空转。
- 一块瓦片变化只上传必要新内容；组/蒙版/剪贴依赖导致的合成重算与源上传分别计数。
- 热资源移动/变换只更新必要几何/参数；若背景依赖要求重算，记录实际通道与区域，不伪称零工作。
- 纯视图导航复用有效缓存；新露出区域按需准备。计划/几何变化不无条件清空全部纹理。
- 统计GPU缓存、暂存、在途包、CPU包/租约pin字节、峰值和预算拒绝。按真实资源格式、对齐与中间通道计账。
- 超预算、慢消费者、取消/关闭及持续COW后，缓存/租约最终回收且有界。GPU对象释放等待安全时点，不能以CPU计数归零推断完成。
- 记录正常快速路径完整帧读回次数，应为零；测试/截图/明确最终作业读回单独计账。初始/变更瓦片CPU→GPU上传不算违规。

## 5. 故障与兼容

确定性注入能力不支持、初始化失败、上传/渲染资源失败、设备失效、显存/暂存预算不足、冷页读取错误、租约过期、乱序/重复包和窗口device epoch变化。

必须区分：可用CPU后备、保留最后正确画面等待、不可恢复的数据读取错误。不能用后备隐藏损坏数据或假装编辑成功。失败不改文档、不重复提交、不丢已接纳笔采样；取消/关闭无死锁，重试有界。比较故障前后核心像素、历史游标、操作回执与最终主帧身份。

至少覆盖强制CPU、Qt软件场景图、真实Windows D3D11显示、wgpu数值GPU不可用但Qt显示可用，以及相反组合。只列通过的平台/API；D3D11通过不推断D3D12/Metal/Vulkan，软件测试不证明实体驱动稳定。

## 6. 工程门禁

按阶段运行相关核心、渲染、协议与Qt定向回归；出现新失败先归因，不反复盲跑扩大矩阵。新增协议检查长度/缓冲区、版本/能力、句柄/租约生命周期与并发，运行生成头和C/C++静态/动态四消费者检查。

最终门禁复用 `python tools/check.py`（默认含Qt；SDK通过 `--qt-prefix` 指定），不新增替代总入口。相关Clippy/fmt、shader构建/产物检查与Release按现有工具执行。G0试验仅按需构建，不能把实验默认装入产品；G5真实运行时交付更新标准 `build/bin/Release/drawverse.exe`，记录启动和实际后端。完整检查未跑或失败必须明确写出范围，不能用定向通过代替。

## 7. 每阶段结果模板

```text
阶段/日期：
准确源码HEAD、工作区差异、二进制摘要：
硬件/Qt/驱动/窗口后端/DPR/刷新率：
完成行为与支持矩阵增量：
CPU后备条件与原因：
正确性：运行命令、样本、结果、差异及原始证据：
性能：before/after、样本数、P50/P95/max、测量边界：
首帧：进入、首次Move、冷准备、热资源节点采用、帧关联：
资源：上传/读回/缓存/暂存/租约峰值及回收：
故障、软件及硬件结果：
ABI/头/shader/Release/完整检查范围：
未测、失败、超时、溢出与未归因记录：
下一阶段/阻塞证据：
```

## 8. 文档交付检查（2026-10-27）

本次只检查新增设计、提示词、验收计划与入口链接、格式及变更范围。结果：

- 对三份新文档及三个修改入口运行一次性 Python 静态检查：共6份文档、382个本地 Markdown 链接均存在，代码围栏成对；三份新文档无尾随空白且有最终换行。
- `git diff --check -- docs/README.md docs/validations/README.md docs/platform/gpu-acceleration-design.md` 通过。新文件尚未跟踪时不在该命令覆盖范围，已由上述脚本检查空白。
- `git status --short` 核对本次交付为三份新文档与三个入口修改，没有新增运行时代码或构建产物。

未编译运行时、未跑核心/Qt测试、未覆盖现有Release，ABI/state/文件格式保持现状；文档静态检查不构成功能或性能验收。

## 9. G0 实施前冻结（2026-10-27）

起点：`General-purpose-GPU-synthesis` / `d13e62b`；工作区原有三份新文档和三个入口修改保留。当前分支无 upstream，不根据历史 master 指令切换分支。以下门槛在运行新增试验前登记，不根据结果放宽。

- 技术试验独立 opt-in 目标，不链接进应用。真实 QQuickWindow 的 QRhi 设备运行三通道：线性 RGBA32F 源乘反向隐藏量蒙版 → 与半透明背景 Normal 合成 → display_pixel 同式 sRGB 预乘转换；结果纹理直接供同场景图采样。至少 64² 非纯色/透明/半透明像素，全像素线性绝对误差 ≤2e-6，显示 RGBA8 每通道误差 ≤1。RGBA16F 本轮不放行。
- 正常试验帧不读回；独立正确性轮次显式读回中间浮点与显示纹理，并抓真实窗口。重复重绘不重复上传源、不重复合成。失效/重建和第二窗口分别分配资源，软件模式明确 CPU 路径。试验失败不得启用产品路径。
- 试验硬预算：64²，三张 RGBA32F 输入、两张 RGBA32F 中间及一张 RGBA8 输出共 344064 字节纹理 payload；上传 196608 字节、三个 16 字节 uniform；单帧最多三通道、一批输入上传、无 mip/MSAA、最多一组在途资源；实际驱动对齐/对象成本另记未测。仅容许这套固定尺寸夹具，不能当产品显存预算。
- 当前移动/变换基线：同一 HEAD 的按需 Release 探针，真实 Windows D3D11；1024²/4096² 满幅红色叠六个彩色半透明笔触，白底；窗口 1024×720。分别记录立即进入、资源就绪、持续 8ms 输入、释放；每主场景三次独立进程。使用 CanvasPresentationObservation / InteractionTrace，不在计时区抓图、写盘。未确认目标/无源阶段单列。
- 节点采用需匹配 canvas/window/generation/source/revision 及实际顶点内容身份，移动另匹配绝对 presentationDelta；不得用控制框变化替代。首次有效输入到节点采用的热资源目标仍为两个实际刷新周期。frameSwapped 只在渲染线程绑定已采用身份时记录，仍非 GPU 完成/物理显示。
- 冷准备后续预算预登记：同夹具进程首次资源准备 ≤500ms（1024²）、≤1500ms（4096²）；不含 OS 冷盘承诺，超限保留失败。G1 初始协议上限候选：4096 节点、4096 源瓦片、64MiB 包/租约、16MiB 上传暂存、256MiB GPU payload、最多两包在途、4M 输出像素；必须在 G1 按实际格式/闭包/对齐重新核实可行性，未经实现不称已保障。
- 软件、设备重建、预算/格式拒绝是 G0 实验的有限验证，不等于文档编辑故障恢复已完成。核心历史/文件/最终像素不改；ABI1.80/state45 保持。

### 运行前夹具修订 v2

初版原始记录 `baseline-transform-1024-1/2` 的最终历史深度为7，构造后预期为9，断言失败保留；从核心 `History::trim` 可见默认64MiB预算可以淘汰构造历史，不能把相对深度当作不受预算影响的事务计数。初版 `baseline-move-1024-2` 另有探针遗漏 timer timeout 连接，sent=0 的失败，已修正，不能用作性能样本。初版 `baseline-move-1024-1` 错把未预热的 CPU 首拖要求为 resident，失败也保留。

夹具 v2 在计时前通过公开 ORA 保存/重开，保留满幅内容和图层、初始历史为0。继续严格要求最终 history +1、Undo 恢复原内容，不放宽产品历史断言。文件往返可能量化像素，故 v1/v2 不合并；构造 IO 不在首拖计时内，也不称冷盘测试。v2 后来被下述 v3/v4 取代，不纳入最终基线。

进一步冻结 v3：必须使用产品 `CanvasPane.qml` 的下层/上层 Loader 与最终 ShaderEffect，单独 CanvasItem 的变换 forward 纹理缺少下层，不能作为整幅内容对照。`v2-transform-1024-1` 保留联合像素失败。所有采集前关闭笔刷轮廓/图片光标，正确性抓图时仅临时隐藏变换控件。早期对 `pane-transform-1024-1` Undo 差异的“光标覆盖”判断并不成立：`final-transform-1024-1` / `undo-diagnosis` 继续失败，最后确认初始抓图采用了旧缩放密度帧，7101个差异像素集中在彩色笔触边缘。初始帧增加当前 viewport region 和 ceil(region×zoom×DPR) 尺寸匹配后，CPU帧与窗口全图严格恢复通过，原断言保留。

最终 v4 在六笔构造后用同图层选择作有序 Actor 屏障，等待修改完成而不依赖可被预算淘汰的构造历史深度；`baseline-v3-move-4096/run-1`、`baseline-v3-transform-4096/run-1` 的构造失败（实际5/预期8）保留。正式结果只取 fixture_version=4、相同基线二进制的四组×三进程，不混入 v1/v2/v3 或诊断轮次。

## 10. G0 交付记录（2026-10-27）

### 10.1 版本、构建与运行方法

实际分支 `General-purpose-GPU-synthesis`，起点 HEAD `d13e62b`；初始工作区三份入口修改、三份新文档均已核查，未切分支或重建历史方案。本次代码增加两个默认不构建的独立探针、两份着色器、采集/分析脚本；产品只扩展默认关闭的观测（CPU帧revision、实际顶点hash/首顶点、transform提交sequence），未新增Rust协议、文档格式或GPU合成产品路径。ABI1.80/state45保持。

Windows / Qt6.8.3 MSVC x64 / Release / 真实 Windows D3D11 threaded场景图；NVIDIA GeForce RTX4060 Laptop GPU，驱动617.42，报告显存8188MiB；DPR1.5。Qt报告屏幕165.019Hz，约6.06ms/周期，仅作参考，未外部量测实际面板刷新周期；不能凭该元数据宣布达到“两个实测刷新周期”。

所有原始产物根目录为 `artifacts/general-gpu-composition/G0/`（忽略目录）。每轮 `invocation.json` 保存HEAD、branch、工作区状态、逐源码SHA256和EXE摘要，`environment.json` 保存Qt/观测配置，`process.json` 保存退出码与溢出门禁；Qt原生日志在 `test.txt` / `qt.log`。基线四组的EXE完全相同，试验源码后续独立修订也由每轮清单如实记录。

| 二进制 | SHA256 |
| --- | --- |
| qt_gpu_composition_baseline.exe（v4） | `79b11e1c2e5725b93318f7d2320938213da99179837108e0f649ca89b7d87321` |
| qt_gpu_composition_probe.exe（最终） | `19d814cbf301d28148c95926e300f121474bee85d467051a8f76b527ef27f331` |
| build/bin/Release/drawverse.exe | `fccd2ba226b259dc2ec76503eaae67c73e460a96458cbe8bbe93f3654a3d0ea1` |

复现示例（先按README准备Qt和MSVC环境，输出目录必须不存在）：

```powershell
cmake -S . -B build -DDRAWVERSE_G0_EXPERIMENT=ON
cmake --build build --config Release --target qt_gpu_composition_probe qt_gpu_composition_baseline -j2
python tools/measure_general_gpu_g0.py build/bin/Release/qt_gpu_composition_probe.exe artifacts/general-gpu-composition/G0/new-probe --repeat 3
python tools/measure_general_gpu_g0.py build/bin/Release/qt_gpu_composition_baseline.exe artifacts/general-gpu-composition/G0/new-move --operation move --size 1024 --trace --repeat 3
python tools/measure_general_gpu_g0.py build/bin/Release/qt_gpu_composition_baseline.exe artifacts/general-gpu-composition/G0/new-transform --operation transform --size 4096 --trace --repeat 3
python tools/analyze_general_gpu_g0.py artifacts/general-gpu-composition/G0 artifacts/general-gpu-composition/G0/analysis.json
```

### 10.2 同设备三通道直显

`probe-final-d3d/run-1..3` 全通过；`probe-final-software` CPU路径通过。真实256²逻辑窗口在DPR1.5下抓384²窗口图；源夹具64²、4096个非均匀透明/半透明像素全部比较。通道为源×反向alpha隐藏量蒙版密度0.75 → Normal over半透明背景 → Rust同式sRGB预乘显示转换。三通道共用窗口QRhi和命令缓冲，最终RGBA8纹理由该窗口QSG直接采样。

三轮相同结果：线性最大绝对误差 `5.960464477539063e-8`；显示RGBA8最大误差1，16384通道分量中15939相等、445差1、0超限；真实窗口RGB最大误差1。正常九个试验帧上传批次1、合成通道共3、读回0；之后独立正确性阶段显式读回2次，不用于生成显示纹理。完整浮点/字节参考及GPU结果、真实窗口PNG均留在各轮目录。

第二窗口QRhi身份与主窗口不同，各自创建资源。主窗口关闭persistent图形后隐藏/releaseResources/重开，created=2/destroyed=1、上传批次=2，证明本轮场景图资源回收重建；不等于真实DeviceRemoved恢复。固定纹理payload344064字节、输入上传196608字节、uniform48字节，无mip/MSAA；未测驱动实际对齐、pipeline/描述符/命令缓冲显存和峰值。没有执行格式/预算/分配失败注入，不能声称产品故障后备完成。

API/版本和后续技术决定见设计第12节。保留 `probe-d3d-1`（NDC方向错误，线性误差1）、`probe-d3d-2`（原64逻辑窗口被Windows最小宽度约束，窗口误差251），修正坐标和窗口尺寸后旧容差不变。非D3D11/不支持RGBA32F/资源分配失败在试验中显式失败；软件分支使用CPU参考。产品默认路径保持稳定，未接入试验。

### 10.3 当前产品首位移基线

正式目录：`baseline-v4-move-1024/run-1..3`、`baseline-v4-move-4096/run-1..3`、`baseline-v4-transform-1024-isolated/run-1..3`、`baseline-v4-transform-4096/run-1..3`。满幅红色普通层、六处彩色半透明细节、白底，实际CanvasPane（含上下层Loader和ShaderEffect），窗口1024×720。通过ORA往返清构造历史并可能量化像素；不称OS冷盘或实体输入测试。

每进程三段：进入即发一次有效Move；等待本轮资源就绪后一次Move；120个计划8ms的持续Move。移动首段确实走未预热CPU路径，不把它强制改成resident；变换先等待真实BEGIN元数据/控制点，等待耗时单列在进入→内容，不能把之后的Move时间当进入成本。所有计时区不抓图、不逐事件写日志；有界内存记录结束后导出。正确性抓图独立于计时，冻结对应内容身份，联合检验露出白底/移动红色实际内容，确认严格单历史事务，Undo CPU帧及窗口全图逐像素恢复。

下表单位ms，数字均为 **P50 / P95 / max**，最近秩分位；首次/就绪各n=3，小样本P95等于最大值。持续仅统计确实被节点采用的输入，不虚报所有120个输入均显示。所有正式轮次退出成功，trace/状态/节点/帧记录溢出0，计划外输入0。

| 场景 | 首Move→内容节点 | 就绪后单Move→内容节点 | 持续匹配数/360 | 持续输入→节点 |
| --- | --- | --- | --- | --- |
| 移动1024² | 218.875 / 253.193 / 253.193 | 23.389 / 35.919 / 35.919 | 333 | 10.949 / 11.296 / 11.992 |
| 移动4096² | 310.036 / 310.394 / 310.394 | 23.657 / 34.579 / 34.579 | 329 | 10.912 / 11.138 / 11.676 |
| 变换1024² | 25.506 / 29.550 / 29.550 | 26.819 / 40.962 / 40.962 | 295 | 26.101 / 39.454 / 42.521 |
| 变换4096² | 27.615 / 51.600 / 51.600 | 27.152 / 27.183 / 27.183 | 295 | 26.214 / 38.458 / 41.368 |

| 场景 | 进入→首次内容节点 n=3 | 持续段首Move→首次可见内容 n=3 | 实际发送间隔 n=357 | release/确认→最终主节点 n=9 |
| --- | --- | --- | --- | --- |
| 移动1024² | 219.968 / 254.222 / 254.222 | 11.506 / 11.992 / 11.992 | 10.996 / 11.536 / 18.125 | 84.388 / 96.346 / 96.346 |
| 移动4096² | 310.828 / 311.162 / 311.162 | 11.110 / 22.356 / 22.356 | 10.986 / 11.354 / 18.454 | 535.044 / 556.887 / 556.887 |
| 变换1024² | 286.663 / 288.458 / 288.458 | 40.094 / 40.166 / 40.166 | 13.164 / 16.001 / 21.482 | 279.743 / 303.417 / 303.417 |
| 变换4096² | 952.574 / 979.923 / 979.923 | 16.475 / 27.166 / 27.166 | 13.102 / 14.396 / 17.989 | 2986.853 / 3118.029 / 3118.029 |

持续段首次内容可能对应合并后的较晚输入，逐轮 `first_content_input` / matches 和计划累计延后见 `analysis.json`；实际节奏明显不是125Hz。匹配输入关联到渲染线程frameSwapped记录共339/335/301/301条，使用节点receipt epoch与adoption_ns匹配；只是Qt提交关联，不是GPU fence或物理屏幕。不能把未匹配输入当作画面已显示，也不因此删除真实业务命令。

自由变换首次进入至BEGIN可用的GUI观察耗时：1024² 260.547/270.579/270.579ms，4096² 924.518/927.900/927.900ms（n=3，包含QTRY观察粒度），在预登记500/1500ms范围内；它不是精确Actor执行或GPU完成时间。移动立即段的ready_ns只是未等待源时刻，不是准备完成，不能拿约0.3ms报告冷准备。源解码/接纳与上传范围在原始 `move.worker.decodePreparation`、`move.worker.readResident`、`move.gui.acceptPreparation/Resident`、`move.render.createTextures/uploadBatch` 中保留；上传仅CPU记录/提交，不是显卡完成。本轮没有新增Rust Actor内部起止或冷页分段探针。

输入t0/end、入队sequence、transform.submit、目标GUI确认、prepared源尺寸/host字节/上传计数、metadata回执、实际节点顶点hash/绝对位移与CPU frame imageKey、对应帧均保留。CPU立即移动用完成UPDATE及稳定像素证明的最终frame imageKey精确关联，不用“当下getter够新”代替节点采用。现有接口没有完整device epoch/view-request-plan链；此身份链与不可变租约需G1实现。本轮同一窗口/generation/source/真实顶点与图像身份足够限定这些冻结夹具。

诊断关闭对照（仍保留节点观察，n=1，仅界定粗略扰动范围）：`control-move-1024` 首次240.254ms、就绪24.519ms、持续n107 P50/P95/max=10.950/11.057/11.287ms；`control-transform-1024` 首次26.271ms、就绪35.726ms、持续n98为26.157/39.619/45.329ms。不据一次对照推断零开销，未做4096²关闭对照。热首帧多数超过Qt报告刷新率对应的约12.12ms，且真实刷新周期未量测，**首帧性能门槛未通过**；G0是before基线，没有优化after，不能声称改善已实现。

### 10.4 失败、工程验证与下一阶段

上述第9节失败目录均保留。另保留 `baseline-v4-transform-1024/run-1`：与Rust编译重叠，即使功能通过也不进入正式性能统计，其首Move约2804ms；`run-2` 捕获WindowDeactivate（event25，计划外输入1）并失败，未关闭/放宽此门禁。随后无并发构建的 `...-isolated/run-1..3` 才进入正式表。旧v3及其他诊断通过记录不与正式v4合并。原有C1/C2与未归因专项记录未改写。

- `cargo test --manifest-path core/Cargo.toml -p paint-render --release --locked`：34项通过，覆盖CPU参考、组/蒙版/剪贴、先合成再LOD、冷页、取消、缓存与平移。无Rust生产修改；未跑无关全workspace。
- `qt_transform_tests` 定向十行在Windows软件/D3D11各通过，另各2个init/cleanup；原生结果 `qt-related-software.txt` / `qt-related-d3d11.txt`。覆盖pendingMoveObservation、旧/中断抓图关联拒绝、其他编辑清预热两行、Ctrl实际目标、四种最终主帧交接、Actor回执前最新移动。
- Release `drawverse_runtime`、两实验目标、`qt_transform_tests`、四ABI消费者构建成功；生成头检查通过；`ctest --test-dir build -C Release -R '^abi_' --output-on-failure` 四项通过（`abi-tests.log`）；现有移动/液化shader预编译检查通过。构建日志 `release-build.log` 保留 windeployqt 找不到 translations/catalogs.json 的警告（使用no-translations）；不删除该记录。
- 标准 `build/bin/Release/drawverse.exe` / DLL已更新，`--preview` 隔离偏好模式真实启动、三笔绘画并截图、退出0；`release-final/release-smoke.json` / `.log` / `release-preview.png` 保留实际D3D11后端和EXE摘要。初次GUI日志未强制stderr的空日志保留，随后补采显式Qt日志。不把应用冒烟当新增合成已默认启用。
- Python两脚本编译与 `git diff --check` 通过；5份相关Markdown的29个本地链接存在、围栏成对、新增源码尾随空白检查通过（`document-check.json`）。原有 gpu-acceleration-design 的CRLF提示保留，未替他人规范化。未运行 `python tools/check.py` 总门禁，不称完整验收。

已支持增量仅是独立64²三通道实验与原路径准确基线。未测：实际文档通用瓦片/增量上传/预算拒绝、真实DeviceRemoved/TDR、慢消费者/取消关闭租约、多文档故障恢复、其他Qt/API/硬件、DPR1/2矩阵、自由变换旋转/透视/Warp首帧、复杂栈性能、真实冷盘、实体笔压力、GPU完成/物理显示；保持CPU真值与现有后备，不新增这些支持声明。**G1–G5尚未实施**。

下一步G1：复核本轮Qt6.8.3/RGBA32F同窗口路线，按第9节候选预算实现不可变计划、有界瓦片读取、版本能力、身份/租约与取消关闭，并提供CPU对照的可运行传输成果；默认显示行为保持。G2才迁移实际Normal/自身蒙版显示，G3以本轮严格首帧/单历史/最终主视图屏障为before比较，不能先标记G4/G5完成。

## 11. G1 实施前冻结

起点为G0提交 `bf9bef9`，Qt6.8.3/D3D11/RGBA32F证据继续有效；产品最低Qt6.5与默认显示保持。G1仅实现不可变计划和传输，不能以接口存在宣称G2画布GPU合成已完成。

64² RGBA32F瓦片65536字节，4096块满幅4K即256MiB；第9节64MiB候选租约无法容纳它，故在实施前修订为每租约最多256MiB像素引用、8MiB元数据估算，进程所有计划（含在途读取）合计512MiB，最多两租约/会话；进程读取暂存16MiB、单次最多一块64KiB。实际资源按保守上界计账，构造前预检，失败不生成部分可用计划。GPU预算尚未实现，原256MiB候选不能当保证。

协议独立version=1，能力Normal/自身蒙版/LinearRGBA32F显式协商；未知版本/能力、超限、陈旧generation/revision/request/device epoch显式拒绝。新窗口设备epoch淘汰该view旧资源；新文档、取消与关闭释放计划，正在读取的不可变数据持有预算至完成，不靠裸指针或layerID复用。读取可触发冷盘，只允许worker调用；GUI和渲染线程不调用瓦片读取。真实采样和历史顺序保持，显示计划请求不修改文档。

初始计划只接纳完整闭包的根Normal栅格及自身蒙版，组/剪贴/其他blend/置入图与编辑会话明确unsupported；不拆穿透/剪贴闭包，不静默略过蒙版。按必要源坐标传输真实64²瓦片，显式透明与隐式白分别编码，不用2048代理或逐层mip代替精确输入。先验证CPU计划参考与现有Renderer像素一致，再验证版本化ABI读取及Qt worker→真实窗口的纵向传输。

最小门禁预登记：Normal/透明/偏移/隐式白/自身蒙版CPU对照；稳定瓦片身份与COW失效；尺寸/缓冲/能力/预算；乱序与重复请求；取消/关闭/新文档/设备epoch；并发读取与预算回收；Rust生成头与四ABI消费者；Qt worker可运行链、软件/D3D11默认原路径相关回归。性能不在G1增加新结论；失败与未测追加记录。

## 12. G1 交付记录（2026-10-27）

代码从G0 `bf9bef9` 续接，仍为 `General-purpose-GPU-synthesis`，没有切分支或覆盖原三份入口文档改动。G0的同窗口QRhi试验/Qt6.8.3精度与直显代码保持，本阶段没有更换API、格式或路线；未据此补称Qt6.6/6.7或其他后端已验证。G1使用设备无关Rust计划与ABI1.81/version1传输，默认画布行为保持，state45/behavior37/VBR和文档格式不变。

### 12.1 实际支持与纵向行为

根Normal栅格、透明、opacity/fill、偏移、隐式白与链接自身蒙版编译为不可变节点及原始64²瓦片。组、穿透/隔离、剪贴、其他blend、置入图、固定蒙版以及活动笔触/移动/变换/液化会话明确unsupported，不静默拆闭包；准备失败不改文档last_error。G1不接管这些编辑中的产品显示，原CPU/常驻预览保持。

新独立显示邮箱不消耗业务sequence；288个显示请求与24笔Begin/End交错，最终24个已提交笔触及精确End序号2..48保留。Stable content ID/COW使旧计划像素保持不可变；私有Weak缓存8192项，不传原生指针。调用方提供完整view/request/epoch/generation/revision/lease/publication身份，乱序、旧设备逻辑epoch、新文档和跨core拒绝。关闭与在途引用测试证明取消后不发布缓冲，预算保留到最后所有者释放。

`qt_composition_transport_tests` 在专用QThread创建文档、申请计划、逐块读取、CPU解释并与现有CPU frame逐字节比较，再在真实256×192窗口显示。夹具128×96，白底、彩色半透明笔触、栅格fill0.7/opacity0.6/偏移(-7,3)、自身蒙版opacity0.45。两后端均3节点/7瓦片，pin458752字节、保守计划计账468480字节，单读峰65536字节；CPU参考最大字节误差0、DPR1.5窗口RGB最大误差0。worker与GUI线程不同，等待期间GUI心跳软件15/D3D11为14；不把心跳次数当延迟或无阻塞的普遍证明。释放后计划/读取当前计账均0。

此窗口显示的是worker CPU解释图，报告显式 `gpu_composition=false`；它验证G1可运行传输与语义，不能冒充G2多通道GPU合成。实际GPU上传缓存、合成依赖/脏区、设备预算和渲染节点关联仍待G2。G0独立三通道实验仍是当前直显证据。

### 12.2 门禁与原始产物

原始目录 `artifacts/general-gpu-composition/G1/`，所有日志和PNG均独立于G0，未重跑无关性能矩阵。`native-final/invocation.json` 固定最终源文件/EXE/DLL SHA256、HEAD及工作区，逐后端environment/transport.json/CPU与窗口PNG/QtTest结果保留；此前 `native-1` 通过记录也保留。

- `render-tests.log`：paint-render完整Release40项通过（含新6项：CPU语义、COW身份、闭包拒绝/预算、LE瓦片/取消、先合成再LOD、冷页构造不读盘）。冷页读通过原PagePool计数证明，不是物理冷盘量测。
- `lease-tests.log`：内部租约/预算/关闭3项Release通过；`public-tests-final.log`：公开ABI6项Release通过，含尺寸/能力/缓冲不部分写、预算/unsupported、新文档/epoch、24笔显示洪泛、8线程读与取消。最终复查将捕获移到本tick元数据发布后，保证key.publication与实际源一致，并新增Qt/公开ABI精确关联断言；未变更原断言。
- `clippy-final.log`：paint-render/paint-ffi all-targets严格Clippy通过；Rust fmt、Python编译与git diff检查通过，原有CRLF提示保留。未运行总 `python tools/check.py`，不能称完整workspace验收。
- `native-final`：Qt传输软件/D3D11各1功能及init/cleanup共3项通过；原移动/变换十行功能各通过（含init/cleanup各12）。D3D11日志实际API4，Qt6.8.3；软件API1。没有新增端到端性能after或物理屏幕计时。
- `consumer-tests-final.log`：四ABI消费者及offscreen软件Qt传输CTest共5项通过；C11/C++20分别静态/动态编译布局断言，并实际申请、读取与释放租约。
- `release-build-final.log`：完整目标构建/部署成功，Rust生成头和既有DXBC检查通过，保留windeployqt缺translations/catalogs.json警告。`release-smoke-final/result.json` / `qt.log` / `preview.png`：标准 `build/bin/Release/drawverse.exe` 已更新，真实D3D11启动/三笔绘画/窗口图/退出0。EXE SHA256 `3731643f59801a965a58fa72800263fef5b87496ea1b5af9087e9bcc3164fd97`；DLL `1b972ed5c5396f7d9911cf5af1cc80f4177334ff73dd394b13069458c04cb982`。中间构建/冒烟目录均保留，不能与最终二进制摘要混用。

### 12.3 保留失败、未测与下一步

`release-build.log` 首次新Qt测试缺include路径失败，补target include后保留；`release-build-2.log` 因ABI注释变更触发生成头漂移失败，Rust重新生成后第三次通过，不绕过检查。`consumer-tests.log` / `consumer-tests-fixed.log` 的受限环境Qt进程约0.09s无输出退出仍保留；补独立CTest的Qt Test DLL路径后受限环境仍失败，最终原生环境相同CTest5项通过，不能将原失败归因断言或删除。历史G0失败与原未归因记录保持。

未测：实际GPU增量/设备epoch绑定、多文档窗口租约消费与显存计账、真实DeviceRemoved/TDR/低显存、IO损坏故障注入、极限4096节点/瓦片压力、Weak索引满额压测、Qt6.5实际编译/其他SDK与平台。支持矩阵之外仍走原CPU路径。G1没有新增性能承诺；首位移性能门槛仍未通过。

下一步G2：在实际画布用所属窗口QRhi消费计划与增量瓦片，保持完整CPU后备；连续绘画及Normal/自身蒙版/隐式白全像素对照，无变化不重复上传，实际GPU结果直显。随后G3扩展编辑源身份/预热和本地最新绝对几何、快释放及最终主视图原子交接，再按G0冻结边界量测。G2–G5尚未完成。

## 13. G2 实施前冻结

起点G1 `b2d51a1`。初期显式环境开关 `DRAWVERSE_GPU_COMPOSITION=1`，默认行为保留至G5。QRhi适配仅在Qt≥6.6构建，当前只接纳Windows/D3D11与RGBA32F采样/渲染能力；其他API、软件、用户关闭以及失败走原CPU。根Normal/自身链接蒙版完整闭包，拒绝G1之外的组合；活动普通笔触需扩展为捕获原CPU可见候选的不可变源，结束仍核心提交单历史。

消费者上限先冻结为64节点、1024块64KiB源（64MiB/包），真实文档面积最多4194304像素、边长≤4096，视口也≤4194304像素/边长4096，LOD≤6；超出显式后备，不用代理缩小原文档。GPU纹理payload上限256MiB（64MiB源atlas、两张原分辨率线性目标最多128MiB、RGBA8视口最多16MiB，加有界查找/轴表），驱动隐藏分配不当作已测。CPU源数据按共享块生命周期进程512MiB上限，包与查找元数据各自有界；窗口缓存不跨设备。正常路径无GPU读回，独立正确性读回与窗口抓图单列。

先在原始分辨率逐层合成，再按当前CPU Renderer的power-of-two block与有效边界面积平均，最后显示转换，不能对各层单独mip。原CPU路径保留供后备及独立对照。新增GPU正确性门槛沿G0：有限值、线性最大绝对误差≤2e-5、RGBA8/真实窗口误差≤1；CPU文档/历史/取消/保存仍严格相等。计时阶段不抓图，G2不提前声明G3首位移门槛通过。

最小门禁：真实Canvas连续24笔及追加第25笔、画笔/橡皮/透明/隐式白/有效自身蒙版/偏移；小夹具全图与CPU、放大和LOD1/2/6及部分边界；无变化重绘上传0、一瓦片变化只上传新内容；不支持/预算/初始化失败后备；生成头/相关核心/ABI/软件与D3D11定向回归，更新标准Release。故障与资源统计、失败及未测随结果追加。

## 14. G2 实际Canvas增量合成（2026-10-27）

### 14.1 实现与资源边界

`CompositionReader` 仅在BackendWorker申请G1租约、按内容ID复用共享块及读取冷页；单轮最多128次新读取或约4ms后让出（单次冷IO耗时不受这个软时间片保证）。GUI最多接纳一个不可变包，节点在所属QQuickWindow的QRhi执行原分辨率线性合成、最终LOD及显示转换，`QSGTexture::rhiTexture`直接显示。正常链路没有GPU读回、PNG或GPU结果QImage；CPU视口目前仍并行准备，供立即后备与独立参考，尚未取消这份CPU工作，不宣称绘画端到端性能已提高。普通活动笔触可捕获不可变CPU候选；GPU不写回、不结束或重复提交笔触。

每个窗口节点一个64MiB atlas，按内容ID只上传新64KiB块；同一包先保护所有需要的槽位，再淘汰不需要的内容。像素内容变化取新旧瓦片在文档空间的并集，清空并依原顺序重合成该区域；层配置/节点结构变化重合成整闭包。无变化重绘没有新合成；仅视口变化保留原分辨率合成并执行显示通道。全分辨率合成后才做有效边界面积平均的power-of-two LOD，轴坐标在CPU用原double采样相位生成有界查找表，避免float UV越过块边界。

第13节单节点当前纹理payload≤256MiB保持。为覆盖QRhi延迟销毁，实施中增加进程2GiB保守预留：每节点按其最大当前payload乘`FramesInFlight+2`预留，增长先申请，失败即CPU；销毁后再保留这些帧，或到场景图失效/窗口销毁释放。D3D11实测FramesInFlight=1；不把该预留称作实际显存占用，也不计驱动隐藏开销。上传共享源按实际frame slot保留，复用该slot才释放；总CPU共享块仍受512MiB计账。64节点/1024源块、2MiB查找/轴/小缓冲预留，所有CPU结构有上限。跨窗口无GPU对象注册复用。窗口epoch改变/初始化、上传、分配或设备身份检查失败停用本epoch通用节点，CPU继续；自动重试/真实设备恢复留G5。

### 14.2 正确性与增量结果

正式产物 `artifacts/general-gpu-composition/G2/native-final-2/`：invocation.json记录HEAD、全部core/ui/tools源摘要和标准EXE/DLL/测试EXE摘要；各后端独立配置/暂存、environment.json、QtTest文本、逐夹具JSON与CPU/GPU真实窗口PNG。软件1进程、D3D11三独立进程均15项通过（13功能行+init/cleanup），Qt6.8.3、DPR1.5、RTX4060 Laptop/API4；软件API1。此处抓图专用于正确性，不是性能采样。

- 130×90全线性像素与真实CPU窗口对照：透明/Normal、白底显式擦除、负偏移、opacity/fill、有效自身蒙版密度，LOD0/1/2/6及部分边界。另1024²密集半透明栅格+自身蒙版、LOD2全线性像素/窗口对照。三进程最大线性绝对误差1.1920928955078125e-7、窗口RGBA最大误差1，未放宽门槛。1024²示例196块/12,845,056上传字节，当前payload103,022,592字节，含在途保守预留309,067,776字节。正确性读回仅每夹具2次。
- 真实Canvas鼠标Press/Move/Release连续24笔再追加第25笔，每笔严格增加一次历史；最后全窗口与CPU误差≤1。正常读回计数0。8次无变化重绘上传/合成增加均0；追加第25笔只上传1块，新脏区4096像素。三轮各29源块/1,900,544字节、25或26次合成（显示可合并，不固定中间帧数），单根Normal为每次清理/Normal/显示三通道；不把通道/节点采用计数当物理屏幕显示。
- 初始化失败、上传失败注入、4096²面积超预算、非Normal混合、固定蒙版五类明确CPU后备；保持原revision/历史/CPU帧，再绘画一次仅增单历史，撤销后CPU帧精确恢复。注入不是真实DeviceRemoved或显存耗尽。
- 三次创建/销毁真实窗口后，进程GPU保守payload预留均归零；测试没有跨窗口GPU共享。多文档/浮窗迁移、高DPI矩阵和真实故障仍留G5。

### 14.3 门禁、失败与未测

`render-tests.log` paint-render Release40通过；`public-tests-final.log`公开composition7项通过，新增活动笔触捕获后继续STROKE_TO/COW、新旧计划区别、旧包不可变、结束单历史；`clippy-final.log`相关all-targets严格Clippy、Rust fmt、生成头通过。`consumers.log`四C/C++静态/动态ABI通过；`qt-consumers.log`最终软件CTest两目标通过。`g1-regressions`原G1传输及移动/变换相关软件/D3D11各十功能行通过（随后最终重链接再验记录见同目录后续批次）。ABI1.81/version1、state45及文档格式保持。未运行总`python tools/check.py`，不称全套通过。

标准`build/bin/Release/drawverse.exe`已构建部署；`release-smoke`显式GPU开关下真实D3D11启动、三笔、截图及退出0。EXE SHA256 `245cd2fd358e95f77b4021f326d2c7af227a5b81ca836a2b7d618554dceb3eee`，DLL `615801d788f2f5ca8cd0133e76f73c99af9888e86e4917894301a913730ce90d`。实际通用GPU采用证据以Canvas测试计数/像素为准，启动环境变量本身不能证明GPU采用。

保留失败：`build-1.log`中Qt `slots`宏冲突已修正；`native-2`的LOD5/预期6、`native-3`的一条视口关联及五条后备QImage尺寸/null失败，均因测试在16ms视口请求稳定前取包/参考。增加精确region/physicalPixels/非空/节点request前提，保留原像素与历史断言，后续通过。`native-3`连续绘画第16笔历史仍15的失败未归因；未改该断言，`native-4`、`native-final-1/2`三独立D3D11均通过，不能声称已修复所有输入竞态。错误Rust测试目标名及错误目录fmt调用日志/工具输出保留，正确目标/目录最终通过。G0/G1既有失败记录保持。

未测：G3首位移/快速释放/最终交接的通用路径、4096²精确GPU合成、其他API/SDK/硬件、HDR/16F、全部湿混/涂抹/选区笔刷矩阵、真实冷盘/损坏页、2GiB预留耗尽压力与真实DeviceRemoved/TDR。当前只支持根Normal完整闭包与链接自身蒙版，组/剪贴/其他混合/固定或祖先蒙版/活动移动变换液化均明确原路径；默认开关仍OFF。下一步G3复用已上传源、确认正确目标后立即按最新绝对几何显示，分别测首位移与稳定拖动，不能用本节正确性结果代替性能门槛。

## 15. G3 实施前冻结

起点G2 `d90eb3e`，保留G0 v4轨迹/三进程/1024²与4096²夹具和首Move、目标确认、准备/上传、内容节点采用、最终主视图交接计时边界；性能运行不抓图、不逐事件落盘，另做正确性截图。不放宽第1节P95两个实测刷新周期目标，分别报告操作开始总耗时、首Move和资源就绪后响应；4096²仍超G2精确预算时明确原路径，不能将其当新GPU通过。

第一纵向项为复用G2已发布源的根Normal移动：BEGIN仍核心验证/拾取目标；确认前只保留最新绝对输入，确认后本地序号独立于核心revision，不等16ms合并或后续Actor更新。整闭包合成不切穿透/剪贴，链接自身蒙版与所属层一起移动。快释放/取消不能等显示准备才提交业务屏障；结束保留源直至同一主Canvas采用对应generation/最终权威revision，概览或任意frameSwapped不作证明。失败/设备禁用随时走原CPU，最终CPU历史/像素必须精确。后续自由变换复用源与本地几何按同样身份/交接门槛补齐；只完成移动不能标记G3完成。

先跑新增Qt及原移动/变换相关、核心/ABI/生成头，再做独立性能。所有原G0失败、G2第16笔未归因失败及未测硬件保留。GUI与渲染线程只处理不可变源及有界几何，不引入冷IO或整图CPU合成。

自由变换首批通用路径限无选区、无自身蒙版的单个根Normal栅格；其他组合继续原CPU/常驻预览。由Rust纯数值入口校验最新输入并生成同源有界Warp网格/逆矩阵，GUI不复制核心曲面算法；只发布CPU几何，不传GPU指针。原始瓦片保留，先变换采样、合成完整闭包，再LOD；六种插值沿核心核函数/钳制，Warp折叠按原三角形顺序覆盖透明像素。初始/最终权威状态与变换中预览分开，全像素/窗口容差沿前节，超范围/无法满足组合显式后备，不把新增网格入口本身当G3完成。

### 15.1 配对事件驱动补充冻结（量测前）

保留全部v4原始运行与未过门槛结论。v4连续轨迹要求8ms，而实际事件间隔约10.9ms，QTRY/qWait的事件泵会限制窗口驱动；另增显式native-v5驱动，用原生QEventLoop及1ms有界谓词定时器等待，轨迹、场景、输入窗口、节点身份/对应帧、像素及历史断言、两个实测刷新周期门槛均不变。新驱动必须在当前同一EXE以开关OFF/ON配对，1024²/4096²、移动/自由变换分别三进程；v5不能覆盖或冒充旧v4结果。计时仍不抓图、不逐事件落盘。4K超过精确合成预算时明确旧路径。两类驱动分别报告，不把事件泵推测作为性能通过证据。

初轮变换 `G3/native-transform-1` 保留：Warp最近邻窗口误差47，其余采样窗口误差≤1；原帧尺寸128×96与稳定视口192×144导致撤销断言失败。补精确region/physicalPixels取帧前提，保留像素完全相同断言；Warp最近邻明确CPU后备，原因是GPU子像素插值与CPU double三角归属的离散采样差异，不宣称该组合迁移完成。`native-transform-2`变换15行通过，移动四行因最终主请求关闭时“视口不可用”被误作故障后备，继续修复；概览不能完成主视图交接的断言保持。

### 15.2 精度补充试验冻结

前述FP32变换结果只覆盖当时夹具，Warp最近邻误差仍作失败保留。继续验证硬边/折叠/全部线性像素；探索同一D3D11窗口设备的double采样与显式三角归属，避免固定功能子像素栅格化改变CPU边界规则。仅当设备报告DoublePrecisionFloatShaderOps及ExtendedDoublesShaderInstructions、实际shader构建和窗口像素通过时启用此变换范围；能力不足继续原CPU显示，G2 Normal不增加double要求。源RGBA32F、全分辨率先合成再LOD、原容差和业务历史均保持。新增几何/缓冲/工作预算，任何拒绝必须CPU后备。此变化后的最终量测重新配对，不把旧EXE数据冒充最终版本。

Qt6.8.3 qsb实测：`G3/fp64-probe.log` 的packDouble2x32→HLSL失败；直接dvec4 uniform的 `fp64-probe-2.log/.qsb` 成功生成SPIR-V100及DXBC50。这只是编译证据，尚不代表设备实际执行或像素通过。4K变换 `v5-transform-4096-off-1/on-1` 曾误并行启动；ON首进程因非计划输入失败，相关批次排除有效性能比较并保留。此前1024配对与4K移动运行独立完成。

### 15.3 最终精度路径与附加Warp性能冻结

正式性能使用完成double采样、显式三角形归属和构建期DXBC后的同一EXE重新OFF/ON配对。除原v5的1024²/4096²移动/普通自由变换各三进程，再增加1024² Warp三进程配对，`--warp --native-driver --operation transform`明确区分。Warp通过真实BEGIN后UPDATE设置，第一阶段不等待首个Warp通道完成，把首次管线/网格准备计入首Move；后续阶段要求最新几何已提交。原轨迹、120个8ms绝对输入、最终CPU像素/历史断言、两个刷新周期门槛不变；不抓图、不逐事件落盘。所有此前二进制的结果仍保留，不能与最终构建混算。

冻结补充预算：每份不可变几何512KiB、进程32MiB；Warp最多145×145点，按核心行列顺序展开最多41,472三角形，顶点缓冲1,990,656字节，裁剪后包围矩形像素工作总和≤33,554,432。超限CPU后备，不静默减少网格。Warp增加原分辨率RGBA32F暂存，仍受单节点256MiB/含在途进程2GiB保守预留；小缓冲/查找表预算由G2的2MiB升为4MiB。分配失败、double能力不足、选择/自身蒙版等未支持变换均显式CPU，正常显示读回计数保持0。

`G3/native-final-1`软件及D3D11三进程各47项通过；57份变换全线性对照最大误差0，尚非最终管线缓存版。`G3/performance-final-1`保留未缓存管线构建的30轮，Warp ON第二进程0xc0000005发生在Qt sendPostedEvents，尚未归因；其余29轮有效。Warp ON另两轮首Move约25–26ms、持续P95约40ms，首帧门槛未过，不用移动/普通变换结果替代。

进一步发现原后端每几何更新创建全套pipeline；改为窗口源准备时创建四个兼容管线（普通线性/显示/精确变换/Warp），保留独立layout-only SRB及兼容render-pass描述，逐帧只建实际资源绑定与有界uniform。增加管线创建计数及几何更新前后不增加的断言；double管线准备失败仅关闭精确变换能力，普通Normal仍可GPU。此优化后的正确性和性能另建批次，不覆盖原结果；缓存本身不能解释或宣称修复Qt访问异常。

`G3/legacy-final-1`完整原Qt输入软件91通过/1失败（tabletContactMoveBelongsToPressedCanvas的相邻画布笔尖可见性），D3D11同断言失败且旧运行器120s期限内未结束；原传输两后端均通过。保留有效断言，增加完整套件的运行时限300s，未缩减场景；须单独复查，不报告完整输入回归通过。错误公开测试目标public_api及无匹配的transform_display_geometry过滤记录保留，正确async_session的composition8项和核心transform的28通过/4原有忽略基准见对应final日志。

## 16. G3 实现检查点（2026-10-27）

按用户要求先提交当前进度。G0–G2已交付，G3限定范围功能已接入、性能验收部分完成；G4/G5未开始，`DRAWVERSE_GPU_COMPOSITION`默认OFF。原三份入口文档的在途改动继续保留，不纳入本检查点提交。ABI1.82、state45，文档格式不因显示改变。

### 16.1 当前实际行为与正确性

根Normal移动及链接自身蒙版复用已有瓦片；核心确认实际目标后，最新绝对输入直接更新本地几何，不等待每次Actor回执或16ms合并。自由变换限无选区/无自身蒙版的单根栅格，原源复用，覆盖六插值、透视、Warp、折叠与局部撤销重做；精度能力不足、工作/资源预算不足及矩阵外组合明确CPU后备。正常GPU显示无读回，最终像素和历史仍由核心CPU提交。快释放、取消与最终主视图采用交接已接通；概览、错误epoch/generation/revision不能结束交接。

最终`artifacts/general-gpu-composition/G3/native-final-3/`固定源码/二进制摘要及环境：软件一进程、D3D11三个独立进程各47项通过（45功能行及init/cleanup）。三轮D3D11共57份变换全线性对照最大误差0，窗口误差≤1，保留原容差；源几何更新上传增量0、管线创建计数不增加。移动七行覆盖拾取确认、链接蒙版、快释放、取消、关闭能力和最终传输失败后备；变换22行覆盖上述精度/输入场景与能力后备。另验证目标回执前无位移的提交/取消、同generation跨客户端epoch、隐藏期间最终帧到达后重新显示。

源准备阶段缓存四种管线，几何更新复用；当前单轮Warp诊断各阶段管线创建4→4，但没有证明其降低首帧延迟。`native-final-2`缓存版三轮D3D11中的两条隐藏交接超时保留：最终CPU帧可在隐藏期间到达，重新显示缺少刷新。现在可见性恢复主动刷新并启动视口请求，测试仅恢复实际可见性，不额外手动刷新或放宽交接断言；`native-final-3`上述四进程通过。该修复不解释另一轮Qt访问异常。

### 16.2 最小门禁与标准运行时

原始日志均位于`artifacts/general-gpu-composition/G3/`：

- `render-tests.log`：paint-render Release40项通过；`core-transform-final.log`：核心transform 28通过、4项原有忽略基准；`public-tests-final.log`：公开composition 8项通过，包含纯几何ABI。
- `consumers-1.log`：C/C++静态/动态四ABI消费者通过；`clippy-1.log`相关核心/render/ffi all-targets严格Clippy、`fmt-final.log`及`header-final.log`通过。没有在后续仅Qt改动后重复无关核心矩阵。
- `related-final-1/results.json`：最终重链接后的G1传输及原移动/变换相关功能，软件/D3D11各10行通过；不能替代第15.3节完整旧输入套件失败结论。
- `build-24.log`：测试目标与标准Release构建部署成功；`shaders-embedded-final.log`：最终标准EXE中五份QSB实际嵌入、DXBC及精确shader的224字节uniform布局检查通过。
- `release-smoke-2/result.json`及`qt.log`：标准`build/bin/Release/drawverse.exe`真实D3D11启动、显式GPU开关、三笔绘画、图像生成与退出0。EXE SHA256 `ff13e99d576d8d14af99fb973d8f8d9b873096aee5a221ba27fffd780f578e88`，DLL `619445cfa8a772f46c4dc5fa6b79373eb882a50ed5c119886ace633cc31c825c`。通用GPU采用证据仍以真实Canvas节点/像素测试为准。

未运行总`python tools/check.py`，不报告完整验证通过。`release-smoke-1`使用PowerShell启动GUI程序时未正确等待，exit为空，不能作为退出验证；后续smoke-2以等待进程结束取得实际退出码。原错误命令、构建中间失败、G0–G2未归因记录均保留。

### 16.3 性能范围、失败与续接

`performance-final-1`是管线缓存前同一版本的30轮OFF/ON配对，29轮有效、一轮Qt访问异常，不能冒充本次最终EXE量测。1024²移动三轮首Move OFF约225–234ms、ON约7–9ms；普通自由变换约17–18ms→3–4ms。该指标是输入至对应节点采用/关联帧，仍须区分命令提交、GPU完成与物理屏幕显示。4096²超出精确合成面积上限，仍原CPU/常驻代理路径，不计作通用GPU性能通过。

Warp ON两次有效首Move约25–26ms、持续P95约40ms，未达到165.019Hz实测两个刷新周期约12.12ms的冻结门槛；ON第二进程0xc0000005位于Qt sendPostedEvents，尚未归因。最终缓存版`warp-cache-1`单进程首Move25.1662ms、持续P95 39.628ms，仅作探索记录，不能宣称缓存解决该问题。当前最终二进制的完整OFF/ON三进程配对尚未补齐。性能计时区间没有抓图或逐事件落盘；夹具准备/计时结束后的正确性图像不能当计时中的物理呈现证据。

继续保留完整旧输入套件的笔尖可见性失败及旧120s运行期限、G2第16笔未归因失败；当前定向通过不能覆盖这些记录。下一步先独立定位Qt访问异常与旧输入失败，再对准确最终版本补齐配对、改善Warp首帧；达到G3冻结门槛后再推进G4完整组/蒙版/剪贴/混合闭包。真实冷盘、实体连续输入/物理显示、其他API/硬件、多文档浮窗/DPI/显存压力及真实DeviceRemoved/TDR恢复仍未验收，G5默认启用门禁未过。

## 17. G3 续接诊断冻结（2026-10-27）

从检查点`e174080`继续，原三份入口文档在途改动保持。先固定旧输入用例中相邻画布B已有鼠标悬停的前置状态，保持原`!b->brushCursorVisible()`与像素/历史断言；原失败继续保留。Warp诊断仅在`--trace`下开启窗口QRhi时间戳，沿既有有界InteractionTrace记录CPU命令准备及最近已完成GPU时间；后者不能强行关联当前geometry，也不是物理屏幕显示。诊断批次不作正式性能通过依据；正式配对仍关闭trace，不改夹具、120个8ms输入、像素/历史及两个刷新周期门槛。

### 17.1 已定位的输入与抓图生命周期问题

`contact-repro-1`原独立用例通过；`contact-repro-2`在相邻画布B先建立真实鼠标悬停后，原实现稳定触发原笔尖可见性断言。现在异画布笔接触清除旧悬停姿态，接触期间拒绝Qt缓存鼠标hover，释放后新的真实鼠标hover仍可恢复。测试保留全部原像素/历史断言，并增加上述前置状态、接触期间缓存hover及释放后恢复；`continue-legacy-1`修复第一部分后两后端各92项通过，最终缓存hover补充及新轮廓行的定向`outline-contact-final-1`各7项通过。后续最终完整套件另列，不覆盖原失败。

`warp-slot-cache-1`再次出现0xc0000005。使用微软签名有效的ProcDump12.02，在`warp-crash-probe-6/dumps/qt_gpu_composition_baseline.exe_261027_193249.dmp`取得完整转储；`analysis.json`记录Qt6Quick.dll偏移0x77bd3、导出邻近QQuickItemGrabResult::render、寄存器0xfeeefeeefeeefeee及反汇编。与固定Qt6.8.3源`qt-grab-source.cpp`对照，原测试轮询`image()`非空后释放抓图对象，而render线程此时仍可在清理和投递完成事件。改为一直持有至GUI线程收到`ready()`；同类变换测试统一等待ready，原像素/撤销/身份断言保持。基准frameSwapped回调显式按值持有observer，避免依赖局部引用寿命。

ProcDump工具、转储和源码只存忽略目录；工具SHA256为`22c2a2b09ade73a342a18dab838354385f73fc98e324b9903fd61fff949c5124`，源码摘要见analysis.json。probe1–5未复现，probe6捕获；调试器下数据全部排除正式性能。该转储证明本次抓图结果寿命错误，不能把此前仅有sendPostedEvents栈的异常逐一认定同一原因。`continue-build-7.log`基准EXE尚未退出导致链接锁失败保留，等待实际进程退出后的build8/9成功。

### 17.2 保持语义的Warp热点优化与探索结果

整型源像素地址改用算术右移得到负坐标也正确的64瓦片索引，避免逐样本double除法；双三次每轴四个double权重只算一次，4×4邻域最多四瓦片槽位复用，保持原f32累加顺序、缺页/隐式白语义。固定循环显式展开，实际DXBC反汇编`warp-current.disasm`的动态循环/索引临时数组在`warp-unroll.disasm`消除；指令槽805→1215，不由静态数量推断性能。

Warp控制轮廓一次快照16点，八条曲线仍各49样本；位于控制行/列的Catmull曲线使用等价单轴计算，其他坐标保留完整二维求值，没有降低网格或轮廓分辨率。新增真实QML轮廓对核心145²网格的两组（含折叠）8×49点对照，坐标容差0.00015仅覆盖f32网格量化，不改变GPU全像素2e-5及窗口≤1门槛。

以下均为探索、各版本独立记录：timestamp-before-1最近已完成整帧GPU时间中位33.406ms，整数寻址/权重复用后的timestamp-after-1约9.699ms；它们开启trace，不能当某次几何的GPU耗时。outline-snapshot-1首位移17.721ms、持续P95 22.034ms；outline-knots-1约10.650/12.995ms。槽缓存单独没有可归因的GPU耗时改善。修正抓图寿命后无trace的warp-response-final-1三轮首位移10.866–11.631ms、持续P95 12.935–13.205ms，仍有超门槛；继续展开循环的warp-unroll-1三轮首位移9.658–10.297ms、就绪8.514–15.726ms、持续P95 12.101–12.127ms，一轮略超Qt元数据参考约12.12ms。所有探索/旧失败保留，不选择最好一轮当正式通过。

这里及第15–16节约12.12ms来自Qt报告165.019Hz，沿用第10节限定，只是参考刷新周期，未测量实体面板刷新周期；原文“实测”不得解读为物理显示测量。最终同版OFF/ON配对及正确性仍须独立完成。

### 17.3 第一份同版续接配对与保留失败

`continue-performance-final-1`完成冻结五场景×OFF/ON×三进程，共30轮，退出/原像素/单历史/身份断言均通过；无trace/调试器，观察溢出与计划外输入为0，计时区间不抓图。基准SHA256为`dee06c1986b25a76915dc44da0a6d0048cec7bf28b06d3705bd104d138d49db3`，对应标准EXE为`0327e06c05db3a9b50e8d2fc09b75950a4a18a8b0a0672bfe0523a502ce7d395`，全部逐轮源码/二进制摘要及原始raw/process/environment保留，不能与随后资源复用版本混算。

1024²三轮首Move：移动OFF202.596–230.858ms/ON2.499–5.689ms，普通自由变换OFF16.010–33.426ms/ON2.915–4.560ms，Warp OFF16.778–18.174ms/ON9.012–9.805ms。Warp就绪ON7.916/9.776/15.909ms，持续三轮P95为12.134/12.305/12.186ms，仍未达到完整门槛，不由首帧改善推断G3完成。4096²仍明确原CPU/常驻代理路径，不算通用精确GPU支持。

该版本`continue-native-final-1`软件一轮和D3D11三轮各47项通过，57份变换全线性误差0、窗口≤1；`continue-shaders-final.log`实际标准EXE五份QSB及double uniform布局通过。原核心/render/公开/四ABI门禁沿用第16.2节，本续接未改Rust、ABI1.82/state45或文档格式。

`continue-legacy-final-1`完整旧输入软件91通过/2失败、D3D11 92通过/1失败：原生笔伴随点击的软件数值未+1/硬件菜单未保持，另软件4K移动待显示证据未匹配。原相邻画布笔尖行及新增轮廓行通过。`continue-legacy-failure-probe-1`独立4K两后端通过，原生点击两后端菜单断言仍失败，保留tablet.log；可见同一时间戳的笔/鼠标位置偏差超过既定取整半径，但尚不能把全部失败归因同一原因。

试验固定回放坐标的`continue-build-10`/`continue-legacy-failure-probe-2`未解决原生点击失败，已撤回，build11重建原输入夹具。probe2软件4K的pending断言实际通过，随后提交历史失败；其有界记录捕获计划外鼠标移动，另D3D11该行通过。两份失败均保留，不删断言、不据独立通过宣称完整旧输入套件通过，也不把桌面干扰推断为所有失败的确定原因。

### 17.4 第二轮资源复用冻结

前述版本仍在每次几何更新重建页表纹理、采样轴、参数缓冲与SRB。只在已记录完整通道所用源revision/generation/设备epoch、节点/瓦片描述、视口/LOD、变换来源边界和Warp拓扑完全相同时，复用这些窗口资源；本地几何序号改变则更新精确uniform/原分辨率网格，元数据重复发布保留缓存。上一帧仅显示通道时不能复用为完整合成，任何依赖或拓扑变化仍完整准备；不增加缓存容积、不改变原预算/释放/故障后备。新增有界累计创建/复用计数，正确性在第二个不同绝对几何上做真实CPU/GPU像素及最终提交对照，同时断言没有新增源上传、pipeline、页表、采样轴、参数缓冲或SRB。新版本另做正确性、旧输入相关复查和同版OFF/ON配对，门槛及范围不变。

`continue-native-final-2`是初版资源缓存，四进程各47项通过，但不含上述仅显示通道保护。补保护和真实窗口resize/普通与Warp拓扑往返后，`continue-native-final-3`原47项仍通过，新增夹具因未先等待新文档完成/创建可绘制普通层而在beginStroke失败；失败批次保留，修正夹具准备后另建批次，全部验收断言保持。新验证脚本同时保存HEAD上的`working-tree.patch`与原有逐文件摘要，便于恢复本次未提交源码，不包括原三份在途入口文档。

`topology-probe-1`还暴露夹具在原笔触历史确认前记录depth、以及128小图resize未改变实际文档视口的问题；改为等原笔触历史+1，在1024²中部绘制并resize部分视口。`topology-probe-2`软件通过，D3D11在CPU帧先到时检查计数过早；加强为精确region/pixels/request/本地序号被节点采用，`topology-probe-3`稳定等待超时，确认实际产品的编辑源包始终保留开始视口。上述失败与有效断言保留。

现于成功主视口请求后，仅重投影编辑base/frame的Qt显示region/pixels/LOD并递增本地几何序号。Rust计划在预算内包含完整可见图层瓦片（paint-render::composition::prepare逐层收集），不是按初始视口截断，故不新增冷IO、源上传或修改不可变租约key；仍用原全分辨率先合成再LOD。投影超4096边/4M输出像素/LOD6显式CPU，隐藏禁用不撤销待交接源，设备epoch仍原身份，不跨窗口沿用GPU对象。`topology-probe-4`两后端各3项通过，原resize/拓扑/像素/取消及历史断言保持。

`continue-native-final-4`新增48项矩阵：D3D11第1/3进程48通过，第2为原transparent-mask-lod1未达到指定zoom（47通过/1失败）；软件原连续绘画第18笔历史17未变（47通过/1失败），新增资源/resize行四进程均通过。原G2第16笔失败继续保留，不能据该相似现象认定已知根因。此次原始last-input记录窗口active=false、drawing=false、无核心错误；原夹具仅show/exposed即向未激活窗口投递输入。后续先显式激活真实窗口、每笔要求Press后client.drawing，并保留原每笔历史+1/25笔/像素/上传断言；缩放也先确认活动窗口并保存失败时的目标/实际zoom。此为加强输入前提及定位，不据此宣称修复核心丢笔；最终重验另列。该批门禁失败，自动串行性能矩阵没有启动，不存在跳过失败后报告全通过。

### 17.5 最终续接回归与独立性能批次

`continue-native-final-5`使用加强活动窗口/真实Press前提的build20：软件48通过，D3D11第1/3进程各48通过，第2进程47通过/1失败，仍为transparent-mask-lod1的原zoom断言。该次失败保存`zoom-transparent-mask-lod1.json`：target=1/3、actual=1、active=false；产品现有WindowDeactivate会停止导航，但没有捕获该次完整事件因果，故不认定GPU或外部干扰为已证实根因，不修改缩放产品行为或断言。连续25笔、全部移动/变换精度、资源复用、resize/拓扑、后备及交接行在四进程均通过；这不等于整个48项矩阵全通过。资源复用后的三轮D3D11变换全线性对照最大误差0、窗口≤1，容差保持。

自动性能链因正确性失败未启动。随后显式运行独立`continue-performance-final-2`，目的为量测当前准确构建，结果不能覆盖该正确性失败或成为完整验收通过声明；沿用冻结场景/输入/门槛，全部原始失败计入批次。`move-1024-on-3`因`ingress.unexpected=9`不满足0而失败，没有形成有效分析行，不能纳入延迟分布；其他结果及最终构建证据在批次完成后记录。

该批30进程已结束，20有效/10失败，无访问异常。九轮直接触发`ingress.unexpected`为1–19的原0断言；另`move-1024-on-1`在ready阶段目标几何等待超时，raw记录16条计划外输入（含type25失活）及取消状态，仍列失败，不仅凭这些事件断言唯一根因。所有失败进程排除延迟分布，不能称完整三进程配对通过。基准SHA256 `85d2051632af3b8b06fb6c18b11b3cd3cae5464cf7be7050228ff69211baa764`，标准EXE `a91121194fb6eed17417cee42df64c6682d4995be844c1b66a51c947e4814226`，各轮source哈希、working-tree.patch、环境和原始记录均保留；`compact.json`标明有效数量，不能与第17.3节旧构建混算。

| 当前1024²场景 | OFF有效进程/首位移ms | ON有效进程/首位移ms | ON持续节点采用P95ms/匹配数 |
| --- | --- | --- | --- |
| 移动 | 2/228.165–245.880 | 1/5.365 | 2.9003/120 |
| 普通自由变换 | 1/20.142 | 3/3.934–5.303 | 5.1115/360 |
| Warp | 2/15.583–17.189 | 1/9.963 | 12.1264/98 |

以上是有效样本分布，因失效进程缺失不能当完整配对结论。Warp ON资源就绪首位移12.819ms，持续P95 12.1264ms；仍不能证明两个Qt参考周期约12.12ms门槛通过，且持续只匹配98/120输入。4096²移动OFF三轮/ON一轮有效、普通变换两侧各三轮有效；移动首位移中位309.385/313.053ms、普通变换17.649/17.373ms，均是原CPU/常驻代理后备，不属于通用精确GPU支持。指标为对应节点采用t6及另存的关联帧t7，非GPU完成或物理屏幕显示。

### 17.6 构建、交付与下一步

`continue-build-19.log`成功构建部署标准Release，生成头检查up to date；`continue-build-20.log`只加强测试夹具，未更改生产代码。`continue-shaders-final-3.log`实际标准EXE五份嵌入QSB/DXBC及double uniform布局通过；`continue-consumers-final.log`四C/C++静态/动态ABI消费者通过。`continue-related-final-2`两后端传输各3项、原路径移动/变换各12项通过。`continue-release-smoke-1`标准EXE真实D3D11、GPU显式开启、绘画生成PNG与退出0，qt.log记录RTX4060 Laptop适配器及窗口QRhi；PNG仅为计时外冒烟。DLL SHA256仍`619445cfa8a772f46c4dc5fa6b79373eb882a50ed5c119886ace633cc31c825c`。三脚本py_compile及diff检查通过。

本续接无Rust/ABI/文档格式改变，ABI1.82/state45保持；核心28通过/4原忽略、render40/公开8及相关Clippy/fmt沿用第16.2节，不冒充新跑。未运行完整`python tools/check.py`，完整旧输入/缩放及无效性能批次的失败全部保留，G3仍部分验收、G4/G5未开始、默认OFF。下一步先取得无计划外输入的准确版本完整三进程配对，定位剩余旧输入/缩放失败并继续Warp门槛，之后才能推进G4组合闭包；真实冷盘、设备故障/恢复、显存压力、多文档浮窗/高DPI与实体/物理显示仍未验收。
