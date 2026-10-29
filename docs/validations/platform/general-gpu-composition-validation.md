# 通用 GPU 合成验收计划与阶段记录

日期：2026-10-28。对应[设计](../../platform/general-gpu-composition-design.md)与[实施提示词](../../planning/general-gpu-composition-prompt.md)。

## 当前状态

G0已交付基线、真实Qt同设备三通道直显试验及选型（第9–10节）；G1已交付不可变计划/有界传输（第11–12节）；G2已接实际Canvas的有限Normal/自身蒙版增量GPU合成及CPU后备（第13–14节）；G3有限范围本地移动/自由变换、交接与资源复用已实现，性能及剩余回归仍部分完成（第15–17节）。G4栅格支持矩阵完成根混合/固定蒙版、隔离/穿透组、有效祖先蒙版、完整剪贴和Dissolve（第18–20节）；置入图、复杂自由变换和超预算整视图CPU。G5交付Windows/D3D11支持范围默认启用、多窗口/DPI/配额与注入故障恢复（第21节）；真实驱动故障及其他硬件仍未测，G3性能门槛保持部分完成。第1–7节为总体验收要求，第8节保留初稿检查历史，不代表运行结果。没有新增实体数位板、物理显示或跨平台通过结论。

| 阶段 | 状态 | 必须补充的证据 |
| --- | --- | --- |
| G0 | 基线/试验/选型完成，性能差距明确 | 第10节；设备真实故障、物理显示和其他API未测 |
| G1 | 计划/传输纵向链完成，默认显示保持 | 第12节；真实GPU资源与窗口epoch绑定仍待G2 |
| G2 | 有限支持范围纵向完成，显式启用 | 第14节；其他API/完整大图/剩余笔刷矩阵与故障恢复仍待 |
| G3 | 有限范围实现完成，性能/完整回归部分完成 | 第15–17节；准确版本配对、保留失败及剩余门槛逐批记录 |
| G4 | 栅格支持矩阵及组合门禁完成 | 第18–20节；置入图/复杂自由变换及超预算CPU，无新性能 |
| G5 | 当前支持范围稳定性与默认接入纵向交付 | 第21节；真实DeviceRemoved/TDR、驱动OOM、跨显示器/其他硬件未测，工程总检及旧失败单列 |

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

## 18. G4 首批门禁冻结（2026-10-27）

用户明确开始G4，当前51a108a/General-purpose-GPU-synthesis，三份原在途入口文档保持。首批限根栅格固定自身蒙版和26种非Dissolve混合（含Normal），不标整个G4完成；组、祖先蒙版、剪贴、Dissolve及置入图整视图CPU，非Normal自由变换也CPU。默认OFF，原预算和G3未过性能/回归记录不变。

实现前冻结：核心计划在固定/链接蒙版、偏移、半透明背景、opacity/fill下逐像素对核心参考，至少26模式全覆盖；旧features=7必须拒绝新组合，不发布部分计划。真实Qt窗口对所有新模式及固定蒙版做全线性误差≤2e-5/显示≤1对照，独立检查先合成后LOD、改变背景/模式/蒙版的脏区及增量源上传、固定蒙版移动/取消/提交单历史和最终主视图交接。矩阵外继续验证明确CPU、绘画及撤销无改变；四ABI/生成头/相关Rust及Qt/Release必过。抓图仅用于正确性，无新性能声明，不重跑无关G3性能矩阵。原始产物存artifacts/general-gpu-composition/G4，逐批保存HEAD/patch/源码与二进制摘要、实际设备和失败。

### 18.1 首批实施与失败保留（补验2026-10-28）

核心增加可协商的两个能力、根节点blend与固定蒙版元数据，Qt原通道直接执行线性混合，无新增表面或读回。移动只平移目标及链接蒙版；固定蒙版保留原坐标。非Normal自由变换主动抑制旧静态合成包，显式显示CPU闭包后备，不改最终像素提交。独立线性校验器只在核心测试模块读取正确性抓图，使用权威blend_pixel计算，不进入应用路径或改变Pixel公共构造API。

`render-1.log`新增核心52场景逐像素通过（26模式×透明/白底有效固定蒙版），总41项；旧features=7拒绝扩展和Dissolve拒绝保持。`header-1.log`错误使用--output参数保留，正确生成见header-2/3。`build-1.log`生成头的1<<32被C编译器视为窄移位/WX失败，改Rust显式64位数值、重新生成头，build2起通过。初次cargo fmt未指定--all无目标，后续fmt-1起为正确workspace格式命令。

`native-1`软件68通过/1失败（剪贴后备启用显示前后revision2→3），D3D11前两轮各67通过/2失败（新增固定蒙版移动两行交接），第3轮65通过/4失败（另mode2透明行opacity请求等待、mode3固定行zoom等待超时）。原始断言、日志与抓图保持。固定移动夹具增加预览抓图时沿用了“关闭GPU”分支，却未恢复GPU就要求保留源直至主视图，因此修正为抓图后恢复开关，原交接/身份/像素/单历史断言不变。剪贴夹具加强为实际clipped元数据已发布，再记录前态，并保存revision/undo/active/drawing诊断；不能把首轮revision失败归因已证实为此问题。

首轮线性后验工具因跨crate调用私有Pixel构造编译失败，三份linear-reference.log保留；将工具移入既有核心单元测试可精确保留原预乘f32，不放宽可见性或经直通颜色往返。`native-1/d3d11-1/linear-reference-repaired.log`独立重验原52份实际抓图，最大误差1.1920929e-7；不能覆盖该进程两条移动失败或把其他进程一起算通过。`transform-fallback-before`未复现可见错误，而在最终撤销QImage128×96/192×144不等失败，说明夹具未等对应DPR主视口；加强初始region/pixels身份后另验，不据此宣称已证明旧GPU遮挡。后备抑制旧包是独立的显式状态屏障。

第二批将LOD正确性相机直接以fitToView布置为原目标zoom/LOD1，保留zoom、计划LOD、全线性及窗口容差；不再把动画导航时序混入像素测试。静态像素/后备夹具不接受鼠标按钮，业务笔触仍用核心真实API执行；原连续绘画和移动输入测试保持。增加已有有界失败状态，不能由这些夹具变更认定旧导航或实体输入已修复。新增同一画布背景/蒙版笔触及模式变更，按原源比较上传增量，验证瓦片脏区与完整线性结果。运行命令为`python tools/validate_general_gpu_g2.py <新目录> --g4`；软件一次、D3D11三次，G4 Qt时限360秒因新增52行而扩展，未移除任何验收行。每个D3D11进程结束后调用核心忽略标记的离线测试并要求56份抓图全部存在；普通测试不需要真实窗口产物，此标记不是免验。

### 18.2 首批最终结果与支持范围（2026-10-28）

`native-2/results.json`：软件一次及D3D11三独立进程均70项通过、0失败/跳过（含init/cleanup）。真实Qt6.8.3窗口选择NVIDIA GeForce RTX4060 Laptop GPU，设备/窗口QRhi证据保留在每轮test.txt。52行覆盖26模式×链接蒙版透明底/固定蒙版白底LOD1，实际96×64全图，不把此范围称1024²复杂栈或性能通过。每轮56份实际RGBA32F抓图经独立Rust核心blend_pixel对照，三轮最大误差均1.1920929e-7，低于冻结2e-5；156份模式窗口对照最大RGBA8误差1。`summary.json`仅汇总原始证据，不取代逐行断言。

| 组合/行为 | 最终结果及边界 |
| --- | --- |
| 根栅格Normal、Darken至Luminosity（ID2–26） | 26模式全图通过；全分辨率混合后LOD1，线性预乘/隐式白/负偏移/opacity/fill保持 |
| 有效链接/固定自身蒙版、密度 | 原坐标反向隐藏量；固定蒙版在目标移动时不跟随；提交/取消、最终主视图交接及撤销原像素通过 |
| 依赖变更 | opacity和混合模式变更源上传增量0；真实背景/蒙版小笔触只上传1–4块（包含活动候选/最终版本），脏区≤4096而非整幅6144像素；四份变更后的全线性结果均通过 |
| 非Normal自由变换 | 明确CPU整闭包，无静态GPU包；实际位移画面与禁用GPU参考一致，单历史提交/撤销精确 |
| Dissolve、隔离/穿透组、祖先蒙版、剪贴、置入图 | 整视图CPU；未迁移，不任意切开依赖。Dissolve/组/剪贴后备实际画笔及撤销通过，能力/初始化/上传/面积预算后备保持 |
| 预算、线程及最终提交 | 沿用G2/G3限额，无新增中间表面/缓存预算，无GUI/渲染线程冷读或CPU整图合成；正常应用路径不读回，读回仅正确性测试观察器。核心最终像素与历史仍由CPU提交 |

`previous-gpu-1`两后端各36项通过，包含原Normal文档/连续25笔增量、精确六插值/Warp/透视、资源复用/resize/拓扑、提前关闭、文档epoch和窗口释放。19份D3D11变换全线性对照最大误差0；这是本次新跑，不能覆盖第17节旧输入/导航失败或变成新性能数据。`related-final-1`传输两后端各3项、原移动/变换相关两后端各12项通过，完整旧输入套件本次未重跑。

### 18.3 ABI、标准运行时与续接

`render-final.log` paint-render Release41项、`public-final.log`公开composition8项通过。`clippy-final-2.log`相关paint-core/render/ffi all-targets严格Clippy、`fmt-final.log`、`header-final.log`通过；`consumers-final.log`四C/C++静态/动态消费者全部通过。ABI1.83、独立composition version1、结构布局不变，旧features7拒绝新组合且不污染文档错误；state45、文档/behavior/VBR不变。GPU默认仍OFF。

`build-6.log`部署显式变换CPU屏障后的生产代码；build7–9补强测试夹具/离线校验并成功构建全部相关目标及drawverse_runtime，最终`build-9.log`成功。`shaders-final.log`检查实际标准EXE五份嵌入QSB、DXBC和double uniform布局。`release-smoke-1/result.json`、qt.log记录标准`build/bin/Release/drawverse.exe`实际D3D11、GPU显式开启、绘画图像生成、退出0；它是冒烟，不代替上述Canvas合成证明。EXE SHA256 `b96501bcb36851689c8419c304d5e72b6eb7bb8a37da36ccec03d981b5053d91`，DLL `0c8b9d68f9bc11ce8d0c3624f983898d52b4037675b9c91c3d485f0364c6602d`。逐批invocation/working-tree.patch保留HEAD51a108a与准确未提交源码/二进制摘要。产物根目录为`artifacts/general-gpu-composition/G4/`；首批失败和原三份在途文档均保留。

本轮没有新增首位移/持续跟随性能量测，不能承诺新模式时延；G3原性能未过门槛、计划外输入、原生点击/zoom和旧未归因失败均不改写。未运行完整`python tools/check.py`，不报告完整验收。G4首批可运行纵向成果已交付，G4整体尚未完成；下一步迁移隔离/穿透组与有效祖先蒙版，再处理完整剪贴闭包和Dissolve坐标/64位种子，对每种组合冻结独立正确性及增量门禁。多文档浮窗/DPI/显存压力、真实DeviceRemoved/TDR故障恢复、冷盘、其他API/硬件、实体输入/物理显示与G5默认启用仍未验收。

## 19. G4组与祖先蒙版门禁冻结（2026-10-28）

89e20f6续接。迁移范围为隔离/穿透组、嵌套边界及有效自身/祖先蒙版；先比较核心权威像素，再在真实窗口全线性误差≤2e-5/显示≤1。冻结透明/白底、两层组混合/嵌套、半透明opacity/fill、祖先灰蒙版链接/固定/隐藏、组内移动与取消/提交/最终交接、背景和蒙版变化的脏区/源增量、隐藏祖先不读源、操作/中间表面预算CPU后备。旧能力请求拒绝整个新闭包，取消/关闭/旧key保持。原G3精确变换/连续绘画/资源释放和四ABI/生成头/相关Rust/Qt/标准Release复查。小图全像素，较大场景不以空白尺寸冒充复杂密集支持；性能另行量测，未测不声明改善。保留第17–18节失败，产物存artifacts/general-gpu-composition/G4/groups-*。

### 19.1 组、祖先蒙版与移动结果

Ctrl+X独立检查点9fc2b02先提交/推送，随后只迁移本节冻结的组范围。`groups-native-2/results.json`：软件及D3D11三进程各37项通过（含init/cleanup），12种嵌套组场景组合内层Normal/PassThrough、外层Normal/Multiply/PassThrough、透明/白底与链接/固定祖先蒙版及LOD1；每场景分别抓初始与参数修改两份线性结果，穿透嵌套固定蒙版场景额外抓背景、蒙版、隐藏、重显。每轮28份真实窗口RGBA32F结果由独立Rust核心混合/Pixel::mix校验，三轮最大误差均1.1920929e-7，窗口显示差≤1；不是独立离屏wgpu或仅PNG。

参数修改上传0；背景/祖先蒙版笔触只上传改变瓦片，数量增量≤4、脏区≤4096；隐藏组后代无源瓦片、无子操作。四个新增组移动用例包含链接组蒙版、固定组蒙版、取消、移动子层而保持祖先固定蒙版；目标确认后最新绝对输入同步改变本地geometrySequence、核心moveSequence不变，瓦片上传/管线创建不增加。CPU预览独立抓图比较，释放保持源直到对应主视图最终revision被采用，错误epoch/generation/revision与概览不交接，撤销原像素精确。包含组的自由变换显式完整CPU闭包通过，未以GPU组通道冒充已实现组变换。

真实2048²组因RGBA32F中间表面超过单节点256MiB而CPU后备，窗口失败计数1、上传/采用0、进程资源预留0，绘画单历史及撤销保持。25笔连续绘画/末笔增量、窗口销毁释放归零通过。保留`groups-native-1`三轮D3D11的同一测试夹具失败：GPU预算后备由Canvas保存，测试误查传输层client原因。生产代码未绕过预算；修正为核对窗口失败计数与明确budget原因，原不丢笔/历史/像素断言保持，`groups-native-2`通过。

