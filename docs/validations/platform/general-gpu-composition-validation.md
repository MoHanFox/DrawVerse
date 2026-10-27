# 通用 GPU 合成验收计划与阶段记录

日期：2026-10-27。对应[设计](../../platform/general-gpu-composition-design.md)与[实施提示词](../../planning/general-gpu-composition-prompt.md)。

## 当前状态

G0 已交付当前版本移动/变换基线、真实 Qt 同设备三通道直显试验及选型记录（第9–10节）。首帧性能目标未达成，产品通用 GPU 合成仍未实现；G1–G5 未执行。第1–7节为后续验收要求，第8节保留初稿文档检查历史，不代表本轮运行结果。没有新增实体数位板或跨平台通过结论。

| 阶段 | 状态 | 必须补充的证据 |
| --- | --- | --- |
| G0 | 基线/试验/选型完成，性能差距明确 | 第10节；设备真实故障、物理显示和其他API未测 |
| G1 | 未执行 | 计划/瓦片协议、租约/预算、旧身份拒绝与四类ABI消费者 |
| G2 | 未执行 | 真窗口增量GPU合成、像素对照与CPU后备 |
| G3 | 未执行 | 移动/自由变换首次及持续输入、快释放、取消和交接 |
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