`groups-core/render-2.log` paint-render Release42项、`public-1.log`公开composition9项、`consumers-1.log`四C/C++静态/动态消费者通过；新增操作实际读取、布局/元数据、越界、旧能力与取消键均覆盖。`clippy-1.log`相关core/render/ffi all-targets严格Clippy、`header-final.log`、`fmt-final.log`通过。ABI1.85/GROUPS32与独立32字节操作，原结构布局、version1、state45和文档格式保持。当前仍默认OFF，剪贴/Dissolve/置入图整体CPU；没有新增首位移/持续性能量测，G3旧门槛与全部旧失败保持，G4整体尚未完成。

## 20. G4剪贴与Dissolve门禁冻结（2026-10-28）

冻结全栅格剪贴基底/剪贴层各自Normal、非Normal和PassThrough组，嵌套穿透/隔离、基底/剪贴/祖先蒙版与半透明、孤立剪贴与隐藏基底、链接/固定蒙版移动、整链/子层移动、提交取消和主视图最终采用；保持核心逐像素对照，实际窗口线性≤2e-5/显示≤1。Dissolve冻结u32种子0/高位/全1、文档坐标边界、偏移及半透明/蒙版、组与剪贴、先合成后LOD；随机保留/拒绝必须与核心完全一致，不能用颜色容差放过错一个像素。背景/蒙版/剪贴依赖增量、参数上传0、节点/操作/表面预算后备、旧能力与取消键为门禁。产物目录artifacts/general-gpu-composition/G4/closures-*，先冻结再执行；不重跑无关性能矩阵，未量测首位移不宣称新延迟。

### 19.2 最终源码与运行时复验

最后为公开Pixel::mix增加NaN输入保护（有限值公式不变）后，重新构建标准Release与相关Qt/四ABI目标，pixel3/render42/严格Clippy、四ABI通过。`groups-native-final`软件及D3D11各37项，D3D11的28份独立核心线性结果最大1.1920929e-7；`groups-related-final`传输各3/原移动各12通过。`groups-release-smoke-1`标准程序D3D11显式启用合成启动绘画通过，EXE SHA256 `115c127a70e48d541a0c27bd335ad6d41711152947703f58d231bbc3573aeca8`，DLL `d61f3bcaeae4b174d96f7624a0b6994ffac46f47ffa912ca38e9c645e2ce089d`。此前三轮组矩阵与根75项结果保持原版本记录，不冒充最终源码三轮。`groups-core/pixel-1.log`为主动中断编译，未运行失败断言。无新性能量测，G4剩余闭包与G5继续。

### 20.1 实际夹具与边界补充冻结

在首批5个真实窗口探针后，完整门禁固定81个可重开ORA夹具：原74个混合/穿透/祖先蒙版/隐藏基底/种子/float阈值边界，另加6个普通栅格基底（Normal/Multiply/Dissolve）×Dissolve/穿透剪贴以及透明空基底1项。逐一与重新读取文档后的核心全分辨率线性结果比较，再独立比较同窗口CPU显示。另加剪贴基底/子层/整链移动、取消、快释放及Dissolve固定蒙版移动5项；旧上传/历史/最终主视图交接断言保持。操作递归爆炸须在源pin前有界拒绝。置入图仍整视图CPU，原后备笔触必须落在可见无遮挡位置。

`closures-native-1`在运行窗口前因夹具生成失败停止：核心公开API明确拒绝把首层设为孤立剪贴（clipping requires a lower sibling base），没有绕开合法文档校验；该项改为合法透明空基底下的剪贴覆盖，完整夹具仍81项。孤立剪贴拒绝由核心API验证，不能称窗口接受孤立剪贴。

### 20.2 多层剪贴与操作预算补充冻结

追加3个三层剪贴链夹具（基底Normal/Dissolve/PassThrough，各含PassThrough、Dissolve、Hue剪贴及固定/链接蒙版），完整夹具计84项；另在budgets子目录保存16级穿透与剪贴导致操作预算拒绝的合法文档，用原后备绘画/撤销断言检查。81项三轮矩阵完成后只补跑新增3项及原根混合/精确变换的相关shader回归，不把不同版本测试二进制混称一轮。

### 20.3 上游alpha收缩边界补充冻结

Dissolve还必须接收与核心同舍入的上游alpha：新增两层Normal先合成、再以Dissolve组边界处理的夹具（总85），固定一个core两次舍入会保留、f32 mul_add单次舍入会拒绝的像素；两层原图不透明，精确alpha通过ORA opacity往返保存，拒绝PNG alpha量化掩盖边界。shader对影响alpha的源缩放、蒙版、Normal/非Normal alpha与穿透插值禁止收缩/重排。新shader必须重新运行最终软件/三轮D3D11闭包矩阵，旧81项三轮仅保留旧版本证据。

### 20.4 精度反例与最终版本复验

`closures-native-2`为收缩修复前版本：软件及D3D11三进程各112项通过，每轮81份重新加载ORA后的独立核心线性对照最大1.1920928955078125e-7/窗口≤1。`closures-remaining-1`两后端各107项通过，另覆盖三层剪贴、操作预算及原根混合/精确变换；这些记录保留对应版本，不代替最终85夹具矩阵。

`closures-alpha-probe`在真实D3D11窗口复现上游Normal alpha被驱动收缩导致Dissolve保留决定不同，线性误差1（2通过/1失败）。没有扩大容差或移除该像素；使用GLSL precise约束源/蒙版缩放、Normal/非Normal alpha、穿透插值和其三因子权重，保留核心逐次f32舍入。`closures-alpha-fixed`同一反例通过，线性误差5.9604645e-8/窗口1；最终完整矩阵另列。

CTest增加独立CPU夹具setup与必需fixture依赖，在窗口测试之前运行生成器并传入路径；时限300秒覆盖新增85行及原功能行，生成器240秒。该调整是功能行数及串行构建隔离，不改变任何性能门槛；缺失夹具仍失败，不能跳过。正常显示仍不读回，正确性观察器的线性/显示读回单独计数。

### 20.5 最终闭包矩阵与支持范围

`closures-native-final/results.json`及`summary.json`：软件一次、真实D3D11三个独立进程均117项通过、0失败/跳过（含init/cleanup）。每轮85个可编辑ORA由独立paint-io重开后取得核心96×64全图RGBA32F真值，D3D11逐像素最大误差均2.384185791015625e-7，窗口显示差≤1，均保持冻结容差。新增float随机阈值与上游FMA反例的随机保留决定严格匹配。实际Qt6.8.3 / RTX4060 Laptop / D3D11同窗口RGBA32F多通道直显，正常路径readback=0；正确性观察器单独读回，不是性能阶段。

支持27种栅格混合及组PassThrough；普通/隔离/穿透基底、普通/隔离/穿透剪贴、多层剪贴、有效自身/祖先固定或链接蒙版按完整闭包执行。隐藏基底不pin剪贴源；旧能力63整闭包拒绝。参数变更上传0，背景/剪贴子层/祖先蒙版的实际笔触增量≤4瓦片、脏区≤4096，变更和撤销的显示与CPU差≤1。新增5种剪贴/Dissolve移动通过原源复用、最新绝对输入、快释放/取消、单历史及最终对应主视图采用；原概览/旧epoch不能交接断言保持。连续25笔、静止上传0、末笔增量及窗口释放归零通过。

置入图、包含组/非Normal/剪贴等尚不支持的自由变换继续完整CPU；节点/操作/表面/256MiB单节点/2GiB进程含在途预算不提高。真实2048²组/剪贴中间表面及16级穿透intrinsic展开操作预算均明确CPU，绘画单历史和撤销原像素通过，不能把此范围称4K完整复杂栈精确GPU支持。所有最终修改仍由核心CPU提交。

`closures-core/render-final.log` paint-render Release44项、`public-3.log`公开composition10项、`consumers-final.log`四C/C++静态/动态消费者、`clippy-multi.log`相关core/render/ffi/io all-targets严格Clippy、生成头及fmt通过。ABI1.86仅增可协商DISSOLVE64/CLIPPING128及操作语义，version1布局/state45/文档格式保持；普通自由变换224字节uniform不变。新模块composition_closures在pin前和逐次追加操作时有界检查。

本阶段没有新增首位移/稳定拖动性能量测，不改写G3原未过门槛、输入/zoom及未归因记录。保留closures-quick-2置入图笔触落在遮挡区域的夹具失败、closures-native-1非法孤立剪贴夹具失败、closures-alpha-probe真实精度失败，以及各旧构建结果。未运行总check；真实DeviceRemoved/TDR、驱动OOM/冷盘/实体笔/物理显示与其他API未测。G4在上述栅格支持矩阵内完成，默认仍OFF；按用户授权提交推送后直接继续G5。

### 20.6 完整CTest的选层确认前提

`closures-core/ctest-gpu-final.log`与`ctest-gpu-final-2.log`完整offscreen软件测试返回失败但控制台无断言；保留原记录。另以独立文件日志运行`closures-offscreen-probe`，结果211通过/2失败，均为新增clip-chain-cancel/quick在核心选层确认前发起Begin。selectLayers先同步更新本地两选ID，再异步提交Select；原夹具只检查本地size=2。现加强为实际activeLayer=clip且!layerEditBusy并核对两个目标ID后才Begin，原Begin必须成功/最新输入/快释放/取消/像素/单历史/对应主视图交接断言全部保持。没有修改产品输入或核心忙状态；最终完整CTest与定向真实窗口复验另列。CTest同时按现有qt_ui方式保留`${build}/qt-gpu-composition-Release.txt`文件报告，防止Windows控制台静默掩盖失败。

### 20.7 最终工程检查与G4交付

选层前提补强后，`closures-core/ctest-gpu-final-3.log`夹具setup及完整GPU组合CTest均通过，实际offscreen软件报告213通过/0失败/跳过；`closures-selection-ack`真实窗口软件/D3D11各4项通过（两条目标用例+init/cleanup）。产品源码和标准EXE未因夹具修复变化；此前最终85夹具三轮、原根混合/精确变换两后端76项保留准确测试二进制版本，不冒充补强后重新三轮。`closures-precise-regression`的56份根混合独立核心校验最大1.1920929e-7，19份精确变换线性误差0/20份窗口比较≤1。

`closures-core/build-final.log`标准运行时、`build-selection-ack.log`最终测试构建通过；`shaders-final.log`实际标准EXE五份嵌入QSB/DXBC、double布局检查通过；`fmt-final.log`、`header-check-final.log`、Python编译和diff检查通过。`closures-release-smoke-final`标准程序实际D3D11显式启用合成、绘画生成图像及退出0；EXE SHA256 `4d6a05d6add790a61ad542ae76b37a465a58da43556cfe436b0724192fe16513`，DLL `14c877c90f7b9b7b4cb3b95399826d3e389a9ca833f01693df93335027774f12`。正常直显由上述真实Canvas证明，冒烟PNG不代替合成证明。三份原在途入口文档仍各+2行，不纳入本提交。

完成G4栅格支持矩阵后提交推送当前有效分支，按用户指令直接继续G5。默认OFF、不新增性能或真实设备故障通过结论；总check在G5最终门禁运行，已知原layer_batch全删除断言及所有历史失败继续保留。

## 21. G5门禁冻结（2026-10-28，实施前）

起点d6d86b5，当前分支General-purpose-GPU-synthesis，原三份在途文档各+2行保持。先冻结后实现，不扩展G4栅格/精确变换边界、预算或误差：真实窗口显示对CPU差≤1，正常路径读回0，历史/像素/身份采用原严格断言。

- 默认未设置开启支持范围，显式0/无效值CPU；Qt软件CPU，CPU数值计算选择不关闭Qt显示。混合悬停预览必须可见、取消恢复，revision/历史不变，提交后恢复GPU。
- 三个独立文档与真实窗口同时显示；换窗重新上传，独立epoch；换client及同generation旧回调不能污染新文档，迟到失败/成功/交接拒绝。DPR1、2、3独立进程，记录实际DPR，不把单显示器全局缩放称真实跨显示器迁移。
- 2048²组/剪贴单节点预算CPU，减少拓扑或新建小文档后恢复；普通笔触/opacity修改不能每帧重试。最多12个128²真实窗口逐个增加直到实际进程2GiB含在途配额拒绝，关闭一个已成功窗口后由资源释放事件恢复CPU窗口；文档/历史保持，最终计账0。属于payload压力，不是驱动OOM。
- 初始化、上传、record阶段device失效确定性注入；后备期间25个实际接纳的Press/Move/Release笔触，每笔单历史；取消注入后真实隐藏/releaseResources/场景图失效/重显，新epoch由权威CPU文档重建，最终显示与CPU≤1、无重复提交、撤销恢复原像素。中途移动失效的快速提交/取消和对应主视图最终交接另验。
- 先做最小Qt定向验证，再软件/三个D3D11独立进程，DPR2/3定向；原G4闭包、G3精确变换/连续输入、传输/旧CPU常驻路径、四ABI/生成头/shader/标准Release和实际D3D11默认启动复查。GUI串行，不与构建/编译重叠。最后运行python tools/check.py，不跳过失败或将定向通过称完整通过。

产物固定artifacts/general-gpu-composition/G5/<run>，保存HEAD/patch/源码/二进制摘要、环境、实际后端、全部失败和独立正确性抓图。无新性能结论；物理屏幕、实体笔、真实驱动故障、冷盘与其他平台/API未测项保持。

补充冻结：支持GPU计划已就绪而CPU主帧/概览尚未到达时，节点仍须创建并实际采用GPU内容；确定性延迟CPU显示引用验证后，再恢复CPU并比较窗口≤1。此项不宣称已移除后台CPU镜像任务或取得新性能数据。

### 21.1 首轮探针与前提补强

`G5/build-1.log`新测试误写不存在的删除方法导致编译失败，改用现有removeLayer，build2通过。`probe-1`软件及D3D11各9通过/1失败：软件25笔及CPU历史通过，失败为软件后备不发QRhi场景图失效信号；软件没有GPU设备恢复前提，保持隐藏/重显后CPU帧及历史断言，D3D11仍必须实际invalidated及新epoch。D3D11失败为无CPU显示引用行窗口比较197；已有GPU节点采用证据，但首轮helper仅核对revision/request，未核对该请求的region/pixels是否为当前目标视口。补强为和G3/G4相同的精确目标视口/像素身份后再抓图，不放宽≤1。原两份图片与失败保持，后续结果不能倒填为首轮通过。

首轮记录实际DPR=1.5（本机原生缩放），不能把QT_SCALE_FACTOR=1称DPR1。后续用QT_SCREEN_SCALE_FACTORS=1及QT_SCALE_FACTOR=1/2/3布置目标，并要求真实窗口DPR等于目标；另保留本机原生1.5定向复验。没有新增性能量测。当前默认代码已开启支持范围，最终是否交付由后续完整矩阵决定。

`probe-2`两后端各9通过/1失败，仅目标DPR断言失败：QT_SCREEN_SCALE_FACTORS在本机仍得到1.5；无CPU显示等待和恢复前提补强后的其他行均通过，不认定旧197失败为产品像素算法错误。`dpi-probe-1`将原生高DPI自动缩放仅在测试子进程关闭，再以全局因子布置1/2/3，三进程实际窗口DPR断言及默认GPU行通过；此配置不修改用户系统。完整G5追加本机原生缩放定向进程，不用整数DPR覆盖该范围。所有旧失败保留。

### 21.2 首轮完整矩阵与确定视口前提

`native-1`保留完整结果：软件17通过/1失败；D3D11三进程分别16/2、14/4、16/2（通过/失败），DPR2/3/原生1.5各12/1。共同失败为CPU数值设备设置发生在异步设备枚举尚busy时，saveComputeDevice按原契约拒绝；测试补上!computeBusy后再设置，保持必须成功及实际CPU设置断言。D3D11另有撤销后128²与原抓图224²尺寸不一致：初始待适配视口尚可异步改变，测试只在瞬间满足cpuReady便保存原图。窗口已exposed后显式fitToView清除待适配，再actualSize固定相机；最终像素、25笔/逐历史、场景图失效及GPU/CPU≤1断言保持。生产导航/忙状态逻辑没有因测试调整改变。

实际进程配额压力在三个D3D11进程均达到10个窗口、峰值预留1,942,327,296字节，关闭一个成功窗口后失败窗口由释放事件恢复，文档revision/历史保持、正常读回0、最终释放0。此为当时通过的单项，不把首轮整体失败标记通过。后续窗口用例另加强三文档内容确实不同、每个窗口及换窗/换client后的GPU与CPU像素对照；同画布预算恢复也增加窗口像素对照。


`native-2`软件18项全通过，D3D11三轮16/2、15/3、18/0，DPR2/3/原生1.5各13项全通过。仍失败的均是上述故障行原图224²/撤销后128²：cpuReady在等待后备失败回调之前检查，而等待回调期间异步视口仍可完成。再次补强为确认后备建立后重查cpuReady，显式断言zoom=1和frame尺寸=128×实际DPR，再保存原图；保留所有恢复和撤销原断言。此后完整矩阵另列，不能把native-2三轮称全通过。


### 21.3 G5专项最终矩阵

`native-3/results.json`及`summary.json`：软件一次及真实D3D11三个独立进程各18项通过，DPR2、DPR3、本机原生1.5三个独立D3D11进程各13项通过，均0失败/跳过（含init/cleanup）。三整数DPR由子进程关闭原生自动缩放并设置因子，窗口实际断言1/2/3；原生进程实际1.5，不称多显示器实机迁移。源码/二进制/环境快照在invocation与各environment文件，生产源码自build2后未改变，build4/5仅补强测试前提和对照。

默认未设开启、显式0/无效值关闭、软件后备，以及核心数值设备显式cpu仍走窗口GPU均通过。无CPU主帧/概览显示引用时GPU节点实际采用，恢复CPU后窗口差≤1；未删除后台CPU视口镜像，也无新首位移时延结论。混合悬停暂用CPU，取消像素/revision/历史还原，正式提交恢复GPU。三文档颜色和位置不同，三真实窗口分别与CPU比较，换窗后重新上传、独立epoch、换client后旧失败/成功回调拒绝，最终窗口像素差≤1、每文档单笔历史保持，所有窗口销毁计账0。

2048²组和剪贴分别触发原单节点预算，后备普通绘画不重复初始化；去组/去剪贴降低需求后同generation一次重试恢复GPU并与CPU窗口比较，新文档清除失败epoch恢复。第二个大文档重新触发预算也保持CPU，未增大原限额。实际进程压力三轮均在10窗口达到配额，峰值含在途保守预留1,941,516,288 / 1,942,327,296 / 1,941,921,792字节，均≤2GiB；关闭一个已成功窗口后仅一次资源释放重试恢复受阻窗口，revision/历史/CPU帧保持、正常读回0、全部窗口释放计账0。它不证明驱动隐藏显存占用或OOM。

init/upload/device三类确定性故障下，真实窗口25次已接纳Press/Move/Release各产生一条历史；后备只失败一次、不逐笔重试。去注入后D3D11实际hide/releaseResources触发sceneGraphInvalidated、重显新epoch并从核心权威文档重建；恢复前后历史无新增，GPU/CPU窗口差≤1，25次撤销恢复原像素。软件仍验证25笔/隐藏重显/撤销，但没有GPU失效信号前提。移动中途失效的快提交/取消、最后输入与对应主视图CPU采用交接通过，提交单历史、取消零历史。这里的device是注入，不是实际DeviceRemoved/TDR。

保留probe1/2、native1/2以及全部旧失败和原断言。真实设备故障/驱动OOM/冷盘/实体笔/物理显示/其他API与硬件未测；G3原性能门槛仍部分完成，不以本阶段正确性抓图/节点采用替代物理显示或延迟量测。相关完整回归、统一check及最终标准Release证据接续下节。


### 21.4 完整相关回归与新增失败调查

`gpu-regression-1`完整软件213通过；D3D11三进程213/0、212/1、212/1（通过/失败）。后两轮均在clippingDependencies的真实笔触后compare步骤失败，未输出尺寸/差值，不能归因旧zoom或称三轮完整通过。85份独立文档真值行、原根混合/组/精确变换、其他移动/交接/预算/连续绘画行分别保持当轮实际通过记录。未更改产品或容差来处理该失败。

新增仅在该比较失败时保存的有界正确性诊断：节点/CPU/包身份、当前尺寸、实际RGBA8与CPU PNG、比较步骤和原因；原<=1及全部原断言保持。`clipping-diagnostic-1`软件1次及12个独立D3D11进程各3项（目标行+init/cleanup）均通过，无法单项复现，故继续完整套件诊断，不将定向重跑当作完整失败已解决。G5移动故障用例另加本地最后绝对位移(18,9)及提交后核心offset(18,9)/取消(0,0)精确断言；产品源码不变，最终补强复验结果单列。


补充冻结：两个带失败诊断的完整D3D11进程均213项通过，未得到旧失败的原始像素；不能倒推原两轮已归因。代码复查确认既有CompositionReadback只按geometry过滤，普通计划均geometry=0；新节点readVersion初始全1会响应旧version，迟到callback既不校验version也不校验包，可能覆盖下一次请求并混合完成计数。修正范围仅正确性观察者：显式请求绑定generation/epoch/revision/request/lease/publication/view、region/pixels与geometry；同锁更新版本、清空与接纳结果，拒绝旧版本/错误身份/重复通道。不改GPU合成或原误差/历史/像素断言，正常无观察者路径仍不读回。先新增确定性协议回归，再完整实际窗口复验；原两轮失败保持未归因，不以观测缺口猜测统一根因。
### 21.5 最终观察协议与G5矩阵复验

`build-observer.log`及`build-observer-unit.log`完成标准Release及相关Qt/四ABI构建；`observer-probe`软件和D3D11各11项通过。确定性协议行覆盖同包旧版本、错误包/epoch/geometry与重复通道，实际读回`observer-pixel-probe`两后端各3项通过。观察者仅正确性测试创建，默认显示不读回；不以协议缺陷推断21.4原两次失败已经归因。

最终源码的`native-final/results.json`及`summary.json`：软件和D3D11三个独立进程各19项、实际DPR2/3/原生1.5各13项全部通过，0失败/跳过。包含最后绝对位移(18,9)与核心提交offset(18,9)/取消(0,0)补强断言。三轮实际进程配额峰值为1,941,110,784 / 1,941,516,288 / 1,941,110,784字节，各10窗口触发后备、实际释放后仅一次重试恢复，最终计账0。其余专项行为与21.3相同；本节绑定新观察协议源码/二进制，不与旧版本结果合并。

性能工具兼容补充：产品默认策略改变后，measure_general_gpu_g0的原OFF/ON配对现显式设置0/1，防止未设变量的旧CPU参考被默认GPU污染；原生探针的composition_enabled跟随实际默认策略。冻结夹具、输入及既有门槛保持，不重跑无关性能矩阵、不产生新的时延结论。工具环境验证与探针构建结果在最终检查中列出。

`gpu-regression-final`软件213项通过；D3D11首轮出现连续documentPixels行仅保存zoom信息、未产生完整像素结果。观察协议改为显式请求后，这几行旧等待只触发一帧，异步读回缺少后续帧推进。停止该轮及后续无效重复，保存interrupted.json，不能称最终三轮通过。测试等待增加window.update推进帧，仍要求completed精确等于2及原包身份和像素阈值；只改正确性测试，不加显卡阻塞等待或生产读回。补强后先定向覆盖document/root/transform/clip，再完整复验另列。

`build-observer-frames.log`通过（含性能探针）；`observer-frames-probe`软件与D3D11各6项通过，document/root/精确affine变换/clip均实际完成读回、原线性/窗口对照通过，验证帧驱动补强有效。新完整矩阵固定为gpu-regression-final-2，旧停止轮保持。

`benchmark-policy-final`仅用替代子进程捕获启动环境，OFF=0/ON=1两项通过，明确synthetic_process，不执行画布或产生性能样本；初次捕获器把git子进程也误拦截而KeyError，修正捕获器后重验。标准原生探针实际构建通过，产品性能仍没有新结论。

### 21.6 最终完整相关回归

`gpu-regression-final-2/results.json`与summary：软件1次和D3D11三进程各213项全部通过，0失败/跳过，包含G2/G3/G4全套。每硬件进程85份ORA重新打开的独立核心真值闭包、根混合/固定蒙版、嵌套隔离/穿透组、有效祖先、剪贴及Dissolve、原六插值/精确变换/移动/快释放与主视图交接、增量绘画和明确后备均通过；线性最大2.384185791015625e-7，窗口通道差≤1。三个进程各56份根捕获、28份组捕获再由Rust独立参考核验，六条忽略验收函数明确调用通过，原日志位于各root/group-linear-reference.log；根最大1.1920929e-7、组最大5.9604645e-8。旧gpu-regression-1两次clippingDependencies失败仍未归因，不能用本次通过替换或宣布其根因已查清。

`related-final`显式CPU通用显示参考保持，软件/D3D11传输各3项、原移动/常驻路径各12项通过。`ui-default-final`不设GPU变量，实际软件/D3D11混合悬停及真实跨窗口停靠各5项、欢迎文档生命周期和编辑结束焦点8组合各11项通过，无GPU对象跨窗口复用；不是实体鼠标/笔或多显示器迁移性能验收。

### 21.7 工程检查、标准Release与剩余边界

`checks/results.json`保存各命令真实退出码。完整构建build-final/build-observer、补充build-observer-frames及baseline-build通过；标准运行路径build/bin/Release/drawverse.exe已更新。CTest七个目标全部通过：四类C/C++静态/动态ABI、独立夹具生成、软件完整合成213项、G5稳定性19项。实际EXE内嵌shader检查、Rust生成头--check、相关Python语法检查通过，ABI1.86/version1/state45/文档格式保持。首次git diff --check报本记录末尾新增空行，整理最终文档后final-diff-check.log及暂存区git diff --cached --check均退出0；不把初次退出2改写为通过。

确实运行未跳项的python tools/check.py：Python工具6项、全工作区fmt及严格Clippy通过；核心库215通过/0失败/9个原忽略，包含耗时454.61秒的整库测试。随后brush_wet_document的image_texture_scatter_and_dual_paths_share_paged_wet_execution在第86行unwrap失败，ResourceLimit("input segment too large; resample into shorter segments")，该集11通过/1失败，总入口退出101并停止。wet-input-reproduction.log单项原断言复现同样失败，0通过/1失败。core-baseline-identity.json证明整个core（含未跟踪项）干净、等于G4提交d6d86b5的核心树3d933b8c492a7f611135e2eedae92bc57cccebc8，测试及Cargo.lock与提交内容一致；G5无Rust改动。此为本轮完整检查新记录的核心基线失败，根因仍待后续调查，不为本次显示任务改采样上限或测试断言。总入口后续工作区测试、doc/build及verify-ui步骤未到达，不能称完整工程通过。此前layer_batch删除全部层断言失败与89e20f6基线证据仍保留，但本次总入口先在湿画笔行停止，不冒充已再次运行该图层行。

`release-smoke-1`用标准EXE、不设DRAWVERSE_GPU_COMPOSITION，真实Windows窗口Qt6.8.3/D3D11日志明确选用NVIDIA GeForce RTX4060 Laptop GPU；三笔实际绘画后保存窗口PNG并正常退出0，未修改用户配置。其源码/二进制与native-final的标准运行时摘要一致；这是应用后端及绘画冒烟，节点精确采用由专项测试证明，不是物理屏幕或首位移性能量测。

- EXE SHA-256：`9bd49dcec9f61a8ff5dd81545432762a55262047078f9cf43376d8a2bdbac1c8`。
- paint_ffi.dll SHA-256：`14c877c90f7b9b7b4cb3b95399826d3e389a9ca833f01693df93335027774f12`。

本轮已支持：当前栅格/有限精确变换范围默认GPU直显、多文档/真实浮窗及DPR、窗口/文档旧回调拒绝、配额压力释放恢复、确定性故障CPU连续绘画及真实场景图重建。CPU后备：软件/其他API、显式关闭或无效开关、能力/格式/预算不足、未支持闭包/置入图/复杂自由变换，以及设备epoch故障；CPU权威文档、真实采样FIFO及最终提交/历史保持。

未测及未完成：真实DeviceRemoved/TDR/驱动OOM、真实跨显示器DPI迁移、其他Qt SDK/API/硬件/平台、冷盘、实体笔/物理屏幕；后台CPU视口镜像仍并行，按实际后备需求减少其任务尚待优化。G3首位移/完整性能门槛仍部分完成，本阶段无新延迟样本。21.1–21.5失败及中断轮次、G3旧输入/zoom、G4原精度反例和21.4两次未归因比对均保留。下一步针对新核心湿画笔资源上限失败单独定位，再补准确版本G3性能及真实设备/冷盘等证据；不以本机G5专项通过宣称全范围工程或硬件通过。

## 22. G5 按需CPU显示与剩余回归续接（2026-10-28）

冻结：当前起点b8cbe24，沿用分支和三份原有文档差异。像素阈值、输入/历史及G3首有效位移到内容节点采用门槛不变；原始产物使用artifacts/general-gpu-composition/G5-followup，不覆盖G0–G5旧失败。先执行湿画笔/图层最小核心用例，再执行未设CPU参考开关的真实窗口按需显示/故障/导航用例；正确性回归显式请求CPU参考，性能进程清除此开关且不抓图、不逐事件写日志。最终使用原统一check入口，遇到失败如实记录。

本机可执行项包括原输入/Qt界面回归和1024移动/普通变换/Warp OFF/ON配对；4096原图继续受既定GPU范围限制，不能当作精确GPU支持。真实TDR、驱动OOM、跨物理屏幕、真实冷盘、实体笔和其他硬件/SDK仍只能列未测，不用注入或热缓存替代。

性能新增v6仅分离抓图与计时，沿用v5原1024夹具、原生事件驱动、三相输入和精确内容/节点/帧关联断言。固定三独立进程/场景，移动、普通变换、Warp各OFF/ON，奇数轮OFF后ON、偶数轮ON后OFF；清除CPU参考开关，禁用所有PNG抓图与像素比较，逐事件仅有界内存记录。先将OS光标移出测试窗并等待60ms再武装观测；不屏蔽或忽略任何计划外输入，仍使本轮失败。独立窗口正确性矩阵承担像素门禁。v6仅与同版本配对比较，不拼接v4/v5；两个实际刷新周期原目标不放宽，Qt屏幕Hz元数据不能替代物理刷新实测。

### 22.1 实现与严格回归

支持范围GPU计划就绪后，主CPU显示及仅供主画布的自动概览暂停；连续25笔的两份CPU显示身份不推进，GPU逐笔采用最新revision且历史25步。显式导航概览独立恢复，主视图仍暂停；快速释放取色、真实Windows Alt持续保持与I工具按需恢复CPU参考，完成后再次暂停且不新增历史。注入设备故障后第26笔及CPU显示恢复；真实隐藏/场景图重建回到GPU，正常读回0。移动故障提交/取消在未开CPU参考的情况下保持最新绝对输入和主视图交接。最后21项稳定性软件/D3D11通过，增加真实Alt/I后的cpuDisplayOnlyOnDemand专项D3D11通过（含init/cleanup共3项）。

导航旧失败定位为DockWorkspace重复发布相同布局也调用captureLayoutPosition并停止缩放；现在只有画布实际几何变化才中断，原场景原点保持。新增连续24次相同布局捕获不截断3倍缩放及真实resize原点断言通过，原navigatorTracksVisibleDocumentAndSharesContentAcrossWindows两后端各3项通过。标准程序的--preview/--benchmark是明确读取CPU帧的旧QA消费者，现显式保留CPU参考；正常启动不设置。最初release-smoke退出0却未生成图片保留，修复后release-smoke-final三笔、截图和退出0通过。

完整GPU合成矩阵gpu-final-software/gpu-final-d3d11各213项通过；85份闭包使用独立CPU线性真值，全部原像素/窗口阈值保持。root-reference-final的56份RGBA32F捕获最大1.1920929e-7，group-reference-final的28份最大5.9604645e-8。该完整矩阵在导航修改前运行，最终导航及显示需求由随后21项和原导航专项补测，不把不同二进制摘要混为同一次最终矩阵。原CPU/resident完整输入软件98项、D3D11兼容路径98项通过；后者明确冻结GPU=0，因为该套件断言旧resident资产，通用GPU由独立矩阵覆盖。早先未冻结路径的input-full-d3d11多项旧资产断言失败及中断、进程dump保留，不称死锁已定位或原输入失败统一关闭。实际UI混合悬停/浮窗与编辑收尾工作流两后端通过，旧Alt合成按键失败另列。

### 22.2 核心与工程入口的实际结果

湿画笔旧同步段工作量10,061,440超过既有8,000,000上限，先保留ResourceLimit复现，再把矩阵驱动改为产品已有有界分页入口；没有扩大预算。由此暴露的独立双刷图片涂抹分段浮点反馈差异，经共享沿程格点和1/10间距等价计算修复；32组合原逐像素精确相等/历史断言通过，brush_wet_document12项、input_相关24项、新距离时间线1项通过。layer_batch13项按此前empty-layer-stack明确契约测试允许清空全部层的单事务与撤销/重做，原无效ID/重复/空请求原子性保持。

总入口实际运行两次，未跳过项冒充全量通过：full-check先在ABR过时错误码断言退出101；full-check-final工具6/fmt/全工作区严格Clippy、核心库216通过/9原忽略及此前湿画笔/图层行通过，继续到公开会话75通过/2失败退出101（该入口累计749通过/2失败/11忽略，含其他已执行目标）。两份VBR人工夹具分别缺Replay版本37/能力、替换图片副笔尖时漏保留原尺寸规则；修正夹具后async_session77项通过，不改产品校验或有效像素断言。

ABR5001px明确是转换器1..=5000支持范围的Unsupported，增加独立IO拒绝证据并保持公开错误码严格一致；另增1字节资源预算LimitExceeded及输出不写入检查，abr_import10项通过。brush_files错误调用按原契约清零required，旧测试误用清零值分配下一次输出；现保存容量并明确检查错误输出0，原有效读/资源原子性保持。余包workspace-remainder-final使用--no-fail-fast且不重复已执行核心：541通过/1失败/3原忽略；唯一失败是旧组状态测试把现在已支持的22当未知未来版本，补22组织等价并用真正未知46保留拒绝，单独4项通过。后续分段补测不被写成总入口完整通过。

最终全工作区严格Clippy、doc、Release、生成头、四类C/C++静态/动态消费者及EXE内嵌shader检查通过；新增组状态断言格式首次失败保留，fmt-after-fix通过。build-final-code曾因测试进程仍运行导致LNK1104，后续build-delivery-final成功，未把失败构建改写为成功。所有命令、退出码、对应源码/二进制身份和原始数据位于G5-followup。

### 22.3 保留失败与边界

探索性完整UI软件轮发现开发者控制台、最大笔径姿态、旧组目录、笔刷多选、共享颜色、速度控件落点越界、副刷尺寸、笔刷混合、双刷布局等断言失败；ui-final-software保留原始输出，按本轮最小相关回归范围停止广泛UI矩阵，未修改其有效断言。定向UI原导航失败已修复；旧altEyedropperSamplesWithoutHistoryAndBracketKeysResize仅QTest按键后等待40ms，与两次物理键轮询恢复冲突，两后端原失败保留；真实Windows键态及实际取色另有专项通过，不能宣称旧UI全套通过。完整工程入口仍非全绿，后续verify-ui整套未由总入口执行。G3旧原生输入/zoom记录、G4原精度反例、G5两次剪贴依赖比对未归因失败及各中断轮次保持。

真实DeviceRemoved/TDR、驱动OOM、跨物理显示器DPI、真实冷盘、实体笔/物理屏幕、其他API/SDK/硬件仍未测。已实现的配额压力、故障注入与真实场景图重建不替代这些证据；原4M像素/有限精确变换支持边界及不支持的完整CPU闭包保持。ABI1.86/version1/state45及文档格式不变。

### 22.4 v6实际首位移与持续采用（18/18有效，门槛仍部分）

performance-v6/results.json与analysis.json/summary.json保存全部18个独立进程；Qt6.8.3/D3D11/RTX4060 Laptop、DPR1.5，Qt报告165.019Hz。未并行编译/其他测试窗；计划外输入及四类溢出全部0，captures=false、pixel_correctness_checked=false、PNG文件0，正常GPU读回0，ON各阶段主CPU和自动概览均明确suppressed=true。场景为1024²、OS热页与进程首次资源，非真实冷盘。OFF/ON是同版CPU兼容/通用GPU功能对照，不能单独归因于本轮暂停CPU镜像，也不是b8cbe24与本轮的隔离因果对照。

下表单位ms，首次有效Move到对应内容节点采用为P50 / P95 / max；每格n=3，最近秩P95等于最大值。“首次”移动没有等目标确认，变换BEGIN准备则另记在preparation/enter_to_content字段，不能把变换准备时间藏进首帧数字。

| 场景 | 相位 | CPU兼容OFF | 通用GPU ON |
| --- | --- | --- | --- |
| 移动 | 首次 | 211.582 / 237.460 / 237.460 | 9.394 / 97.389 / 97.389 |
| 移动 | 就绪 | 11.766 / 12.173 / 12.173 | 5.634 / 10.135 / 10.135 |
| 普通变换 | 首次 | 17.776 / 18.366 / 18.366 | 4.740 / 5.073 / 5.073 |
| 普通变换 | 就绪 | 17.248 / 18.047 / 18.047 | 4.945 / 5.661 / 5.661 |
| Warp | 首次 | 18.846 / 26.421 / 26.421 | 11.366 / 13.451 / 13.451 |
| Warp | 就绪 | 23.329 / 26.930 / 26.930 | 12.958 / 13.467 / 13.467 |

对应已采用身份的首帧提交ON首次/就绪P95分别为：移动98.477/10.640、普通变换9.443/6.707、Warp18.247/16.789ms。提交通过渲染线程绑定内容采用身份关联，不是任意frameSwapped，更不是GPU完成或物理屏幕显示。

持续相位各120输入×3，只统计真正显示的输入：移动OFF n360、ON n360，P50/P95/max 5.980/6.066/6.295 → 1.105/2.116/4.735ms；普通变换n293→360，18.060/23.931/24.752 → 3.499/4.360/6.423ms；Warp n285→308，22.438/28.159/35.417 → 9.140/13.377/16.076ms。每轮实际输入间隔、计划迟到、准备及release_to_final_node在analysis.json完整保留，显示合并不被写成每条输入均显示；最终主视图交接仍遵守权威状态采用。

移动首轮97.389ms有效长尾保留，其中目标确认92.040ms、确认后采用约5.349ms；不能把它剔除，只报告其余4.922/9.394ms。单独move-diagnostic-trace开启有界内存trace后首次4.956ms/确认3.220ms，未复现92ms，故长尾尚未归因，诊断轮不混进正式三轮分布。Warp就绪首帧12.355–13.467ms、持续P95 13.377ms，仍超过Qt元数据对应两周期约12.12ms；实际物理刷新周期未测。**本轮三进程证据完整，但G3首帧/全部性能门槛仍未完成**，不能用稳定拖动的较小数字替代。下一步聚焦目标确认长尾、Warp准备/采用与最终提交耗时，并另行安排真实设备、冷盘及实体证据。

### 22.5 最终交付身份

build-delivery-final更新标准build/bin/Release/drawverse.exe及paint_ffi.dll后，delivery-stability-software/d3d11在最终二进制各21项通过；abi-delivery四消费者通过。delivery-smoke使用标准EXE、未预设GPU开关，实际Qt6.8.3/D3D11日志选择RTX4060 Laptop，三笔后生成窗口PNG并退出0。此QA入口明确请求CPU参考，不能用其耗时替代默认路径性能；默认暂停CPU镜像由最终稳定性原生窗口用例证明。

- EXE SHA-256：`3c49e7636ed883c4b1a14afee2a8736cc4b52a1460e29ae15a2e545a74561e94`。
- DLL SHA-256：`06c1829e82b9ebc91f7bd746901771b26db9436c870b1887f8cf6360475476f8`，与core/target/release实际生成物一致。
- 原始产物根目录：`artifacts/general-gpu-composition/G5-followup/`；旧阶段产物不覆盖。起点b8cbe24及三份原有文档入口差异保持，沿用General-purpose-GPU-synthesis分支，不修改ABI、state或文档格式来交付显示优化。

## 23. 首次移动开销检查（2026-10-28，冻结条件）

起点82cfee5，标准Release与原v6工具；新产物放`artifacts/general-gpu-composition/first-move-cost/`。先运行三个独立D3D11进程的1024首次/就绪/持续移动诊断，禁止抓图、逐事件磁盘日志及并发构建。保留所有失败与长尾，诊断不覆盖或混入22.4正式分布。Qt/Rust trace各自检查容量与丢弃；命令序号关联，两个时钟不直接相减。已有约12.12ms双名义周期参考门槛保持，节点采用与对应帧提交分开，不声称物理显示。

若实施优化，运行最小对应原生窗口正确性与相同三进程测量；资源优化必须证明上传/分配计数及不支持后备未失效，目标确认仍须核心成功，不以预先显示未确认目标降低数字。原97.389ms和未归因记录保持。

### 23.1 发现与实施范围

`before/run-1..3`均有效，trace丢弃0，无抓图或计划外输入。首次BEGIN核心提交→接受10.1/19.6/16.6µs，执行16.0/21.7/21.3µs；Qt BEGIN→元数据发出9.613/2.662/7.181ms。静态路径确认：命令执行只置dirty，空闲Actor仍需等下一次8ms tick才发布，Qt的1ms异步确认观察无法提前读到该状态。这是确定可移除的周期等待，不能将它单独归因为旧92.040ms的全部来源。

移动BEGIN/PICK（含原resident变体）及多目标BEGIN现在执行后立即复用同一权威发布函数，失败也发布，保留分页可发布守卫。UPDATE、收尾、CPU最终结果和历史协议没有换成GPU结果。新核心用例校验非法ID/部分非法多选拒绝、真实拾取/单目标/多目标成功确认、三次取消无历史及提交一次/撤销恢复。首次新用例误用透明空层导致PICK无命中，`core-move.log`原失败保留；改为显式白底可拾取夹具后原断言保持，`core-move-final.log`通过。

### 23.2 相同诊断开关配对与最终无诊断量测

`before-analysis.json`和`after-analysis.json`各三独立进程，均开启同一Qt/Rust有界trace；Rust采用独立原点，仅按session/sequence关联。首次内容采用中位/最大6.926/9.985→3.411/3.971ms；就绪后首移7.104/7.184→2.397/3.187ms。优化后首次核心提交→权威发布59.5/60.1/58.5µs，执行后→发布16.0/19.8/14.8µs，不再依赖8ms服务tick。此诊断比较不混入22.4原始正式三轮。

最终二进制`final-timing/run-1..3`关闭两个trace，未请求CPU参考、不抓图。三进程全有效，全部输入/采用/帧/状态观测丢弃与计划外输入0，三阶段主CPU及自动概览均暂停。1024/DPR1.5/Qt165.019Hz元数据，非真实冷盘或物理屏幕量测：

| 相位 | 首个有效Move→内容节点采用（逐进程ms） | 对应帧提交（逐进程ms） | 目标确认（逐进程ms） |
| --- | --- | --- | --- |
| 首次 | 2.2642 / 1.4973 / 1.7728 | 2.8581 / 2.6472 / 3.7185 | 1.7113 / 0.0916 / 0.0948 |
| 就绪 | 1.8903 / 1.8398 / 2.6813 | 2.7127 / 2.8480 / 3.9926 | 0.0933 / 0.0971 / 0.1729 |

持续每进程120输入全部匹配，P95分别2.1257/2.1099/2.0755ms，最大2.3925/3.2969/2.3263ms。首次release→最终权威主节点67.282/66.241/67.594ms，收尾CPU路径仍有成本，不与首移混称。原97.389ms有效长尾上述六个配对诊断/三个无诊断进程均未复现，仍未归因，不能声称永久消除或全部G3/G5性能完成。

瓦片上传保持257次/16,842,752字节、testReadbacks=0，各阶段移动无新增源瓦片上传。仍有可评估的资源复用：连续相位页表/轴各18→264，uniform/SRB各24→507；现有只对transform复用，移动每次准备会重建这些对象。诊断首移渲染prepare约0.54–0.85ms（整段CPU准备，并非各资源独立计时），并不是旧92ms确认长尾证据。本轮未扩展这条缓存路径；后续须绑定同源、设备epoch、视口及组/蒙版/剪贴拓扑，不能绕过闭包校验。隐式白、合成后LOD及CPU最终提交不变。

### 23.3 最小验证与交付

- `core-move-final`：Release相关lib4通过/原手工性能1忽略，公开async_session相关8通过；相关严格Clippy、fmt、生成头检查通过。Python工具原6项执行5通过/环境条件1忽略。没有重跑总check或无关性能矩阵，22节已有总检/探索UI失败保持。
- `final-trace-tool-check`独立验证最终测量工具的Rust closed/dropped检查，退出0、Rust/Qt丢弃0；不并入上述配对或正式分布。
- `correctness-move-software/d3d11`：各20通过，覆盖18种Normal/蒙版/组/剪贴/Dissolve/拾取/快释放/取消/后备及严格最终主视图交接；为独立正确性抓图，与性能进程分离。
- `stability-move-*`以及最终二进制`delivery-stability-*`：软件/D3D11各5通过，覆盖移动设备注入故障提交/取消、25笔零CPU镜像及真实取色需求恢复。最终四个C/C++静态/动态ABI消费者通过；ABI1.86/version1/state45/文档不变。
- `build-final`更新标准EXE/DLL，包含嵌入shader与头一致性检查。`delivery-smoke`在未设置GPU覆盖的标准EXE上实际D3D11/RTX4060 Laptop启动，绘画窗口PNG、退出0；QA入口的CPU参考与抓图不混入性能数字。真实设备故障、驱动OOM、冷盘、实体笔/物理屏幕与其他硬件仍未测。
- 标准`build/bin/Release/drawverse.exe` SHA256：`771bdffc53bcb41c9960b21fcdf5e77b47616e0002cc7c4ffe47376a3eacf12c`；DLL `d47d9bf50c8df45b7afff1b45e836c1a34af09ba83cfe0f65986d157aac08bdd`与core/target/release一致。原始产物均在`artifacts/general-gpu-composition/first-move-cost/`，分析和计数在`before-analysis.json`、`after-analysis.json`、`final-timing-analysis.json`、`final-summary.json`。

## 24. 自由变换目标边框与拖动续查（2026-10-28，验收冻结）

从601a14d续接。回归两种不同范围图层连续开启/取消：BEGIN返回时控制点必须为空，确认后只出现目标范围。普通栅格连续绝对拖动须不等Actor回执就更新本地包，并以已有精确像素/取消/主视图交接用例验收。软件与D3D11分开，置入图/复杂闭包CPU后备不得冒充通用GPU支持。原失败保留，性能独立无抓图。

### 24.1 实现与正确性

确认旧状态来自BEGIN先公开active、但控制点仍为闲置默认值或上一次会话值。提交成功时清空控制点/初始边界并通知，核心确认前拒绝UPDATE。目标范围仍由核心发布，不在GUI猜测图层边界。

原始产物继续位于`artifacts/general-gpu-composition/first-move-cost/`。最终`transform-response-software/d3d11`各12项通过，含不同范围图层三次开启/取消、无效中间几何恢复、真实QML关闭及透明/组栈常驻预览。`transform-local-final-d3d11`24项通过，覆盖原六插值、透视/Warp、本地几何、源复用、像素及快释放/最终交接；新增BEGIN控制点为空的断言保持。`transform-fallback-final-software/d3d11`各5项通过，含非Normal/组变换CPU帧及置入图显示后备绘画/撤销；后者不是置入图变换性能证明。

`transform-placed-ui-final-software/d3d11`各3项通过，运行原真实QML置入图控制点拖动、Warp、撤销/重做、数值修改、Ctrl+T/Escape及浮窗快捷键；不与无抓图性能轮合并。最终C11/C++20静态/动态四ABI消费者退出0，头一致性和`git diff --check`通过。

### 24.2 有界响应优化及测量边界

非通用GPU的UPDATE后48ms内，Worker轮询临时2ms，之后恢复原8/16ms。只提前完成包分发，不新增整图任务或扩大近似支持，CPU最终提交和历史不变。普通GPU路径仍由最新绝对输入同步更新本地几何，不进入该加速轮询。

测量沿用v6、1024²、D3D11、实际DPR1.5、native-driver/timing-only，无trace、无抓图；`transform-response-analysis.json`保存一对前后，`transform-final-analysis.json`保存最终GPU一次与旧常驻路径三次。均通过现有输入/窗口/有界观察有效性检查。以下为内容节点采用，帧提交另列，均不是物理显示。

| 路径/轮次 | 首个有效位移→内容采用 ms | 就绪首移 ms | 持续P95 / 最大 ms | 持续匹配输入 |
| --- | --- | --- | --- | --- |
| 旧常驻，优化前单次 | 5.9896 | 16.5959 | 24.0050 / 24.6111 | 100/120 |
| 旧常驻，优化后配对单次 | 20.2093 | 12.1993 | 18.9017 / 36.2579 | 97/120 |
| 旧常驻，最终1 | 17.9712 | 17.6482 | 18.4864 / 23.0260 | 101/120 |
| 旧常驻，最终2 | 12.1190 | 12.0729 | 18.4444 / 18.9986 | 100/120 |
| 旧常驻，最终3 | 17.9199 | 17.8567 | 23.2435 / 23.9651 | 99/120 |
| 通用GPU，最终单次 | 2.9995 | 3.7647 | 4.3037 / 5.2586 | 120/120 |

旧常驻显示会合并中间几何，未匹配输入不等于丢弃真实笔采样/历史。优化后配对首帧/最大值更差，最终三轮仍有波动，不能宣称首帧性能改善或G3完整门槛通过。GPU单次首移对应帧提交7.6964ms，就绪4.9683ms；进入变换→首次内容12.4820ms。旧常驻最终三轮准备212.2720–341.3691ms，首相位进入→内容257.7710–277.8538ms，收尾209.3082–251.2529ms仍有成本。GPU收尾223.8241–353.3708ms也不与首移混称。此场景不是实际复杂蒙版、置入图、实体笔或全部目标的性能验收。

### 24.3 构建与剩余边界

`transform-response-build`构建Release应用/变换测试/合成测试/原生测量入口成功，生成头及嵌入shader检查通过。`transform-delivery-smoke`标准EXE实际D3D11/RTX4060 Laptop、默认GPU请求启动绘画退出0；独立QA抓图不计入性能。EXE SHA256 `de90317a400148e17e2fd3bf26c678ca4ce499a1311c9b1ad25d44eab9a5cb6f`，DLL仍为`d47d9bf50c8df45b7afff1b45e836c1a34af09ba83cfe0f65986d157aac08bdd`。标准`build/bin/Release/drawverse.exe`已更新；ABI1.86/version1/state45/文档格式不变。

保留`transform-target-build-real`链接EXE失败LNK1104记录，随后重试和最终构建成功；最初辅助脚本GBK读取中文失败发生在编辑前，原日志保留。未重跑总check或无关核心矩阵；此前总check、UI、原首次移动长尾等失败保持。复杂目标CPU成本、物理显示/实体手感、其他硬件仍未完成验证，不将本轮分发优化称为所有图像滞后已消除。

额外后台轮询的CPU利用率未独立量测；本轮只记录有界持续时间及既有节点采用计时，不作整机功耗或占用承诺。

## 2026-10-28 缩放闪烁最小回归

新增 zoomKeepsCompositionPresentation：等待 GPU 首次真实采用后，六次往返缩放记录实际呈现身份，断言均有效且为 composition。旧 ready 条件实际捕获 CPU 呈现并失败（build/bugfix-before-gpu.txt）；修复后通过。continuousPaintingAndIncrementalUploads 的25笔及静止增量检查通过。初次回归曾包含启动未就绪帧而失败，保留 build/bugfix-gpu-d3d11.txt，最终增加首次采用守卫。未做实体笔/物理屏幕验收或整套check，不称所有闪烁原因消除。


## 2026-10-28 计算硬件统一开关验收

设置中的计算硬件统一控制液化数值计算、画布通用合成及移动/自由变换/液化常驻预览；CPU亦令画布的专用光标/选区显示采用原后备。诊断环境变量只能进一步禁止合成，显式1不能覆盖CPU。指定计算卡不迁移Qt窗口显卡，界面本身仍可使用GPU，设置提示已说明。

切换时清理移动预热、失效旧请求、刷新绑定画布并恢复CPU视口需求。其他文档沿原共享设置轮询在空闲时跟随；后台发布已接受的设备，不把尚未应用的偏好提前映射成显示策略。笔触/液化/变换/移动期间拒绝切换。

证据在 artifacts/general-gpu-composition/G5-followup/hardware-switch-*：

- 最终软件 defaultPolicy 六行及 hardwareSwitchUpdatesAllCanvases 一行，共7功能行通过；D3D11补原CPU按需/移动故障提交取消，共10功能行通过。含显式GPU环境覆盖下CPU仍关闭、双文档auto→CPU→auto、另一文档液化期间延后采用、CPU移动/变换无常驻预览、资源释放归零、绘画撤销及切回GPU后窗口误差≤1。
- 原计算设备持久化/失败回退及真实设置QML各2功能，两后端均通过；新增提示内容断言。D3D11设置截图 settings.png 已检查，提示完整换行、存储控件可滚动。
- 原套索液化矩阵、蒙版变换边框及连续绘画后编辑两后端各10功能行通过。
- 四ABI通过；标准build/bin/Release应用、相关测试构建及生成头/shader构建检查通过；实际D3D11 Release --preview绘画冒烟退出0并产出图像。EXE SHA256 5597f89eb9f6bcec6ee50d6e53cef4c0f644af36220c91c836d444fb6b3e3b57。

初次新增双文档夹具没有复用应用的共享笔刷目录连接，ready(b)失败；改为复用brushCatalogConnection后原断言通过，原日志保留。编译期间另一个任务同时构建相同目录，数次对象文件Permission denied记录保留；等其他构建结束后串行发起构建通过（first-move-cost/hardware-switch-build-serial.log）。没有运行总check，没有新增性能、实体笔或物理显示结论。ABI/state/文档格式不变，原有未提交修改保留。

## 2026-10-28 BUSY 闪回续修

新增请求/读取 BUSY 两行，验证繁忙350ms保留已显示包，解除后同revision恢复。隔离HEAD源码叠加本次 CompositionPacket.cpp 和 GPU 测试，使用快照既有核心库：旧处理逻辑两行均失败（build/flicker-busy-baseline.txt），修复后通过。新增18次实际窗口像素检查，绘画及往返缩放期间中心红色笔迹保持；原缩放呈现、连续25笔、init/upload真实回退定向复验。初轮像素行被前一测试留下的GPU关闭值阻断，保留 build/flicker-isolated-tests.txt，补独立初始化后通过。

主工作区同期存在未完成 adjustment 核心/UI 修改，正常构建在paint-ops编译失败（build/flicker-repro-build.log）；未修改该功能，也不称完整工作区构建或总check通过。隔离构建显式复用预编译核心，仅验证本次UI显示层。最终证据 build/flicker-fixed-final.txt。未新增实体笔/物理屏幕验收。

最终隔离D3D11共7功能行通过（Qt总数9含初始化/收尾），Release应用已在build/flicker-isolated/bin/Release部署Qt运行依赖。旧逻辑对照还原后曾因Copy-Item保留较旧mtime而误运行旧二进制，失败记录保留build/flicker-stale-binary.txt；显式更新时间并确认CompositionPacket重新编译/重链接后，build/flicker-fixed-final.txt全7行通过。标准Release未覆盖，避免丢失同期未完成改动。


## 2026-10-28：动态预算/紧凑窗口最小验证

最终日志 `artifacts/compact-native-*`：核心paint-render CPU 6项通过（含非整数缩放LOD、4K尺寸合法性及原稀疏路径）；Qt软件调整6行+预算持久化/重启及真实3840×2160输出1项通过；D3D11调整/后备9行通过；D3D11预算后备/视口外像素平移进入/绘画导航/缩放显示共4项通过；快捷键列表1项通过。init/cleanup不计功能行。未跑总check、性能矩阵、实体笔或其他后端。

保留最初非模态测试仍要求modalWindow的6行失败，修改为符合新交互要求的断言后通过；保留首次全区间floor LOD导致超大稀疏测试期望20/实际19的失败，恢复高LOD策略后6项通过。

标准主程序链接遭LNK1104：三个已有DrawVerse进程占用EXE，未关闭用户会话。使用相同构建对象单独链接 `build/bin/CompactReview/drawverse.exe`；最终完整应用D3D11 `--preview` 冒烟成功生成 `artifacts/compact-review-smoke.png`，退出0；调整三窗口截图在 `artifacts/compact-layout`，人工检查色阶/HSL布局无重叠。标准Release的主EXE尚未替换；测试目标为最终源码构建。


## 2026-10-28：最高质量与工作预算修订最小验收

- `native-quality-streaming-final.log`：paint-render CPU 7项通过，其中4096×2160隐式白底以lod0完整输出，原整批瓦片量超过96MiB，实际像素全白；超缓存流式执行。
- `native-quality-storage-test.log`：storage公开测试1项通过，新增32GiB缓存配置创建/文档/释放，非法2TiB仍拒绝。
- `artifacts/native-quality-gpu-verified.txt`：D3D11五功能通过，4096×2160真实QWheelEvent八次输入保持整图尺寸/lod0、上传不增加、曲线仍GPU；64MiB显式预算CPU后备、视口外像素进入、绘画与缩放原回归。
- `native-quality-curves-verified.txt`：密集4096×2160真实曲线12次拖动，无源增量上传/资源重建，确认单历史，提交与预览画面对照，以及原三类后备，共4功能通过。
- `native-quality-settings-final.txt`：软件原调整6行+内存默认/持久化/重启/原尺寸输出1项通过；`native-quality-preferences-verified.txt`：可输入缓存及保存重开2项通过。init/cleanup不计功能。
- 四ABI重新构建及CTest通过；标准Release主EXE/DLL已更新，真实应用D3D11 --preview成功生成 `artifacts/native-quality-release-smoke.png`。未关闭仍运行的旧CompactReview会话。

保留首轮5项因旧DocumentOptions的1GiB上限导致ready失败；配置边界现已同步放宽。保留新曲线夹具缺modes导致拒绝、旧拖动测试释放时移动回原点导致端点恢复1、设置页旧文案断言失败；分别补齐请求、在最终拖动位置释放、按现行文案验证后通过。未跑总check/性能矩阵/实体或真实显存耗尽故障，不把本轮诊断时刻当端到端性能结论。最高质量仍受设计列出的协议/设备支持范围限制。


## 2026-10-28：长图缩略图、隐藏编辑和原生绘画增量修复

- 软件：隐藏图层/隐藏父组两行、renderBudgetSettings 通过（`artifacts/canvas-fixes-hidden-settings.txt`）；按实际文档管理的共享刷库连接、依次初始化两个客户端后，最高画质关闭/开启均跟随，`canvas-fixes-document-settings-verified.txt` 通过。独立刷库同时启动两个同配置客户端的初始夹具 ready 超时保留在 `canvas-fixes-settings-verified.txt`，不冒充通过。
- 软件 UI 两行通过 `canvas-fixes-ui.txt`。1600×63 不透明长图的棋盘容器尺寸按 paintedWidth/Height，clip=true、高度<3；原剪贴/蒙版及浮窗交互断言保持。最高画质关闭→保存→重开为关闭，CPU状态下勾选→保存自动切GPU，重开与新客户端均开启；无QML警告。最终 D3D11 同两行在显式 CPU 参考需求下通过 `canvas-fixes-ui-verified-d3d11.txt`；首次未启用参考需求等待被主动暂停的CPU帧而失败，保留 `canvas-fixes-ui-d3d11.txt`。
- 实际 D3D11：`canvas-fixes-gpu-verified.txt` 五功能行通过，包括4096×2160原生24段绘画+第二处远端笔迹、滚轮和曲线、25笔原视口绘画，以及Normal/固定蒙版Multiply两行（后者含背景/蒙版增量）。原生输出像素与同尺寸CPU整图最大通道差≤1，未修改区域逐像素保持白色；最后一笔显示颜色转换仅4096/8847360像素、轴分配保持2、源仅增量上传一个瓦片，非端到端加速倍数或实体输入结论。原生关闭模式的25笔GPU/CPU窗口差≤1，正常呈现零测试回读。
- 实际 D3D11 隐藏两行及密集4096×2160曲线拖动通过 `canvas-fixes-hidden-curves-d3d11.txt`，12次LUT交互零新增源上传/采样轴/绑定/管线，单历史且预览/确认像素保持。早期将原生纹理与DPR1.5 CPU放大纹理直接比较的连续绘画差异保留在 `canvas-fixes-continuous-diagnostic.txt`；旧视口测试显式关闭最高画质，新增原生测试按同尺寸独立比较，不放宽精度断言。
- 一次计划外完整root矩阵52行均因旧LOD夹具/缺少必须的捕获目录失败，保留 `canvas-fixes-gpu-final.txt`；修正视口夹具并提供目录后仅定向复验上述两行，没有宣称完整矩阵通过。既有未测/失败边界保持。
- `canvas-fixes-verified-build` 构建Qt应用库及三个测试目标成功，生成头检查与内嵌shader检查通过；`git diff --check` 通过。标准Release EXE被用户运行中的进程占用，`canvas-fixes-final-build.log` 中 LNK1104 保留，没有结束用户进程。同一最新对象/库重新链接至 `build/bin/CanvasFixReview/drawverse.exe`，独立部署Qt/DLL，日志 `artifacts/canvas-fixes-review-build.txt`，实际D3D11隔离偏好冒烟截图 `canvas-fixes-review-smoke.png` 通过；标准Release EXE未更新。ABI/文档格式未改，本轮没有全量check、真实显存耗尽或实体笔延迟量测。


## 2026-10-28：最高画质重启误勾选回归

根因：renderSettings 的 highestQuality 原样返回QSettings QVariant，INI跨进程读取显式布尔键时成为QString，QML把非空字符串false赋给checked后判为true。同进程重建客户端仍共享QSettings的布尔类型缓存，上一轮未覆盖真正进程重启。本轮仅在C++/QML设置边界增加toBool，保留首次缺省开启及已保存的开/关意图。

新增三个独立进程写入→退出→新进程读取的回归行：关闭、开启、从未设置。读取进程同时校验实际模式、QML首选项复选框、QVariant布尔类型，以及打开后再次保存没有反转。修复前显式两行失败、缺省通过，保留 `artifacts/quality-restart-before-fix.txt`；修复后三行及原跨文档同步一行全通过，见 `artifacts/quality-restart-after-fix.txt`，日志分别为QVariant(bool,false/true/true)。只运行相关软件测试，没有全量check或新增性能结论。

`quality-restart-fix-build.log` 的Release应用/测试构建和内嵌shader/生成头检查通过。这次标准Release未被占用，`build/bin/Release/drawverse.exe` 已更新，同时重链接/部署上次 `build/bin/CanvasFixReview/drawverse.exe`。标准Release实际D3D11隔离偏好冒烟通过，截图 `artifacts/quality-restart-release-smoke.png`；未修改用户真实偏好文件。ABI/文档格式不变，原失败保留。


## 2026-10-29：GPU下拉框错误显示CPU

用户截图显示下拉CPU但实际NVIDIA Vulkan；只读检查实际偏好computeDevice=auto、highestQuality=false，未修改该文件。QML原indexOfValue只依赖selectedComputeDevice，异步硬件模型完成后没有重新定位，Math.max(0,...)把未找到项显示为首行CPU。改为显式按ID同步并监听model/count，打开时即使草稿ID未变也同步；原草稿保存和最高画质独立默认关闭保持。

按用户此前快速修改/不测试的要求，本轮未运行测试或应用冒烟。`compute-selector-restore-build.log` 的drawverse_runtime构建、QML字节码、生成头及内嵌shader检查成功，标准build/bin/Release已更新。用户仍运行CanvasFixReview进程，未关闭、覆盖该进程或更改真实偏好。没有功能测试通过或全量check结论。


## 2026-10-29：GPU绘画资源复用最小验证

普通绘画每次发布原先销毁并重建页表纹理、uniform和SRB；相同设备/文档/视口/节点属性/操作拓扑现保留资源，重新填写页表并增量上传源。瓦片新增/替换不要求重建绑定；布局、属性、编辑或资源变化仍完整准备。

- artifacts/gpu-paint-reuse-d3d11-final.txt：连续25笔与4096×2160连续24段及远端第二笔两功能通过，新增断言页表/参数缓冲/绑定/管线创建数保持；单瓦片增量、单笔历史与CPU像素差≤1原断言保持。4K最后显示脏区4096/8847360。
- artifacts/gpu-paint-reuse-software.txt：原连续25笔软件回归通过。
- build/gpu-paint-reuse-build.log：Release应用及测试构建成功、标准build/bin/Release更新；build/gpu-paint-reuse-test-build.log为夹具修正后测试重建。git diff --check通过。

首轮4K夹具未显式启用最高画质，随当前默认关闭得到864×456，原失败保留artifacts/gpu-paint-reuse-d3d11.txt；显式开启后原4096×2160断言通过。没有全量check、性能前后量测或实体手绘笔验收，不能宣称所有卡顿消除。未修改用户偏好或关闭现有进程；旧运行进程需使用新Release。ABI/文档格式保持。


## 2026-10-29：用户反馈资源复用后仍卡，持续输入诊断

上一轮资源复用不能视为用户卡顿已解决。新增sustainedPaintingCadence CPU/GPU两行，1024²、160次约8ms间隔输入，整个笔触期间不等待revision追平，采样CPU frameRevision/GPU nodeRevision交付变化；结束后仍要求最终追平与单笔历史。它是交付节奏诊断，不是物理屏幕或端到端延迟测量，测试PASS仅表示诊断执行及最终一致性成功，不代表性能通过。

两独立进程（artifacts/gpu-paint-cadence-before.txt、gpu-paint-cadence-repeat.txt）：CPU更新86/89次，输入1441/1438ms，最大交付间隔27/19ms；GPU更新27/27次，输入1440/1442ms，最大间隔162/127ms。复现GPU持续更新明显稀疏；未做PhotoCraft同条件性能量测。

只读对比本地D:/PhotoCraft/photocraft-main：ui-egui/src/canvas.rs feed_live_stroke批量增量push并记录damage，gpu_canvas.rs refresh_with按damage直接调用composite；gpu/src/lib.rs使用有界1024²累加块、页面驻留与prefix下层检查点。DrawVerse BackendWorker::poll正常16ms，Session actor服务8ms，composition.rs与session.rs在reserve/capture阶段要求revision精确匹配；CompositionReader经独立请求/读取轮询并完整枚举节点/瓦片描述，变化源再经C ABI复制。CompositionNode仍对脏区执行整栈、使用文档大小线性目标而非块/prefix。前三项是持续绘画优先排查方向，块/prefix对大图复杂栈更相关；未对各阶段耗时或BUSY比例量测，不能把某一个差异确认为唯一根因。

本轮仅新增定向诊断，测试目标构建成功（build/gpu-paint-cadence-build.log），未再次修改生产渲染路径或更新应用行为。下一实现应优先建立活动笔触发布/唤醒与不被较新revision饿死的有界快照交接，保持generation/设备epoch/取消守卫及最终历史权威；不能简单关闭守卫或提升轮询频率后宣称完成。


## 2026-10-29：参考PhotoCraft的活动笔触交接实现与最小验收

ABI1.89增加可协商LATEST_SNAPSHOT=256，仅显式请求把revision解释为下界；同generation捕获时将实际revision/publication与像素配对，旧精确请求不变。UI能力查询后启用；完成时记录真实revision。活动笔触采用独立2ms显示邮箱timer，只读session info/composition，停止/禁用/收笔恢复普通节奏，整套后台服务及核心服务周期不加速。来源参考本地PhotoCraft活动笔触damage驱动刷新思想，未移植其代码、整套分块或prefix缓存。

- build/gpu-live-core-test-final.log：租约层4项通过，新增下界/精确语义与返回实际revision/publication、原取消/预算/epoch守卫。首次新测试使用不存在的Brush构造器编译失败保留build/gpu-live-core-test.log，改复用原Brush::default夹具后通过。
- build/gpu-live-public-test.log：公开composition_api 11项通过，新增陈旧精确请求BUSY/最新请求实际revision/未来revision及跨generation拒绝；原能力、裁剪、组、预算与源读取保持。公共request helper显式不选latest，让旧严格回归继续覆盖旧语义。
- artifacts/gpu-live-d3d11-final.txt与gpu-live-d3d11-verified.txt：两次持续160输入，GPU分别93/87次交付、最长间隔27/27ms，CPU87/88次、28/27ms。前一版基线GPU27/27次、162/127ms。约1.44秒，观测节点提交/CPU交付revision，不是物理屏幕或实体笔端到端延迟，也没有PhotoCraft性能配对或各改动单独消融归因。
- 同两日志的连续25笔和4K连续24段/远端笔触通过，单历史、CPU像素差≤1、源增量/零新增资源与4K显示4096/8847360像素保持。artifacts/gpu-live-software.txt原连续25笔通过。
- BUSY旧测试用pointer判同包，与默认关闭最高画质时既有本地视口重投影冲突；首次两日志失败保留。尝试显式最高画质后zoom本来就不触发请求，恢复等待失败也保留在verified日志。最终显式viewport模式，忙期间要求原source request/revision及完整data指针列表保持、无CPU fallback；恢复后要求新request_id而非单纯新pointer，增强实际传输恢复判据。artifacts/gpu-live-busy-verified.txt两行通过。
- build/gpu-live-abi.txt：四C/C++静态/动态ABI通过；生成头由paint-api-gen生成，构建时check通过；cargo fmt --all --check、git diff --check通过。没有总check、无关矩阵或新增实体性能验收。

标准EXE初次链接被用户进程占用，LNK1104保留build/gpu-live-build.log；未关闭用户进程。初次在目标尚未重链接时误运行旧Qt测试，日志gpu-live-d3d11.txt保留，不纳入after数据；单独重建测试确认后取得上述final/verified结果。先以相同对象链接部署build/bin/GpuBrushReview/drawverse.exe，实际冒烟生成gpu-live-review-smoke.png；随后标准EXE可写，build/gpu-live-release-final-build.log成功更新标准build/bin/Release，Start-Process等待实际退出0并产出artifacts/gpu-live-release-smoke.png。测试构建记录gpu-live-test-build/gpu-live-test-final-build/gpu-live-busy-fixture-build.log。state45/文档及version1布局保持，原未测边界不变。


## 2026-10-29：变换与显隐响应修订（验收条件）

保留修复前二进制、工作区补丁及原生无抓图变换/Warp基线。新增Warp网格逐顶点bit精确map对照，涵盖密集/折叠/负坐标；原变换像素/历史/取消/六插值保持。显隐新增同层反复切换、局部damage、页表零新增和CPU真实窗口对照，软件后备保持；组/蒙版原矩阵另查。最终Release定向回归及原生性能配对，失败轮保留，不将节点采用时间写作物理显示延迟；未完成项如实记录。


## 2026-10-29：用户实际密集4K图片与换色首笔收尾

用户图片通过DRAWVERSE_DENSE_PAINT_IMAGE只读传入（4096×2160、约13MiB压缩PNG），无修改原图。它包含2176源瓦片，超过CompositionReader旧2048身份整表清空阈值。每次替换包重复读完整图片；空白4K没有这些源瓦片，前轮测试未覆盖。最近颜色在像素提交回执后调用useColor，旧queuePersistence清空整个设置/catalog对象；共享快速投影未放行recentColors，导致该更改走完整恢复。

修复：缓存完成时只保留当前包的≤4096 Weak身份，读取期间前后包查找表硬界8192、请求开始清过期身份；达到硬界仅淘汰单项，不清整表，源像素分配/预算/取消不变。最近颜色只补丁已有snapshot.recentColors并排队持久化，不失效catalog/预设数组；仅已验证同进程source沿sharedParameters投影recentColors、发一次通知，外部JSON仍完整验证。新增transportReadTiles是Qt包诊断字段，不改变C ABI或持久化格式。

- artifacts/dense-paint-image-before.txt：修复前实际图片小点笔读取2176瓦片，等待提交/显示409ms，新增≤4瓦片断言失败。该耗时受QTRY轮询与同期编译负载影响，不是精确输入到屏幕延迟。
- artifacts/dense-paint-colors-before.txt：最近颜色导致snapshotObject.rebuild=1，新增零重建断言失败。原失败日志保留。
- artifacts/dense-paint-image-final.txt：三笔（红/红/蓝）各只读1瓦片、源增量≤4，所有未变content_id的数据指针严格复用，单笔历史/最终节点revision追平/无错误。QTRY观察三笔52/51/51ms不作精确延迟或倍数结论。同图96段连续输入期间50次更新，约915ms含末笔等待、最大采样间隔35ms；非实体/屏幕或完整性能门槛。
- artifacts/dense-paint-colors-final.txt：5功能行通过，新回归含三隔离模式最近颜色零重建/可信共享零presetsChanged及单通知/快照冷恢复精确/外部非法颜色拒绝；原三标量颜色投影、立即真实像素保存重开、双文档两模式及单写入者保持。artifacts/dense-paint-colors-software.txt同最近颜色与真实像素保存重开两功能通过。
- artifacts/dense-paint-original-continuous-final.txt：原25笔真实窗口像素/单瓦片/历史回归通过。初次与另一任务窗口基准同时运行，window.isActive断言失败保留dense-paint-image-after.txt；未修改该断言，独立进程复验通过。dense-paint-image-after另记录首轮修复三笔读取均1。
- build/dense-paint-baseline-build.log、dense-paint-fixed-build.log、dense-paint-final-test-build.log构建成功；原工作区另一任务同时构建核心/Qt目标，出现Cargo锁等待，未停止其他进程，资源计数断言与定向功能结论保持，不以受干扰墙钟作精确性能归因。生成头/内嵌shader检查和git diff --check通过。
- 标准build/bin/Release已更新，Start-Process等待实际D3D11应用--preview退出0，生成artifacts/dense-paint-release-smoke.png。未改变用户真实偏好、未关闭用户会话。未跑总check/无关矩阵，没有任意大图、冷盘或实体笔全面流畅结论。原transform_mesh.rs及其他任务修改保留；ABI1.89/state45/文档保持。


### 2026-10-29：变换/显隐响应修订结果与未解决性能边界

本次改动在已有ABI1.89工作区上进行，保留其他任务的绘画/最近颜色改动。仅优化共享mesh计算、变换通知、Properties权威发布/短轮询和普通节点页表/显隐damage，不扩大GPU变换支持条件、不改网格密度/插值/最终历史。页表复用限于普通无编辑节点；普通/Warp切换继续按原断言重建。显隐奇偶交接有整目标GPU复制，因此局部damage统计只代表合成/颜色转换面积，不能当总GPU访问像素。

证据根目录 `artifacts/transform-visibility-response/`：

- `core-final.log`：核心transform 25通过/3原忽略；`transform-core-final.log`：原公开变换7通过。新增96/1024/4096尺寸、负坐标、普通/透视/Warp/折叠的全部网格顶点与未修改map函数逐f32位精确一致。
- `verified-software`：原GPU套件所选30功能行、原实际QML变换13功能行通过（含初始化/清理分别32、15）。`verified-d3d11`原变换/后备/六插值/透视/折叠/资源/取消/提交等28功能及实际QML13功能通过；新增显隐两行的窗口截图在DPR1.5、logical zoom1时失败，不计通过。
- `visibility-final-software/d3d11`：最终稀疏/密集1024层各12次开关及原拓扑往返3功能行均通过。三层夹具含远处上层蓝色内容，验证可见/隐藏两个状态的原尺寸1024²GPU全像素与核心误差≤2e-5；窗口按原同物理像素密度设置zoom=1/DPR，误差≤1，页表零新增，稀疏合成damage≤4096。不得将含抓图/全像素读取和QTRY的observation_ms作性能数据。
- `first-tests`原稀疏damage断言失败保留：观察节点采用早于对应record提交，改为等待渲染统计revision后原面积阈值通过。`final-d3d11`原拓扑重建断言失败保留，收窄页表复用到非编辑普通包后原断言通过。`verified-d3d11`与`pixel-diagnostic`的DPR1.5截图失败及图像保留；GPU原生纹理线性放大与CPU视口采样不同，匹配物理密度并新增原图全线性校验，未放宽像素阈值。
- Release最终构建见 `build/interaction-final2-build.log`，测试最终构建见 `build/visibility-final-build.log`；标准 `build/bin/Release` 已更新。`release-smoke`实际应用D3D11 --preview退出0并生成PNG。中间LNK1104保留于interaction-final-build.log，重试成功，未关闭用户进程。
- `abi-direct`四个C11/C++20静态/动态消费者退出0；头及嵌入shader构建检查通过；核心严格Clippy、fmt、diff检查通过。此配置ctest -R abi报告No tests were found，不计CTest通过，改为直接执行四个真实消费者。初次Cargo过滤命令错误和一次全集编译中断不计测试通过，最终核心定向日志为准。未运行总check。

原生性能继续使用既有v6无trace/无抓图/每轮120个8ms绝对输入，修复前与最终各三独立进程，两场景12进程均有效。旧EXE保留于before；before-three工具采集的源码清单属于测量时工作区，不能当旧EXE源码，其身份以保留二进制和起始before.patch为准。中间after-three也全部保留，不挑最好轮次。

| 1024²节点采用时间ms | 修复前3轮 | 最终3轮 |
| --- | --- | --- |
| 普通首次 | 4.444 / 4.473 / 2.951 | 3.899 / 4.376 / 2.623 |
| 普通就绪首移 | 4.099 / 4.560 / 2.735 | 4.984 / 4.180 / 5.132 |
| 普通持续P95 | 4.093 / 4.524 / 4.182 | 4.165 / 4.526 / 6.416 |
| Warp首次 | 8.894 / 8.403 / 9.863 | 8.515 / 10.897 / 7.747 |
| Warp就绪首移 | 10.761 / 7.796 / 14.555 | 18.050 / 16.223 / 9.998 |
| Warp持续P95 | 13.104 / 12.244 / 12.955 | 13.548 / 12.576 / 12.377 |

普通连续360输入均匹配；Warp修复前311/360、最终310/360实际采用，合并的输入不算逐条显示。数据见analysis-all.json/final-*-three各轮raw.json及关联帧。**本次没有稳定端到端性能改善证据，Warp首帧/长尾和全部跟手问题未解决，不得宣称这几个卡顿已全部修复。** 结构性重复计算/全局通知/页表重建已减少，显隐局部合成正确性已闭合，但显隐无精确前后延迟配对；下一步仍需对GPU精确Warp采样、帧排队和复杂CPU后备分段归因。上述均非GPU完成/物理屏幕/实体笔延迟，未新增真实冷盘或任意文档全面流畅结论。


## 2026-10-29：用户实际4096×2160图变换诊断

原图为用户提供的“2024年4月21日14 (赛博朋克)_无水印.png”，只读打开，原图SHA256在各invocation.json。新增measure_general_gpu_g0.py --image（仅timing-only）及--api-driver：禁用交互面板，直接GUI几何API提交，排除鼠标命中/QML事件耗时。实际图Warp拖控制点0，旧基准Warp开启后整图平移不等同真实网格变形。原有合成夹具和有效性规则保持。

证据artifacts/actual-image-{transform,warp}-api-before：各一独立D3D11进程通过，原图经ORA往返清历史。普通持续120输入匹配115，节点采用P95 30.4332ms；Warp匹配103，P95 60.5674ms、最大251.0792ms，对应帧提交P95 63.4968ms。Warp三阶段确认至最终节点交接约2974–3232ms。输入API调用P95分别0.8672/0.902ms；这些不是实体输入/物理屏幕延迟，不是PhotoCraft配对性能结论，也不是修复后成绩。

独立trace进程actual-image-warp-api-trace通过，零trace丢弃：连续阶段96次composition.record，CPU准备P95 2.085ms、记录P95 2.575ms；QRhi前一完成帧GPU时长中位58.949/P95 63.973/最大141.250ms。该值含窗口GPU帧，非独立shader计时；支持主要瓶颈在GPU帧，不能把每毫秒归因给单一pass。代码每次几何更新全4096×2160线性RGBA32F精确采样，Warp额外scratch/over，双精度重心及最多16次源采样后再LOD。参考本地PhotoCraft transform_tex.rs按屏幕密度选择会话mip、warp_preview.rs按屏幕密度细分，解释架构差异但未量测其帧时。

原生鼠标驱动三次失败保留actual-image-transform-before、actual-image-warp-before、actual-image-warp-before-repeat，均有额外鼠标事件，不计有效性能。API诊断不替代真实QML输入验收。仅验证打开后的普通栅格；置入图片路径、显隐本轮未测。当前新增诊断，尚无新的生产修复；下一步应分离交互预览采样与最终原图物化，验证缩放细节/折叠遮挡/提交取消及最高画质语义后再发布，不能以缩短轮询代替GPU工作量优化。


## 2026-10-29：密集绘画预算与分批上传

用户只读原图“2025年9月13日-漠·寒.png”为4096×1782，SHA256 `9778D0A32D7FC5F039B638D854E07A47718D1B7198487AA5318C690631D4AC19`。测试由 `DRAWVERSE_BUDGET_PAINT_IMAGE` 指定，不修改原图或用户设置。新增 `paintingImageBudgetRemainsOnGpu` 将CPU瓦片缓存显式设64 MiB；修复前普通画质首次显示真实失败 `CPU: composition transport 5`，见 `artifacts/image-budget-before.txt`。这复现了提前回退的一条路径，不证明用户原会话采用64 MiB。代码另有4096瓦片/大图256 MiB/小图64 MiB及全进程512 MiB源门槛。

最终 `artifacts/image-budget-final.txt`：原图普通画质三个独立密集层5376瓦片、GPU计账625597392字节；最高画质五层8960瓦片、887799808字节（实际覆盖扩列atlas）。两行均通过原图显示、逐层密集填充、三笔单历史、缩放、曲线GPU预览、零render失败及窗口中心填充颜色通道差≤1；六笔每笔只读取一个变化瓦片。初次修复普通画质在导航后源增量断言失败，保留 `image-budget-after.txt`；最终同设备/文档导航保留Weak索引后原断言通过。源上传按8 MiB/批提交，非跨帧无限显存或超显存分块合成结论。

同一最终D3D11进程显式64 MiB显存：128²仍GPU、2048²真实预算不足CPU两行通过；原documentPixels四行、精确affine-2/Warp-0、窗口释放归零通过。其余两行 `white-mask-lod2`、`partial-edge-lod6` 仍以实际LOD1/5对预期2/6失败；取整视口/floor LOD逻辑本次未改，旧夹具失败保留，不能称整个入口全绿。独立 `image-budget-regression.txt` 原25笔/撤销/源增量及注入上传失败两功能通过；软件 `image-budget-software.txt` 原25笔与透明像素两功能通过。

核心paint-render composition共11项（含新增6000引用、调用方瓦片/pin/元数据下界拒绝），公开composition_api共11项、四ABI均通过；相关两crate全target严格Clippy、fmt/生成头、嵌入shader DXBC检查、Release构建及实际标准应用D3D11 --preview退出0通过。日志 `artifacts/general-gpu-composition/first-move-cost/image-budget-{core,ffi,clippy,build-final,build-chunks,navigation-build}.*`，冒烟 `artifacts/image-budget-smoke.{txt,png}`。初次构建SDK的fxc未进PATH及随后生成头漂移失败保留，补齐本地SDK环境并由生成器更新头后通过；无源文件硬编码SDK路径。标准 `build/bin/Release` 已更新，ABI1.89/state45/文档格式保持；未跑总check、前后性能配对或实体笔，不承诺所有文档永不CPU。同期既有变换诊断文件与记录保留。


## 2026-10-29：实际4K变换颜色核优化验收

生产变更仅composition_sampling.glsl：颜色核权重多项式与权重乘法改float；几何坐标、透视逆矩阵、Warp重心/边界覆盖仍double，原始像素地址和16样本累加顺序保留。最终CPU原图物化/六种插值和历史未改；无ABI/state/文档格式变更，无代理分辨率下降或mip实现。预览不是逐位相同，按既有线性2e-5/窗口每通道1容差验收。

D3D11原变换/六插值/透视/折叠/局部撤销/取消/快释放/资源复用/目标未确认关闭27项通过（含init/cleanup），artifacts/transform-float-kernel-tests；最终同组再次27项通过transform-kernel-final-tests。最终应用联合构建因运行中标准EXE LNK1104停止，新增测试尚未编入时第二组27项日志保留；随后独立构建tests成功，新增1024²三种核Warp回归5项（含init/cleanup）通过transform-kernel-dense-tests。所有相关线性误差最大4.231929779052734e-6，未放宽阈值。没有跑全量check或新实体/物理屏幕测试。

原图GUI API诊断：普通修改前单进程持续P95 30.4332ms，修改后12.1461ms（115/120与116/120匹配）；Warp修改前60.5674ms，修改后三进程30.2217/31.0733/30.2230ms，匹配97/108/95个输入，均120输入。证据actual-image-transform-float、actual-image-warp-float-kernel、actual-image-warp-float-repeat及前节基线；无trace/抓图。它们是节点采用延迟，排除鼠标命中链路，不是实体/屏幕或PhotoCraft配对；修改前三轮同条件基线未补齐。首移仍有波动，不声称稳定改善，确认后仍秒级，未独立归因其变化。

标准build/bin/Release/drawverse.exe被用户进程4512占用，未停止或替换。复用实际已编译对象与同一链接命令，仅替换EXE/PDB输出至build/bin/TransformReview，生成独立drawverse.exe；Qt运行依赖复制自标准包。独立应用--profile-dir隔离--preview冒烟退出0并生成真实PNG，日志transform-kernel-smoke；link日志build/transform-review-link.log。源/二进制摘要artifacts/transform-kernel-manifest.json。标准Release仍旧EXE，不能称已更新标准包。

## 2026-10-29：PhotoCraft路线大图网格预览验收

实现及边界见general-gpu-composition-design同日“大图交互网格”节。普通画质的大目标优先复用三图静态源、Qt mip和纯几何本地刷新；最高画质保留原尺寸精确合成，最终原图六插值/历史未改。预览代理最长边2048、RGBA8编码颜色过滤，非最终逐像素等价；没有完整移植PhotoCraft自适应细分或无限原图纹理。其他栈仍由原严格分割/后备规则决定。

验证记录：
- build/transform-mesh-d3d11-final.txt初轮9项通过。最终加入最高画质守卫后，transform-mesh-d3d11-verified.txt为35通过/1失败，失败为原小图compositionTransformReusesSource(warp-2)等待readback完成超时；新大图四行（含确认/取消、窗口像素、源QImage/上传身份、本地几何及最高画质边界）均通过。保留失败，未认定根因；原精确组单独复跑transform-mesh-exact-d3d11-repeat.txt 27项全部通过。
- transform-mesh-software.txt 35项通过；原常驻完成交接/透明栈/非法几何/取消软件及D3D11各15项通过，transform-mesh-resident-{software,d3d11}.txt。新增提交夹具第一次历史深度断言失败保留在transform-mesh-d3d11.txt；按原正式性能夹具先ORA往返清建图历史，仍严格断言单条提交及撤销/重做，后续通过，未放宽断言。
- 四ABI通过（transform-mesh-abi.txt），生成头及预编译shader检查随构建通过，最终应用嵌入shader字节核对通过（transform-mesh-embedded-shaders.txt）；git diff --check通过。未跑全量check。UI共同工作区构建一度撞到其他任务正在修改的UiResources.h重复声明，原日志保留；文件稳定后重试成功，没有把该项归因给本轮预览。

实际用户4096×2160 PNG：artifacts/actual-image-{transform,warp}-mesh及对应-mesh-repeat各两独立进程，6次诊断均通过；普通持续120/120匹配，P95为6.3933/6.4289/10.8775ms；Warp为107/110/109个匹配，P95为12.6143/13.2393/12.1419ms。补充最高画质守卫后的最终二进制actual-image-{transform,warp}-mesh-final各一进程通过，普通120匹配/P95 6.3690ms，Warp107匹配/P95 10.6744ms。所有诊断无trace/抓图、排除鼠标命中，原始输入/节点/帧身份及来源摘要由原工具记录，均不是实体或物理屏幕延迟，也没有PhotoCraft配对。此前颜色float版普通单进程12.1461ms、Warp三进程30.2217/31.0733/30.2230ms为历史对照，未重新构建同批配对基线。最终Warp确认到最终节点仍约1.98–2.05秒；不能称秒级收尾已解决。

标准EXE仍被已有用户进程锁定，最初drawverse_runtime构建LNK1104保留；测试及最终应用对象构建完成后，沿实际链接命令仅改EXE/PDB输出至build/bin/MeshTransformReview/drawverse.exe。该包沿用标准Release的同一已测试paint_ffi.dll，未以另一个core/target DLL替换；本轮无核心生产修改。最终独立应用D3D11 --profile-dir/--preview冒烟退出0，生成PNG且人工查看界面，artifacts/transform-mesh-smoke。标准Release未更新。源/二进制摘要见artifacts/transform-mesh-manifest.json。

交付前补上会话结束/主帧接管时清除大图预览策略标记，避免影响随后由文件工作线程开启的置入会话。最终同源码构建transform-mesh-delivery-build.log通过；D3D11新大图/最高画质/视口拓扑/提前关闭9项及原主帧交接6项通过（transform-mesh-delivery-d3d11.txt、transform-mesh-delivery-handoff.txt）。最终重链接包的D3D11冒烟退出0并生成artifacts/transform-mesh-delivery-smoke/preview.png。此生命周期清理不改变上述持续拖动采样算法；性能原始记录仍保留其各自源码和二进制摘要。


## 2026-10-29：按设备预算缓存原尺寸源的验收

本节修订上一节固定2048代理的画质限制。ABI1.90新增read_transform_preview_assets可选入口，尺寸/总包预算由UI按所属窗口QRhi与进程预算传入，完整纹理不受单次renderer8192边/16M约束，内部按有界条带准备。选区、旧kind1/3、最高画质精确路径和最终原图物化/历史保持；细节见设计同日末节。

核心：transform-adaptive-core.log四项通过，含9000宽实际编码跨条带逐像素一致、原图逐像素/旧2048包、单源Arc复用与退出释放，以及12000×8000在充足预算下完整保留/设备和内存预算降低时同比缩小/不足预算拒绝。transform-adaptive-public.log三项通过，含新入口选区/旧generation拒绝和查询/复制一致。严格Clippy（paint-ffi/paint-render全部targets）通过，生成头检查通过；独立build/transform-adaptive-abi四C11/C++20静态/共享ABI通过。没有总check。

实际用户PNG（只读环境变量指定）：transform-adaptive-quality-final.txt三项通过（含init/cleanup）。原图4096×1782，源纹理4098×1794（包含瓦片对齐尾部和透明边界）；全部原图范围RGBA8最大通道误差0，更新复用同一QImage与单次上传，取消保持无历史。此结果证明输入纹理未丢细节，不是任意变形后与最终六插值逐像素等价的声明。

transform-adaptive-large-final.txt七项通过：普通/Warp确认及取消四行，另9000×256完整源D3D11上传/复用、即时几何及窗口蓝色像素一行，含单历史、撤销重做及最高画质精确边界。策略JSON见artifacts/transform-adaptive-policy，实际硬件TextureSizeMax=16384；9000宽例原通用合成view unavailable，新路线独立读取设备能力并由核心严格三图分割确认，未要求先获得通用合成帧。12000×8000仅为预算计划测试，没有实际分配96MP纹理或量测其流畅度。

原精确路线联合组transform-adaptive-regression-d3d11.txt为30通过/1失败，失败为旧transformResourcesFollowViewAndTopology首个update等待revision/CPU帧/geometry一致超时，单独复验transform-adaptive-topology-isolated.txt三项通过。失败原样保留，尚未统一归因；不可称完整批量回归全绿。

探索性失败保留：transform-fullres-core.log首次缺Shared测试初始化字段、transform-fullres-build.log的kind局部插入错误、transform-fullres-public.log选区恒等初始源漏拒绝，修正后对应定向通过；transform-fullres-d3d11.txt质量夹具误以为尾部不对齐，改为允许至下一64像素瓦片且仍逐像素核对原图后误差0。初次Clippy元组复杂度失败保留transform-adaptive-clippy-initial-failure.log，替换具名缓存结构后通过。transform-adaptive-d3d11/device-d3d11/mesh-final等调查轮遇首次设备能力未送达而未进入网格，部分进程由本任务终止，保留部分日志，不计通过；能力改为独立发现且BEGIN直接读取当前窗口原子计数，最终五数据行同进程通过。wide-single初次依赖原通用合成就绪失败，修复能力/路线耦合后实际宽纹理通过，未放宽像素/上传/历史断言。

首次预备仍可能重复构建旧2048源与完整源，且尚无所有resident工具/多窗口/在途资源的统一预留池；保守预算份额不是总峰值保证。最终确认收尾、任意超硬件范围图、真实设备故障、实体笔/物理屏幕和PhotoCraft同条件配对均不能从这些结果推出。当前只交付独立包，已有标准EXE及用户进程保持。

更正上一节二进制说明：CMake中drawverse_ui链接drawverse_ffi_static，应用和Qt测试实际核心来自EXE内静态库；不能仅用同目录paint_ffi.dll身份判定其核心版本。此轮应用/测试重新静态链接ABI1.90，独立包另带匹配DLL供共享消费者，四ABI在独立目录验证。


最终补验：软件通用合成组35项通过（transform-adaptive-software.txt），原常驻主帧交接/透明栈/非法中间几何/取消软件与D3D11各15项通过（transform-adaptive-resident-*.txt）。最终应用静态链接产物build/bin/FullResolutionTransformReview/drawverse.exe，D3D11隔离profile/preview冒烟退出0，实际PNG人工查看界面正常（artifacts/transform-adaptive-smoke）。应用嵌入shader字节检查transform-adaptive-embedded.txt通过，git diff --check通过。标准Release仍被已有用户进程使用，未停止或替换；新包包含当前匹配DLL，但应用核心来自静态链接。

实际提供PNG的四个独立进程：artifacts/mohan-image-transform-adaptive/run-{1,2}普通持续P95 12.1362/12.3036ms，120输入分别匹配115/117；mohan-image-warp-adaptive/run-{1,2} Warp持续P95 9.3100/9.9408ms，120输入分别匹配115/118。全部工具判定通过，无trace/抓图、API驱动、DPR1.5，排除真实鼠标命中，不是实体/屏幕延迟或PhotoCraft配对。此PNG旧2048代理单进程基线mohan-image-transform-2048普通持续P95 6.3255ms/120匹配；完整纹理普通尾延迟更高，不能称画质提升没有任何耗时成本或性能再次翻倍。新四进程最终交接仍约1.33–1.62秒，未解决收尾。记录与源码/EXE摘要由原工具保留，最终交付摘要artifacts/transform-adaptive-manifest.json。
