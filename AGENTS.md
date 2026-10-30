# DrawVerse

- 选区操作与菜单2026-10-30：调整整组移到滤镜首项级联；右键羽化当前组合选区、复制/剪切选区建层、填充及内侧描边接核心单事务，三个同风格 ApplicationModal 窗口确认后修改、取消无历史，绑定所属画布。ABI1.95/历史49–53，完整森林 LayerEdit 内部tag14与冷页/编码后备，普通内容层及既有预算/锁保护；核心6/原选区3/历史跳转6、公开1、软件/D3D11各5功能、四ABI/Clippy/fmt/头及标准Release/D3D11冒烟通过。初始坐标/编码/测试夹具失败保留，无总check/实体或新性能结论；同期旋转等工作区改动独立保留。见docs/tools/selection-context-menu-design.md及对应验收。


- 统一调整画布2026-10-30：图像大小/裁剪合并为编辑→图像→调整画布，八手柄/九宫格/尺寸提示、像素/比例0:0/交换、DPI、默认保留裁剪像素与白色/前景扩展填充；进入收起面板并隐藏浮窗，确认单历史、取消无历史。ABI1.93新增48字节DTO与精确publication DPI读取，ORA dpi.txt与PNG pHYs保存。核心4/历史跳转6/IO2/公开1、软件/D3D11各4功能与四ABI/严格Clippy/头/fmt通过；宽泛history长测试主动中断，旧失败保留，无总check/实体性能结论。标准EXE运行中保留，独立build/bin/CanvasResizeReview交付。菜单前三项图层/图像/选择；选区右键仅接入现有五项，其余参考能力未实现。见docs/tools/canvas-resize-design.md及对应验收。

- 图层透明度选区2026-10-30：保留全选/Ctrl+A整画布，选择菜单新增“选取不透明像素”；Ctrl缩略图替换，叠加Shift/Alt/两者添加/减去/相交，编辑目标/图层多选保持。原图8位alpha/孔洞/偏移裁剪，普通内容忽略自身显示属性/蒙版，组独立合成、蒙版取白覆盖；空层为启用空选区，9000²隐式白底保持几何，沿原蒙版/编码预算原子拒绝。ABI1.92新增入口/历史45，原DTO/state/文档不变；核心5/公开相关7、软件与D3D11各5功能、严格Clippy/fmt/头/四ABI及Release对象通过。标准EXE运行中保留，独立build/bin/LayerAlphaReview静态链接与实际D3D11冒烟0。初始夹具/错误状态断言/QML构建失败保留，其他并行工作区改动不纳入本模块提交，无总check/实体或新性能结论。见selection设计/验收2026-10-30节。

- 选区变换跟手修复2026-10-29：ABI1.91发布原位/移动不可变源对，BEGIN去同步编码、先读网格，普通安全选区（含小层）按最新输入本地几何/原位切源；两源平分设备预算，无固定整图边长。用户4096×2160羽化持续输入旧30秒无源→硬/羽化普通/Warp最新节点551–906ms，仍有冷准备且非实体/屏幕。D3D11新17/原相关34及最后dense4、软件34、核心31/原4忽略、新actor1/公开1及原4/严格Clippy/四ABI通过，原夹具/编译失败保留，无总check。标准EXE被用户进程占用，独立build/bin/SelectedTransformReview静态新核心/UI及D3D11冒烟通过；详见general-gpu-composition末节。

- 变换确认收尾2026-10-29：CPU三次插值复用double权重/同瓦片四像素定位，精度顺序及原图历史不变。用户4096×2160普通确认到节点1.56–1.64→两进程1.20–1.29秒、Warp1.68–1.73→1.25–1.35秒（非实体/屏幕/零延迟）。核心31/原4忽略、严格Clippy、四ABI、大图D3D11七项及原交接24项通过，标准Release构建更新；夹具偏移/测试类型初始失败保留，无总check。详见general-gpu-composition末节。

- 大图变换进入修复2026-10-29：无选区BEGIN去旧2048预备，完整源先捕获网格避免连续输入revision失配反复复制，原尺寸单根瓦片直编码/显示查表去roundf。用户4096×2160原图首次最新位置节点3118→三进程225–236ms、原图RGBA8误差0；非实体/屏幕或零延迟，收尾未优化。核心/公开/四ABI/相关Clippy及大图9、常驻选区移动软件/D3D11各25通过；中间无改善/构建失败保留，无总check。独立build/bin/TransformStartupReview静态链接及D3D11冒烟通过，标准EXE未替换，预算仍按设备无固定整图边长。详见general-gpu-composition设计/验收末节。

- 原尺寸变换缓存2026-10-29：ABI1.90按所属窗口QRhi/DXGI与内存预算协商完整源，分块准备不以4096/8192或16M限制整张纹理，通用合成未就绪时保守根栅格可尝试、核心权威分割。用户4096×1782 PNG缓存原图范围RGBA8误差0，9000宽实际D3D11完整上传复用通过；普通/Warp持续P95两进程12.14–12.30/9.31–9.94ms（非实体/屏幕/PhotoCraft配对），交接仍1.33–1.62秒。核心4/公开3/四ABI/Clippy/软件35/原常驻两后端各15及大图五行通过；旧批量拓扑超时与调查失败保留，拓扑独立通过，无总check。独立build/bin/FullResolutionTransformReview静态链接新核心及D3D11冒烟通过，标准EXE未替换；首次重复准备/统一跨resident在途预算仍待，详见general-gpu-composition末节。

- PhotoCraft路线大图变换2026-10-29：普通画质可安全分离的大单根Normal层优先静态三图/mip/本地几何网格预览，最高画质保留原尺寸精确路径，最终原图六插值/历史不变。交互代理仍最长2048，非自适应细分/原图全尺寸mip。实际4096×2160四进程普通持续P95 6.37–10.88ms、Warp10.67–13.24ms；历史float版12.15/30–31ms非同期重建配对，Warp确认仍约2秒，无实体/屏幕结论。新大图四行、软件35/常驻两后端各15/四ABI通过；D3D扩展35通过1旧readback超时保留，原精确组独立27通过。标准EXE锁定，独立build/bin/MeshTransformReview已链接及D3D11冒烟通过；详见general-gpu-composition设计/验收末节，无全量。

- 自定义快捷键：`docs/interface/custom-shortcuts-design.md` / `docs/validations/interface/custom-shortcuts-validation.md`；命令真值在 `ui/src/platform/ShortcutSettings.h`，菜单和各画布读取 bindings，录入/冲突/持久化定向验证见验收。

- 大图取消边框/方向角稳定2026-10-29：关闭请求入队即隐藏独立控制层，拒绝尾随UPDATE/CHECKPOINT，像素revision交接保留；4096×2160 QML及原相关软件/D3D11各8、GPU关闭6行通过。初移0.75px抗抖锁向，绘制方向最短角差按距离平滑，原落点/其他方向动态保持；核心新2/device11/direction3/相关39、Clippy/fmt/头/Release/标准D3D11冒烟通过。标准build/bin/Release已更新，运行中TransformReview保留；无实体或总check。见transform-resident-preview与direction-angle-smoothing设计/验收。

- 实际4K变换颜色核优化2026-10-29：GPU颜色权重改float，几何/覆盖仍double、原图物化不变。原图GUI API普通持续P95 30.43→12.15ms，Warp60.57→三进程30.22–31.07ms；非实体/PhotoCraft配对，首帧/秒级收尾仍待。D3D11原27项两次、新1024²三核5项通过，线性最大4.232e-6。标准EXE被运行进程锁定未更新，独立build/bin/TransformReview/drawverse.exe链接及实际D3D11冒烟通过；原LNK1104保留，无全量。详见general-gpu-composition验收末节。

- 密集绘画预算修订2026-10-29：GPU源传输与CPU瓦片缓存解耦，按内存/GPU能力取预算；计划65536引用/128MiB元数据、唯一源atlas按硬件扩列小步增长，上传8MiB分批，导航保留同文档/设备Weak身份。用户4096×1782原图64MiB缓存下原提前回退复现；最终普通3层5376/最高5层8960瓦片绘画/导航/曲线均GPU，六笔各读1瓦片。核心11/公开11、四ABI/Clippy/fmt/头/shader/Release/D3D11冒烟及原25笔两后端通过；两项旧LOD断言及初次导航增量/构建失败保留。标准Release更新；非超显存分块合成/任意文档永不CPU/全量/实体性能结论，详见general-gpu-composition末节。

- 实际原图变换诊断2026-10-29：用户4096×2160 PNG打开后走D3D11精确GPU；独立GUI几何API普通/Warp持续节点采用P95 30.433/60.567ms，Warp最终交接约3秒。trace窗口GPU帧P95 63.973ms、CPU记录2.575ms，指向GPU工作量；非单shader/实体/PhotoCraft配对结论。新增实际图/真实控制点诊断，无新生产修复，原生鼠标干扰失败保留；置入/显隐本轮未测。见general-gpu-composition验收末节。

- 变换/显隐响应修订2026-10-29：共享网格横向插值/投影系数复用、UPDATE去全局状态通知，显隐即时元数据/短轮询、普通页表保留及有界damage，奇偶交接复制保留远端上层。核心25+公开7、软件/D3D11定向及新增显隐像素/历史/拓扑、四ABI/Clippy/fmt/头/Release/应用D3D11冒烟通过；原失败与DPR夹具纠偏保留。12原生配对进程有效，但普通持续P95及Warp首帧/长尾无稳定改善，不能称几个卡顿已全部解决。标准Release更新；详见general-gpu-composition验收末节，无总check/实体延迟结论。

- 最新实际密集4K/换色收尾2026-10-29：用户PNG2176瓦片触发旧2048源身份整表清空，小笔读2176→1；完成包最多4096 Weak身份、在读硬界8192，像素预算不变。最近颜色仅补丁snapshot并沿可信共享投影，保留catalog/预设数组，外部JSON完整验证。实际图三笔未变数据身份/源增量/历史及96输入50更新、颜色5功能+软件2、原25笔独立复验通过；旧窗口失活/两项修复前失败保留。标准Release更新，实际D3D11冒烟退出0。同期其他任务编译等待/窗口干扰记录，不作精确倍数/实体/任意大图或全量结论，ABI1.89/state45保持。详见general-gpu-composition末节。

- 最新GPU绘画交接2026-10-29：参考PhotoCraft活动笔触刷新，ABI1.89协商LATEST_SNAPSHOT=256按revision下界捕获并回传真实身份，旧精确语义/epoch/取消/预算保持；独立2ms活动显示邮箱轮询，后台其余服务原频率。两轮160输入GPU27/27→93/87更新，最大交付162/127→27/27ms，非实体或屏幕延迟。核心4/公开11、D3D11持续对照/25笔/4K及BUSY两行、软件25笔/四ABI通过。旧BUSY夹具pointer/原生zoom等待失败保留，按source身份及新request恢复复验。初次标准EXE占用、误跑旧测试日志保留，最终标准Release及GpuBrushReview更新，实际标准D3D11冒烟退出0。无全量/实体或独立消融结论，详见general-gpu-composition末节。

- GPU绘画卡顿复查2026-10-29：用户确认资源复用未解决。新增持续160输入不等revision的CPU/GPU诊断，两进程CPU86/89更新、最大交付27/19ms；GPU27/27更新、162/127ms，最终历史/追平通过不代表性能通过。本地PhotoCraft对比指向活动笔触damage直刷与DrawVerse16ms后台/8ms核心/精确revision请求捕获重试链路差异；整栈/prefix及块资源另有差距，未分阶段归因。仅补诊断，无新生产修复或实体延迟结论，详见general-gpu-composition验收末节。

- 最新GPU绘画资源复用2026-10-29：同拓扑普通绘画保留页表纹理/参数缓冲/SRB，只更新页表与增量源，布局/属性/编辑变化仍完整准备。最小D3D11连续25笔及4K连续24段/远端笔触两功能、软件25笔一功能通过，零新增相关资源/单瓦片/历史与CPU像素保持；4K默认夹具失败记录保留，显式开启后通过。Release构建并更新标准包，无全量/实体手感或前后性能结论。详见general-gpu-composition设计/验收末节。

- 最新GPU设置框恢复2026-10-29：硬件列表异步到达后显式按已选ID同步ComboBox，去掉未找到强制CPU首项；实际auto/GPU未关闭，用户偏好未改。最高画质独立缺省关闭、显式已存值优先。按用户要求未跑测试，drawverse_runtime构建通过、标准Release更新；运行中的CanvasFixReview保留。设计/验收见general-gpu-composition末节。

- 最新最高画质重启恢复：general-gpu-composition设计/验收2026-10-28末节；INI跨进程恢复的QString false在QML误判真值，renderSettings边界统一toBool，缺省仍开启。独立写/读进程的关闭/开启/缺省三行及跨文档同步通过，修复前两行失败保留；标准Release和CanvasFixReview已更新，标准D3D11冒烟通过。ABI/文档格式不变，无全量或新性能结论。

- 最新长图预览/隐藏编辑/最高画质保存与绘画增量：general-gpu-composition设计/验收2026-10-28末节；长图棋盘显式裁剪，隐藏层/祖先内容编辑静默忽略，最高画质保存即生效并跨文档跟随。原生显示颜色转换按脏区保留，4K小笔触4096/8847360像素且CPU同尺寸通道差≤1、轴复用；定向软件/UI/D3D11及密集曲线通过，旧DPR/参考需求/矩阵夹具失败保留，无新全量或实体性能结论。用户运行Release锁定EXE，标准链接LNK1104保留；最新可运行包build/bin/CanvasFixReview及实际D3D11冒烟通过，未关闭用户进程或更新标准EXE；ABI/文档格式保持。

- 2026-10-28最高质量/预算纠偏：general-gpu-composition设计/验收末节。CPU删除整次瓦片量96MiB拒绝，仍有界缓存/逐瓦片输出；内存/瓦片默认物理RAM一半且可输入，驻留1TiB按需上限。默认最高质量需GPU，整图原尺寸/lod0包括视口外；自动显存读Qt实际DXGI Budget留10%，删除4M/大图单节点及固定256/512MiB/2GiB显存门槛，不重复乘算共享目标在途帧。原协议/节点/源瓦片/16M输出/设备与选区后备保持，不能声称任意文档只因显存不足后备。核心7/存储1、GPU导航预算5/密集曲线及后备4/软件7/设置2/四ABI/Release/实际D3D11冒烟通过，标准Release已更新；旧失败保留，未跑总check/新性能/实体。


- 2026-10-28预算/显示续接：general-gpu-composition设计/验收末节；Adjustment紧凑28px/9px标题、NonModal保留导航，Ctrl+M/L/U及快捷键分类。启动内存/瓦片/GPU三项预算约束传输/多通道与在途帧，超GPU预算CPU；CPU预算内最高16M/8192边保留4K原尺寸。预取不挤占可见像素、普通GPU导航重投影全源计划，普通floor LOD/超大稀疏原策略。定向最小验证与原失败详见验收；标准EXE被已有进程占用，独立产物build/bin/CompactReview，未跑总check或性能/实体结论。


- 最新图像调整修订：`docs/tools/image-adjustments-design.md` / `docs/validations/tools/image-adjustments-validation.md`；可拖动ApplicationModal透明标题、无磁吸，独立曲线/直方图、色阶三角手柄、HSL彩轨及着色；关闭按钮默认透明/hover实红。D3D11无选区按原通用合成能力在目标源采样后调整，BEGIN中性预备，持续uniform/LUT且零源上传/回读，确认单历史并保留至主画布采用；选区/CPU/能力及故障走核心后备。核心4/算子10/GPU2、软件6/实际D3D11共21功能行、四ABI/相关Clippy/fmt/头/Release通过；原合成D3D11定向95项及56根/28组独立捕获校验通过，实际应用冒烟通过，标准build/bin/Release更新。ABI1.88新增有界原图直方图及着色语义，文档/state45保持；原失败保留，无大图代理/端到端性能/实体或总check结论。

- 最新GPU统一开关：gpu-acceleration-design及general-gpu-composition验收2026-10-28末节；计算硬件CPU统一关闭文档合成/常驻编辑预览/液化数值计算，自动或指定GPU按能力启用，Qt窗口显卡仍独立。切换清预热/恢复CPU需求，多文档空闲跟随，活动编辑不提前采用。软件7/D3D11含原后备10功能、设置各2/原交互各10、四ABI/Release/实际D3D11冒烟通过，标准build/bin/Release更新。初始夹具及并发构建失败保留；无总check/新性能/实体结论，ABI/state/文档不变。

- 最新自由变换边框/响应续查：general-gpu-composition设计23/验收24节；BEGIN成功即清除闲置/上一会话控制点和初始边界，核心确认前拒绝UPDATE，避免Ctrl+T错误边框先显。非通用GPU UPDATE后48ms内后台轮询临时2ms，随后恢复8/16ms，不新增整图任务或近似支持。软件/D3D11变换各12、后备各5、置入图真实QML拖动/快捷键/浮窗各3，最终D3D11精确合成变换24、四ABI/头/shader/Release/实际D3D11冒烟通过，标准build/bin/Release更新。无trace/抓图1024² GPU单次首移节点2.9995ms/对应帧7.6964ms、持续P95 4.3037ms；旧常驻三轮首12.119–17.971ms/持续P95 18.444–23.244ms，配对首帧及最大值更差，不称首帧改善或所有滞后消除。复杂CPU准备/收尾、额外轮询占用/实体/物理屏幕仍待；旧失败保持，ABI1.86/state45/文档不变，无新总check。

- 最新首次移动开销检查：general-gpu-composition设计22/验收23节；确认BEGIN执行后原等待Actor下一8ms服务周期发布，移动/拾取/多目标BEGIN复用原权威发布函数立即确认成功或错误，保留分页守卫/核心目标确认/最终CPU与历史。三个配对诊断首次采用中位6.926→3.411ms；最终无trace/无抓图三进程首次1.497–2.264ms、对应帧提交2.647–3.719ms，就绪1.840–2.681ms，连续360输入均匹配，零干扰/丢弃/增量源上传/读回。旧97.389ms含92.040ms确认未复现且未归因，不能称永久解决；移动页表/轴/SRB仍重建，收尾约66–68ms与Warp/实体等边界保持。相关核心4/公开8、窗口移动软件/D3D11各20、最终故障/按需各5、四ABI/头/Clippy/fmt/Release与实际D3D11冒烟通过，标准build/bin/Release更新。新测试透明空层PICK无命中失败保留，改显式白底夹具后原断言通过。ABI1.86/state45/文档不变，未新增总check或无关矩阵，旧失败与未测保持。

- 最新通用GPU合成G5按需续接：general-gpu-composition设计21/验收22节；GPU就绪后暂停主CPU及自动概览镜像，显式导航/Alt/I取色和后备按需恢复，快释放保留单个带generation/revision/颜色身份的采样；相同布局刷新不再截断缩放，真实布局仍保留原点。湿画笔矩阵用原有界分页入口并修复分段格点浮点反馈，32组合精确；旧空栈/ABR/VBR人工夹具按现行契约补严格回归。完整GPU两后端各213、最终稳定性各21、原导航各3、CPU兼容输入各98及四ABI/头/Clippy/doc/Release/shader通过；56根/28组独立线性最大1.1920929e-7/5.9604645e-8，标准build/bin/Release的EXE/DLL更新并实际D3D11冒烟。v6三轮18/18有效、零干扰/溢出/抓图；ON移动就绪首P95 10.135ms、普通变换5.661ms，但首次移动有效97.389ms含92.040ms确认未归因，Warp就绪首P95 13.467ms，G3门槛仍部分。总check两次实际失败保留，后续对应定向通过不冒充整入口全绿；探索性完整UI及旧合成Alt按键失败保留。ABI1.86/version1/state45/文档不变，真实TDR/驱动OOM/跨实体屏幕/冷盘/实体笔/物理显示及其他SDK硬件未测。

- 最新通用GPU合成G5：general-gpu-composition设计20/验收21节；Qt6.8.3 Windows/D3D11支持范围默认GPU直显，显式0/无效值/软件/其他API及不支持或预算不足明确CPU。无CPU主帧/概览引用可采用GPU；窗口/client/文档/场景图epoch拒绝旧回调，真实资源释放或需求降低后有界恢复，混合悬停暂用CPU。最终软件一次及D3D11三进程各19项、实际DPR2/3/原生1.5各13项；10窗口触发进程配额、关闭后一次恢复/计账0，三类注入故障25笔单历史/撤销及实际场景图重建、移动最后绝对输入/快提交取消/主视图交接通过。完整合成软件及D3D11三进程各213，85份独立核心真值/轮最大2.3841858e-7/窗口≤1，另56根/28组捕获每轮Rust独立校验通过；传输3/原移动12、实际UI5/工作流11两后端通过。四ABI/软件CTest213+19/生成头/shader/Release默认D3D11冒烟通过，标准build/bin/Release更新。总check工具6/fmt/全工作区严格Clippy/核心215+原忽略9通过后，湿画笔image_texture_scatter_and_dual_paths_share_paged_wet_execution输入段ResourceLimit失败退出101，干净G4核心同树及单项复现保留；不是完整通过。观察协议拒绝旧版本/包/重复通道，原比对失败及中断轮保留，未统一归因。ABI1.86/state45/文档保持；G3性能门槛部分、CPU视口镜像仍保留，真实设备故障/驱动OOM/跨显示器/冷盘/实体/物理显示及其他硬件未测。

- 最新通用GPU合成G4闭包完成：general-gpu-composition设计19/验收20节；27种栅格混合含Dissolve、隔离/穿透嵌套组、有效祖先/固定蒙版与完整剪贴链同窗口RGBA32F多通道直显。ABI1.86/DISSOLVE64/CLIPPING128、原布局/version1/state45/文档保持；整数hash/阈值及precise上游alpha修复真实D3D11 FMA错点。最终软件及D3D11三进程各117项、每轮85份独立重开核心真值最大2.3841858e-7/窗口≤1；原根混合/精确变换两后端76项、D3D56份独立核心结果通过。render44/公开10/四ABI/相关严格Clippy/头/fmt、完整软件CTest213项、补强选层确认后原两行两后端各4项、实际嵌入shader/Release/D3D11冒烟通过，标准Release更新。原精度反例等失败保留；置入图/复杂自由变换/超预算整CPU，默认OFF，无新性能，G3旧门槛/失败保持，G5续接，真实设备故障/实体/物理显示/总check未完成。

- 最新通用GPU合成G4组检查点：general-gpu-composition设计18/验收19节；隔离/穿透嵌套组、有效祖先和固定蒙版同窗口多通道直显，组/子层移动复用源并保留最终主视图交接，组自由变换显式CPU。ABI1.85/GROUPS32、32字节有界操作流，version1/state45/文档保持。软件及D3D11三进程各37项、每轮28份独立核心线性校验最大1.1920929e-7/窗口≤1，根回归两后端75项、传输3/原移动12、Ctrl+X8/快捷键3、render42/公开9/四ABI/Clippy/头/shader/Release/D3D11冒烟通过；最终构建两后端37项及相关回归复验通过。标准build/bin/Release更新。groups-native-1预算观察对象错误失败保留；G4剪贴/Dissolve继续，置入图CPU，默认OFF，无新性能，G3旧门槛/失败及G5/真实故障/总check未完成。

- 最新图层剪切：layer-actions设计/验收2026-10-28；Ctrl+X无选区剪切完整层/组子树、有选区仅剪切内容，先编码后单次原子删除/清除，跨文档粘贴及撤销保持。文本焦点原生剪切、锁定/当前蒙版拒绝保护。ABI1.84增加action6/7，文档/state45不变。核心3/公开1/原选区1、软件与D3D11各8及快捷键列表各3、四ABI/Clippy/fmt/生成头/Release/D3D11冒烟通过，标准build/bin/Release更新。原layer_batch删除全部层断言在89e20f6基线同样失败，保留断言/日志；未跑总check或性能。GPU G4支持边界和默认OFF保持。

- 最新通用GPU合成G4首批：general-gpu-composition设计17/验收18节；根栅格26种非Dissolve混合（含Normal）及固定自身蒙版接同窗口RGBA32F多通道直显，固定蒙版移动保留原坐标，非Normal自由变换显式CPU闭包。ABI1.83新增可协商能力8/16及节点元数据，version1布局/state45/文档不变，旧features7保持。最终软件及D3D11三进程各70项、三轮56份独立核心线性校验最大1.1920929e-7/窗口≤1；参数上传0、背景/蒙版瓦片增量/脏区、固定移动提交/取消及主视图交接通过。原GPU两后端各36、传输各3/原相关各12、render41/公开8/四ABI/相关Clippy/头/shader/Release及实际D3D11冒烟通过，标准build/bin/Release更新。native-1失败保留；G4整体部分，组/祖先蒙版/剪贴/Dissolve/置入图仍整视图CPU，默认OFF。无新性能量测，G3未过门槛及原生输入/zoom旧失败保持，G5/实体/真实故障/总check未完成。

- 最新通用GPU合成G3续接：general-gpu-composition设计16/验收17节；同源同视口同拓扑自由变换复用窗口页表/轴/参数/SRB，仅更新精确uniform/原网格，resize重投影Qt显示包且保留Rust租约。Warp保持double/原累加精度，优化权重/瓦片寻址与等价轮廓；抓图持有至ready，ProcDump定位一例Qt抓图对象寿命错误，旧异常不统一归因。相邻画布旧鼠标悬停在笔接触期间清除/拒绝缓存hover。最终48项软件及D3D第1/3通过，第2有原zoom失败；57份变换线性误差0/窗口≤1，资源/resize/25笔四进程通过。当前30性能20有效/10失败，9项计划外输入断言、1项目标等待超时伴计划外输入/失活；有效1024移动首5.365ms、普通变换3.934–5.303ms、Warp9.963ms/就绪12.819ms/持续P95 12.1264ms，不能称三轮完整门槛通过。原相关各12/传输各3、四ABI/嵌入shader/头/Release/D3D冒烟通过，标准exe更新；ABI1.82/state45保持。原生点击/zoom及旧失败保留，G3性能/完整回归部分，默认OFF，G4–G5/实体/真实故障/总check未完成。

- 通用GPU合成G3实现检查点：general-gpu-composition设计15/验收15–16节；根Normal/链接自身蒙版移动复用原瓦片，实际目标确认后最新绝对输入本地几何直发；无选区/自身蒙版的单根栅格自由变换复用原源、六插值/透视/Warp同核心精度，D3D11 double能力不足明确CPU。ABI1.82纯几何入口，核心revision/本地序号分开，快释放/取消及最终主视图采用交接、跨客户端epoch、隐藏恢复主动刷新；管线源准备时缓存。最终软件及D3D11三进程各47项、57份线性对照误差0/窗口≤1、原相关各10行、核心transform28/原4忽略、render40/公开8/四ABI/头/Release通过，标准exe更新。首次未缓存构建1024移动约225–234→7–9ms、普通变换17–18→3–4ms；当前缓存版正式配对待补，Warp约25ms仍未达两个周期。保留一轮Qt sendPostedEvents访问异常未归因、原完整输入两后端笔尖可见性失败及旧120s超时，native-final-2隐藏交接失败修复后native-final-3通过。默认OFF，G3性能部分/G4–G5/真实故障/实体/总check未完成。

- 最新通用GPU合成G2：general-gpu-composition验收第13–14节；实际Canvas显式DRAWVERSE_GPU_COMPOSITION=1接所属窗口QRhi，根Normal/链接自身蒙版、偏移/透明/隐式白与原分辨率合成后LOD直显，默认OFF。内容ID增量atlas/瓦片脏区；普通活动笔触CPU候选不可变捕获，历史仍核心提交。4M文档/视口、64节点1024瓦片、单节点256MiB payload/进程2GiB含在途保守预留/CPU块512MiB，超范围CPU；软件及D3D11三进程各13功能行通过，1024²密集全线性误差1.192e-7/窗口≤1，25笔单历史/静止上传0/末笔上传1、五类后备绘画撤销精确及窗口释放归零。render40/公开7/相关严格Clippy/四ABI/头/Release通过，标准exe更新，ABI1.81/state45保持；原CPU视口仍并行准备，无性能after。保留native-3第16笔未归因失败；G3–G5、真实故障/压力/冷盘/实体/总check未完成。

- 最新通用GPU合成G1：general-gpu-composition验收第11–12节；ABI1.81独立version1计划/能力/瓦片/租约，CPU文档与state45不变。根Normal/透明/偏移/隐式白/链接自身蒙版完整闭包，其他组合明确unsupported；每会话两租约、每租约256MiB pin/8MiB元数据、进程512MiB计划/16MiB读取，冷页仅worker读。显示邮箱独立业务FIFO，epoch/新文档/取消关闭拒绝旧发布；COW内容身份有界Weak缓存。render Release40、租约3/公开6、四ABI、Qt传输软件/D3D11及原相关各10行通过；真实3节点7瓦片与CPU/窗口误差0，释放计账0。标准Release更新并D3D11绘画冒烟，默认显示保持；G2–G5/实际GPU增量合成/首帧改善/真实故障/总check未完成。

- 最新通用GPU合成G0：general-gpu-composition设计/验收第9–10节；实际分支General-purpose-GPU-synthesis、起点d13e62b，保留原工作区。Qt6.8.3/D3D11同窗口RGBA32F蒙版/Normal/显示转换三通道直显三轮及软件通过，正常九帧上传1/通道3/读回0，全像素线性误差5.96e-8、RGBA8/窗口误差≤1；第二窗口独立与隐藏重建通过，非真实设备故障。v4移动/变换1024²/4096²各三进程，首移动219/310ms中位、就绪约23–27ms、变换持续约26ms，首帧目标未过；冻结失败/干扰与精确EXE摘要保留。CPU render34、相关Qt软件/D3D11各10、四ABI/头/Release通过，标准build/bin/Release更新；ABI1.80/state45保持。试验默认OFF，产品未接通用合成，G1–G5/真实冷盘/设备故障/实体/物理显示/总check未完成。

- 最新压感预览免刷新：brush-preview-refresh-policy设计/验收2026-10-27；具名standard-pressure统一主/副/描边来源笔尖，标准预览固定线性大小/流量压感，三项压感开关、曲线及最低值不失效缓存或在途任务，真实绘画与保存保留用户值。四功能含真实PNG重新生成一致、持久化及实际笔触alpha软件/D3D11各6通过，Release/生成头/diff通过，build/bin/Release更新；ABI1.80/state45保持，无新增实体或全套验收。

- 最新笔刷预览刷新规则：brush-preview-refresh-policy设计/验收；独立具名规则管理关闭模块、无效压感/最低圆度、图片硬度、零抖动种子、备用描边宽度及当前不模拟的喷枪。setter统一比较有效预览参数，请求/恢复/跨工具缓存同源，真实参数仍保存，保留在途任务/错误；启用和有效变化正常刷新。16类关闭模块/条件/异步/隔离/真实冷预览PNG一致与开启变化、原白色五类等软件/D3D11各23通过，倾斜/旋转实际预览重开软件4通过；Release/生成头/diff通过，build/bin/Release更新。ABI1.80/state45不变，未跑无关全套或新增实体/性能验收。

- 最新高漫实体按钮重复修复：wintab-duplicate-device设计/验收2026-10-26实体节；用户日志确认WinTab后伴随无Windows笔标记的core pointer点击，按精确包时间戳/按钮/相位/取整位置配对去重，保留Pen设备控件回退，无时间窗口防抖。用户实体确认像素+1、菜单保持、复选框单切均正常；原生队列日志回放及原输入四功能软件/D3D11各6通过，测试补齐原生落点遮挡检查。Release标准exe更新，生成头/diff通过；默认关闭的有界点击诊断保留，ABI1.80/state45不变。实体连续压力/侧键/翻面未新增验收。

- 最新手绘笔按钮重复点击：wintab-duplicate-device设计/验收2026-10-26末节；WinTab启用时仅对应用Quick窗口过滤Windows笔标记伴随鼠标消息，原Tablet控件回退保留，无时间防抖。像素单击+1、带标记消息不重复、真实鼠标/快速笔点击及顶栏菜单通过；相关控件/菜单/收笔最小回归软件/D3D11各6，Release与生成头通过，标准exe更新。ABI1.80/state45保持；用户高漫+Ink为背景，测试为真实Windows队列注入，实体驱动仍待用户确认。

- 最新编辑错误及GPU空转修复：layer-move-preview设计/验收2026-10-26末节；移动预热清理按实际资源状态幂等，核心允许液化/变换期间仅清缓存，真实修改仍BUSY。复现两条截图错误，D3D11静止400ms清理/呈现液化28/29、变换31/31→均0/0且零错误。按用户要求最小验证：新增公开ABI1项、Qt两行软件/D3D11各4、四ABI/生成头/fmt/Release应用构建通过，标准exe/DLL更新。ABI1.80/state45不变；未跑全套或量测GPU利用率百分比。

- 最新Ctrl移动及结束闪回修复：layer-move-preview/transform-resident-preview设计验收2026-10-26末节；Ctrl启用既有PICK_DEFERRED，核心拾取实际目标后后台源就绪即复用最新四顶点位移。关闭元数据保留resident至最终主视口revision，避免移动/自由变换露出旧帧；概览不触发交接。新增五行协议回归、完整变换输入软件/D3D11各89、真实Ctrl移动UI各4、四ABI/生成头/完整Release通过，build/bin/Release更新。ABI1.80/state45保持；首次冷资源/实体连续手感未量测，复杂蒙版仍CPU回退。

- 最新移动GPU即时显示：layer-move-preview设计/验收2026-10-26节；核心确认BEGIN来源后，CPU按最新绝对输入计算四顶点UV，Qt GPU复用纹理，不等待16ms提交/后台回包；独立显示位移参与几何及观察身份，旧回包不回退画面，快释放/取消与无源CPU回退保持。完整变换输入软件/D3D11各84、移动专项各5、高DPI实际DPR3共5、四ABI/生成头/完整Release通过，build/bin/Release更新。软件抓图证据改为等待真实ready回执，原像素/关联条件保持；ABI1.80/state45不变。复杂有效蒙版等仍CPU回退，首次冷页及实体持续手感未验收。

- 最新颜色/基础参数增量发布：shared-settings-publication-fix设计/验收；大小/流量/不透明度补丁快照当前行，同进程验证来源只放行颜色/三值/选择，工具投影与复杂修改完整校验保持，候选相同复用编码。正常隐藏服务及三档隔离/立即像素/保存重开软件与D3D11定向通过；489项1/2/3文档各192移动零整库恢复/重建。单文档D3D11慢速最大心跳色轮297→22ms、三参数361–382→32–38ms，非端到端；三文档仍最高88ms。四ABI/生成头/Release通过，build/bin/Release更新；ABI1.80/state45不变，首次冷加载/实体持续未验收。

- 最新第二点即时状态链路：shortcut-cursor-stroke-recovery设计/验收；Ctrl/Alt复用Space的KeyPress/KeyRelease即时状态发布，Hover/TabletMove/MouseRelease不再覆盖键盘快捷键，16ms轮询仅恢复丢失KeyRelease；画笔/橡皮/涂抹/套索8轮切换、无指针移动、漏modifier/物理键无Qt事件、重复Press与异常离笔软件/D3D11通过，Release与生成头更新build/bin/Release。ABI1.80/state45保持，实体持续输入仍待。

- 最新第二点快捷键UI通知：shortcut-cursor-stroke-recovery设计/验收；光标getter统一读取刷新快照，16ms轮询能比较旧/新可见性，修复Alt前后均Blank时QML轮廓/吸色十字漏通知，笔尖姿态独立。真实Windows SendInput屏蔽Qt键事件、鼠标/笔悬停静止切换及原有漏modifier/重复Press，软件与D3D11各4功能通过；Release应用/头更新build/bin/Release。ABI1.80/state45保持，实体数位板连续输入仍待。

- 最新画布停更竞态：shortcut-cursor-stroke-recovery设计/验收末节；BackendWorker所有精确publication读取成功后才设置metadataPending，最后笔触回执BUSY不再永久阻断元数据与像素轮询。新增24笔连续输入/真实像素、液化与变换退出后第25笔，连同快捷键/重复Press/异常离笔/4K预热移动，软件与D3D11各7功能通过；Release应用与生成头通过，build/bin/Release更新。ABI1.80/state45保持；并发用例不保证命中竞态窗口，实体持续输入仍待。

- 最新WinTab双设备流：wintab-duplicate-device设计/验收；WinTab启用时消费Qt仍并行投递的wmpointer Press/Move/Release及proximity，笔触重复Press幂等且以client drawing收尾，悬停轮廓与卡绘制修复。WinTab/Ink设备选择、双流Press/Leave拒绝、重复Press后Release及笔刷/液化离笔定向6项通过，完整Release与生成头通过，build/bin/Release更新。ABI1.79/state45保持；输入为真实窗口注入事件，实体连续压力/悬停/翻面仍待。

- 最新快捷键光标即时重算：shortcut-cursor-stroke-recovery设计/验收末节；参考PhotoCraft逐帧重算，画布指针在内时每16ms按Win32物理键态统一应用Item与原生窗口光标，Alt/Ctrl按下/释放不依赖单条Hover或KeyRelease，WinTab漏modifier不再闪烁；手势仍按Press时刻修饰键判定。鼠标/手绘快捷键、丢失KeyRelease恢复、重复Press及离笔Transform7项通过，完整Release与生成头通过，build/bin/Release更新；未改ABI/state，实体持续输入仍待。

- 最新快捷键光标与笔尖姿态分离：shortcut-cursor-stroke-recovery设计/验收末节；16ms轮询只更新快捷键光标，不复用完整brushCursorPose读取，笔尖轮廓半径/角度/圆度仍由真实Tablet/Hover样本更新，避免异步姿态回退导致移动闪烁；Caps Lock精确十字不经姿态读取保持稳定。鼠标/手绘快捷键及丢失KeyRelease、重复Press、离笔Transform7项通过，完整Release与生成头通过，build/bin/Release更新；未改ABI/state，实体笔连续移动仍待实机确认。

- 最新快捷键光标与鼠标笔触恢复：shortcut-cursor-stroke-recovery设计/验收；Ctrl/Alt按输入事件锁存、只在KeyRelease或窗口状态向系统键盘同步时清除，无modifier的TabletMove不再切回画笔轮廓；鼠标重复Press保持本地/client笔触一致并由Release收尾，非活动工具常驻预览不再遮住普通笔触。快捷键光标/重复鼠标Press/重复手绘Press/离笔共Transform7项、Ctrl移动UI3项通过，完整Release与生成头通过，build/bin/Release更新；未改ABI/state，实体长时连续输入仍待。

- 最新移动预热与清晰纹理：layer-move-preview设计/验收末节；撤销512px三纹理降级，移动资产按视口物理密度稀疏采样、最高2048px，空切片可1×1；切换移动工具/目标图层或多选集合后后台准备源，按generation/revision/目标ID集合/请求失效，首拖复用四顶点与同源纹理。预排未就绪时不阻塞输入并保留完整帧，不进入模糊移动纹理；资源准备/抓图计时不是端到端输入延迟。软件、D3D11完整变换套件各69项及2倍DPI首拖、四ABI/严格Clippy/fmt/头/完整Release通过；ABI1.80、state45保持，冷页/实体持续与T2/P1/H1仍待。

- 最新复杂变换CPU采样热点：transform-resident-preview设计/验收末节；SourceSampler保持64瓦片副本/FIFO，固定槽及最近瓦片寻址避免逐样本查树，缺页保留隐式白底/读取错误与单页驻留。核心24/公开7/原生4、软件及D3D11各22行含4K真实固定/祖先蒙版、四ABI/严格Clippy/fmt/头/完整Release通过，build/bin/Release更新；1024px两更新核心减少38.8–56.2%且摘要相同，非UI。ABI1.79/state45保持，实际复杂栈仍CPU物化，T2/M1/P1/H1及实体持续性能仍待。

- 最新恒等覆盖蒙版常驻预览：transform-resident-preview设计/验收末节；空/opacity=0自身及可消去祖先蒙版共用移动/变换/液化快速路径，私有三图过滤恒等覆盖避免无owner，真实内容/链接/层级保持。72组合/取消/有效遮挡拒绝与共享核心31、公开18/原生5、软件及D3D11各24行/四ABI/严格Clippy/fmt/头/完整Release通过，build/bin/Release更新；无新算子或ABI1.79/state45改变。实际遮挡固定/祖先蒙版、半透明祖先/非连续目标及首次资产/冷盘/实体性能仍待。

- 最新液化历史单份候选：long-task-resource-budget设计/验收末节；直接跳转只建一次字段/瓦片表，全部校验成功后一次游标/修订发布，逐步恢复共享LocalStep。2048键32步稀疏六跳核心5.146→0.198ms（非UI），磁盘基本持平。核心24/公开11/原生1、软件四项/D3D11五项含实际六跳画面逐像素、四ABI/Clippy/fmt/头/完整Release通过；build/bin/Release更新，ABI1.79/state45保持。H1真实冷盘、T2/P1与实体输入仍待。

- 最新液化会话字段引用：long-task-resource-budget设计/验收末节；原每步预算内保留两侧不可变字段，按容量保守计账，超预算仍原编码/磁盘校验。核心23/公开11/原生1、软件四项与D3D11五项、四ABI/Clippy/fmt/头/完整Release通过，build/bin/Release更新。64块六跳核心160.736→0.044ms、检查点8.379→0.526ms（字段非UI/冷盘）；混合后备/释放/分支/回滚通过，ABI1.79/state45保持。H1真实冷盘、T2/P1与实体输入仍待。

- 最新QML字节码：qml-bytecode设计/验收；同清单62个QML构建时编译，原qrc路径/相对导入保持，统一初始化避免静态库丢缓存。缓存与源码逐文件校验，源码/控件台/Live错误恢复/欢迎两后端各12、共享UI各8、焦点软件5/硬件6含Warp、2倍DPI两项4、四ABI/头/Release及Debug通过，build/bin/Release更新。新引擎同文本面板加载创建源码813/823→39/33ms（非整机/冷盘），硬件真实首轮26ms；软件跨工具仍有256ms，P1未全结项。ABI1.79/state45保持，T2/H1与实体输入继续。

- 最新隔离工具快照复用：brush-library-switch-latency设计/验收末节；切换复用已验证参数数组，部分隔离只补丁三值，六槽编码精确共享；发布选择前更新快照。修复闲置profile旧共享输入引发跨文档整库恢复。512项30次切换含编码部分712→49ms/完整738→29ms（内存非UI），真实489项切换软件24/29ms、D3D11 20/19ms心跳且目录重建0；两后端模型各9/共享各9/真实各3、四ABI/头/完整Release通过，build/bin/Release更新。ABI1.79/state45保持；首次硬件478ms、P1/T2/H1及实体输入仍待。

- 最新历史事务冷页校验复用：history-storage末节；Inline/Paged在一次直接跳转共享不可变Tile校验身份，下次请求重新验证；65536身份临时表满后原校验后备，不拒绝任务。新4/Release历史65与公开6/原生1/软件D3D11共享通过，四ABI/严格Clippy/fmt/头/完整Release更新build/bin/Release。512px六跳核心读取1554→402、141.698→37.146ms（OS热后备，非UI/冷盘）；ABI1.79/state45保持，H1字段引用/真实冷盘及P1/T2/实体输入仍待。

- 最新选区内容编辑：remaining-fixes设计/验收末节；选区Delete清像素保留层/选区，Ctrl+C只复制所选普通子层内容，组附带蒙版保持，置入清除可撤销恢复原源；共享行覆盖/偏移/瓦片候选与PagePool结果历史。核心新3/共享28/原生1、软件与D3D11各7行、四ABI/严格Clippy/fmt/头/完整Release通过，build/bin/Release。ABI1.79 kind4/5及历史41、state45不变；复制保留森林非合并可见结果，显式蒙版复制仍拒绝，实体输入/P1/H1/T2/I3/I6仍待。

- 最新三工具独立笔刷选择：brush-library-switch-latency末节；三个参数隔离档均独立记忆刷/擦/涂抹选择，切换隔离级别保留；Qt toolIsolation v2固定三选择ID，v1共享档迁移与完整隔离保持，选择补丁/接收方工具投影同步。新三档及输入/编辑合并软件/D3D11各12行、共享八项分别通过，完整Release/头更新build/bin/Release。ABI1.78/state45/VBR保持；I1端到端延迟、实体压感/全部停刷与焦点、I5/P1/H1/T2仍待。

- 最新隔离工具预览复用：brush-library-switch-latency末节；直接比较实际预览输入替代每预设两次JSON导出，目标缓存图保留且不同形状退役旧token，只增量更新预览行。512项完整隔离30切换207.502→13.463ms（仅内存模型，非端到端），软件/D3D11新两档及原五项各通过，完整Release/头更新build/bin/Release。ABI1.78/state45保持；三工具所有隔离档独立所选笔刷、快照编码与P1/H1/T2仍待。

- 最新 WinTab 输入：remaining-fixes末节；窗口创建前通过 Qt 配套 GuiPrivate 原生接口启用 WinTab，自动以实际 enabled 判定并后备 Windows Ink；偏好新增三档/当前后端，重启生效。真实 native 初始化本机 wintab，六驱动组合/持久化/QML及编辑回归软件/D3D11各7行、完整Release/头/真实应用预览通过。build/bin/Release，ABI1.78/state45保持；实体关闭Ink连续压力/倾斜/翻面/重复输入尚待，I1/I3/I5/I6和P1/H1/T2继续。

- 最新手绘笔异常离开/变换面板：remaining-fixes末节；proximity丢release结束笔触/液化timer并释放手势，鼠标失抓清tablet，笔绘制主动焦点；editing锁与liquify隐藏分开，变换保留浮窗、状态栏正确工具、结束清编辑守卫提示。软件/D3D11各6行及完整Release/头通过，build/bin/Release，ABI1.78/state45保持。实体压感及其他停刷/快捷键原因、I1/I2/I5和P1/H1/T2仍待。

- 最新复杂移动稀疏合成：layer-move-bounded-fallback设计/验收末节；共享RowSamples/read_sampled_row与原组/蒙版/剪贴/混合规则，仅合成512px显示所需坐标，瓦片按需pin。核心3/render23/共享定向28/原生2、4K有效固定自身及祖先蒙版与旧七类移动软件/D3D11各11行、四ABI/严格Clippy/fmt/头/完整Release通过。4K CPU预览46.208→20.461ms、2000px48.657→32.193ms，非UI延迟；种子白色蒙版误设修正黑色后窗口像素通过。标准build/bin/Release，ABI1.78/state45保持；M1首次资产/实体持续及T2/H1/P1/L1边界仍待。

- 最新 Qt Creator QML 预览：`ui/qml/README.md` / qml-preview设计与验收；独立按需宿主读取源码，临时偏好/布局，五个工作区运行项及真实六类控件台。Live按文件事件300ms合并、hash过滤及原子保存重绑，全会话析构重建，语法错误修正后可恢复；刷新清空临时编辑，正式qrc/设置保持。软件/D3D11各15行及四ABI/生成头/应用与预览Release构建通过，输出build/bin/Release；无ABI/state变化。Qt原生preview探测不稳定，最终用独立监听；未验收IDE GUI操作/Designer拖拽，首次资产性能待办保持。

- 最新组内液化：liquify-resident-preview设计/验收末节；复用移动/变换私有祖先快照，支持可消去Normal/PassThrough祖先中的普通层（含套索/羽化），真实parent不改。核心4项含36组合/7拒绝、公开11/变换14/原生1、软件及D3D11各5行及旧硬件15000px、四ABI/Clippy/fmt/头/完整Release通过；build/bin/Release，ABI1.78/state45保持。自身/祖先蒙版/半透明祖先仍回退，极窄选区粗场/2048近似/实体持续性能未结项；软件一次撤销抓图失败与最终按实际像素等待通过详见验收。

- 最新变换双源GPU预热：transform-resident-preview设计/验收末节；ABI1.78 kind3预读备用资产，Qt一次解码两套，窗口/sourceId弱注册共享纹理，首次网格帧提交两套上传，退出卸载上下层Loader。原生3/Actor1、4K硬件五行首拖及两轮原位往返上传各1→1，软件共享14/硬件16、两后端焦点/欢迎各9、四ABI/Clippy/fmt/头/完整Release通过；build/bin/Release，state45保持。首真实帧前极早输入/驱动冷启动与实体端到端仍待，复杂T2/M1/H1/P1/L1边界保持。

- 最新套索液化常驻预览：liquify-resident-preview设计/验收末节；选区不再关闭普通根层网格路径，CPU场/静态mask共用按行覆盖，Qt GPU目标坐标混合恒等与变形源。ABI1.77 focus bit2/kind2、state45不变。核心12组合/采样3/公开11/原生2、软件及D3D11各2套索行与既有无选区硬件、四ABI/Clippy/fmt/头/完整Release通过；2048×1536六输入核心1441.085→17.376ms、最终摘要相同，非UI延迟。标准build/bin/Release；复杂栈/软件仍回退，2048近似、极窄选区粗场与实体持续性能保持待办。

- 最新翻转历史/冷页恢复：ABI1.76 kind39/40区分水平/垂直，纯自由变换同用；Inline/Paged按身份去重校验冷页checksum而不载入像素LRU，显示再按需读。1024密集低驻留六次跳转核心369→215ms（热OS后备非UI）；Release单元192/公开24/原生2、软件与D3D11各10行Qt、四ABI/Clippy/fmt/头/新路径Release通过。见history-storage/history-labels验收；未称全量，耗时mix未验，套索液化及实体/冷盘仍待。

- 最新变换首拖CPU准备：deferred BEGIN/ENABLE确认前Actor合成真实移动源2048代理，会话双源最多两套三图，历史往返复用、结束清空。Actor1/核心14/软件与D3D11各6行Qt/四ABI/Clippy/fmt/新路径Release通过，见transform-resident-preview末节。ABI1.75/state45不变；Qt首次纹理上传及复杂回退/实体时延仍待，不称全部资源预热。

- 最新编辑结束焦点修复：CanvasPane保留恢复请求至busy/可见/宿主激活满足，不再跳过未激活窗口。真实变换/液化、确定/取消及主/浮文档8组合，结束不点击直接E/B/M；软件/D3D11各9行含欢迎生命周期通过，完整新路径Release更新。见transform-resident-preview验收末节；ABI1.75/state45保持，其他新增问题继续。

- 最新液化会话历史分块读写：4KiB缓冲保持编码/两侧完整校验，仅分配恢复侧；核心16/公开11/原生1、软件Qt3/D3D11 Qt3、四ABI/Clippy/fmt及新路径Release/Debug通过。字段六跳转内存174→109ms、磁盘热缓存274→174ms，非UI/冷盘；D3D11 focus帧事务回归在旧新程序均失败，见长任务预算验收。ABI1.75/state45保持，新增套索液化/翻转历史/焦点/变换首拖问题继续处理。

- 当前用户交付统一输出 `build/bin/Release/drawverse.exe`，以后完成优化须更新此路径；旧build/qt验收路径仅保留历史。最新build/run配置：CMakePresets及.run共享CLion配置，build只构建/部署Release应用，run先部署Debug再执行cmake-build-debug/bin/Debug/drawverse.exe；drawverse_runtime不编译测试，默认完整构建保持。两预设/README完整Release、两配置独立目录D3D11启动绘制、四ABI与新路径软件/D3D11各5项工作流通过。见build-reproducibility及application-build-profiles-validation，无ABI/state变化；余下优化继续按remaining-fixes推进。

- 最新默认暂存路径提示：性能设置及旧暂存框在空值时显示默认实际系统目录，恢复系统目录/保存/重开仍保持空值自动选择；两后端各5项创建与设置Qt通过，无QML警告，Release及生成头更新。见scratch-directory-creation-validation末节，ABI/state保持。

- 最新暂存缺失目录修复：Release错误8来自已保存D:/DrawVerse/tmp不存在，同一DLL系统目录正常；ScratchSpace后台初始化递归建目录，失败含路径/原因，不回退或改配置。实际原路径修复后返回0；存储9/软件与D3D11各4项Qt、四ABI/存储严格Clippy/格式及README原命令完整Release部署通过。README补齐workspace九包职责；详见build-reproducibility及scratch-directory-creation-validation。ABI1.75/state45保持，其他性能待办不变。

- 最新多类长任务结果历史：变换/整图翻转/栅格化LayerEdit及桶Stroke复用PagePool引用，选区按真实内存保守计账，未托管/跨池/元数据超预算保持编码后备；见history-storage及remaining-fixes设计/验收。核心结果6/历史5/直接跳转3/进度1/公开变换7/原生3、软件与D3D11各9行Qt、四ABI/严格Clippy/格式/生成头/Release通过。75MiB热样本物化中位275.930→0.129ms，仅核心非UI/冷IO；标准drawverse.exe于2026-10-21 17:16更新，ABI1.75/state45保持。会话字段/其他命令/完整8K16K、T2复杂回退/M1/P1首次资源仍待。

- 最新启动笔刷配置后台准备：BrushLibrary::preparePreferences沿用完整校验并预编码，BackendWorker传不可变无QObject数据，GUI一次应用、重新校验共享输入身份并丢弃关闭结果。软件/D3D11工作流各11行、共享UI各3项及Release/生成头通过；489项同步恢复187/178ms→已准备GUI应用0.171/0.177ms，非端到端。首次QML/资源仍469/361ms心跳，P1部分完成；详见笔刷切换延迟和remaining-fixes验收末节，ABI/state/VBR无变化。

- 最新浮面板销毁补齐：PanelContent在分类直接销毁时同样完成Loading再卸载，修复快速双击收起后遗留2个创建计数；原回归新增残留/cancel/alpha failOnWarning。软件/D3D11各2项共享UI与7行欢迎/关闭/拖动/state45通过，无上述诊断；共享移动/变换各14行通过。完整Release/生成头通过，标准drawverse.exe于2026-10-21 14:32更新；见remaining-fixes和welcome-workspace验收末节，其余复杂性能待办保持。

- 最新批量收尾：remaining-fixes清单/验收、welcome-workspace及transform-resident-preview/brush-library-switch-latency末节。U1/U5关闭：Loader按真实窗口/关闭状态卸载，完成未结束deferred控件，图层列表到Component完成再填充；浮窗固定透明surface，CanvasPane跨窗先隐藏解绑纹理源，原跨窗工作流与生命周期软件/D3D11通过。组内局部选区共用祖先分割，核心14/公开7/移动3/液化3/原生3及4K Qt各5通过，4K六次Warp核心345.593→1.528ms、摘要相同。state45等价目录上界修复，真实489项双文档重建16→0，软件/硬件80次选择各通过；首轮仍374/367ms，P1冷准备未完成。四ABI/Clippy/格式/生成头/Release通过，输出 build/qt/bin/Release；T2复杂固定/祖先蒙版/非连续、M1/H1与首次资产/实体持续性能仍待，无新增ABI或GPU算子。


- 最新组内变换共享常驻源：transform-resident-preview设计/验收末节；移动/变换共用可消去Normal/PassThrough祖先分割，变换原层覆盖保留显示parent，真实层级不变；完整普通目标/链接蒙版可常驻，固定/祖先蒙版、非连续和选区复杂目标仍回退。核心变换13/公开7/移动3/液化3/原生3、软件与D3D11复合变换各7行及共享移动7/4K选区2、四ABI/Clippy/格式/生成头/Release通过；硬件焦点批次激活失败后独立通过，见验收。2000px链接灰蒙版+两级Normal组六输入核心中位18335.863→5.087ms，确认3.23s，摘要一致，非UI时延。ABI1.75/state45保持，标准 `build/qt/bin/Release/drawverse.exe` 已更新；T2仅部分完成，首次资产/实体持续性能及其余清单待。

- 最新文件成功提示移除：welcome-workspace设计/验收末节；文件成功清空fileNotice，去掉欢迎页残留“文件操作完成”，失败及保存关闭保护保持。白底/透明新建直接关闭、真实保存后关闭/打开失败，软件与D3D11各2行及Release通过；ABI/state不变。D3D11退出仍有1个异步对象诊断，U5未完成。

- 最新局部选区变换常驻源：transform-resident-preview设计/验收末节；普通Normal根层、opacity/fill均1、无自身蒙版，按原选区一次分割移动源/余像素，原位资产避免羽化切缝；拖动只CPU几何/Qt GPU显示，确认原六种取样，蚂蚁线确认后更新。核心12/公开7/原生2、软件及D3D11各10行/四ABI/Clippy/格式/生成头/Release通过；4096×2160约256px套索六次Warp核心中位334.561→1.556ms，最终摘要一致，非UI时延。ABI1.75/state45不变，用户旧EXE锁定，新产物 `build/qt/bin/Release/drawverse-next.exe`。复杂T2、首次两套资产与实体持续手感仍待，2048代理仍近似。

- 最新旧描边执行清理：复刻描边设计/验收末节；删除Path/SDF执行模块、缓存、二值band及核心OutlineAlgorithm，旧模型名只在IO校验/迁移，唯一Replay/state45。核心19/算子7/IO描边7共享24/原生三模型1、软件与D3D11工作流各6行及UI各3/四ABI/Clippy/Release通过；ABI1.75不变。用户运行锁定原EXE，新产物 `build/qt/bin/Release/drawverse-next.exe`。O1完成，T1/T2/M1/H1/P1、U1/U5仍待；干画笔普通层/原预算/无新增GPU。

- 最新复刻描边实际接入：`docs/brush/core/brush-stroke-replay-outline-design.md` / 对应验收；实际主体印章/尾部四通道max差集、预览与单历史，Qt移除旧算法/独立路径调度并加允许溢出，旧设置严格校验后迁移；state45/behavior37/ABI1.75 bit63，VBR1.0。核心新5旧11内部3/IO7/原生1、软件与D3D11工作流各8行及UI各3/四ABI/Clippy/Release通过，输出 `build/qt/bin/Release`。仍CPU/干画笔普通层/8Mi预算，原生Path/Sdf兼容执行仍在，O1旧代码删除及T1/T2/M1/H1/P1、U1/U5仍待，不称清单全部完成。

- 最新大范围像素滑条：`docs/interface/pixel-slider-exponential-design.md` / 对应验收；跨度≥512px像素使用共享有符号指数轨道，含笔刷/液化/描边/起收笔/纹理偏移，1px加减与精确输入保留，非像素/小范围线性。软件/D3D11各7项、跨窗笔刷及Release通过；ABI/state不变。统一剩余清单U2/U3/U4已修复，U1/U5及算法项仍待。

- 当前统一待修复入口：`docs/planning/2026-10-21-remaining-fixes-design.md` / 对应validation；用户文档(4)剩余描边/选区及复杂变换/移动与历史冷性能，连同U1–U4 UI失败、U5生命周期诊断共同跟踪。直接master按小项验证、提交和push，不能把算子或核心计时当功能全部完成。

- 最新折叠列浮面板合并：内容区追加/标签栏插入，单面板、整组、整列按来源组与标签顺序合入同一组；侧栏图标同步顺序并复用窗口，保持折叠/保存恢复、Ctrl禁用与Esc取消。新4+欢迎2+滚轮1在软件/D3D11各7通过，Release输出 `build/qt/bin/Release`，ABI1.74/state44保持。旧面板专项仍因7条QML警告失败，欢迎页扩展其余3失败未解，不称全量通过；见 collapsed-panel-flyout-merge 设计/验收。

- 最新无文档欢迎页：正常启动不注册画布，菜单/背景/欢迎语与真实准备阶段，完成为126×34胶囊新建/打开；无文档隐藏编辑工作区/工具详情/浮动栏与flyout，布局保持，首真实文档显式绑定客户端。新2+共享2在软件/D3D11通过、变换2/四ABI/Release通过，ABI1.74/state44不变，产物 `build/qt/bin/Release`。扩展UI有4失败尚未归因，退出仍有2异步对象警告，不称全量通过；详见 welcome-workspace 设计/验收。

- 最新严格测试检查修复：移除变换测试两处i32冗余转换及历史Copy.clone，将液化测试模块移到末尾（生产和测试内容不变）。全workspace/all-targets严格Clippy与fmt通过；变换预览9/直接历史3/液化自适应3定向通过，无运行时/ABI1.74/state44改变，未称全量测试通过；Release沿用本轮Qt更新，见长任务资源预算验收。

- 最新笔刷完整恢复复用：一次restore内按原始行+精确schema复用已验证Preset，子角色直接传解析对象，保留容量/重复ID/组织/选择/编辑器与角色校验，候选失败不发布、请求结束释放。新六类失败及共享等九项软件/D3D11通过，Release输出 `build/qt/bin/Release`，ABI1.74/state44保持。489项完整恢复五次中位348.705→264.598/247.736ms，非文件冷IO或UI延迟；首次窗口仍604/528ms，见笔刷切换设计/验收，不能称冷加载完成。

- 最新共享工具缓存修复：共用模式切换工具同步偏好context并清字节缓存，保留执行目录/预设数组，无额外通知或保存。六次切换重开及共享三回归在软件通过，Release输出 `build/qt/bin/Release`；ABI1.74/state44保持。489项完整恢复五次中位348.705ms作为冷加载基线，未称首次加载完成，见笔刷切换延迟设计/验收。

- 最新跨工具笔刷选择同步：DocumentManager附已验证来源且广播字节必须匹配，精确比较所有工具profile参数与其余偏好，只投影接收方执行展示并接受选择变化，真实修改/外部/陈旧内容完整恢复；保留自身工具、预览，无目录重建/反馈。512三档×三接收工具轮换发送工具对照及共享等八项在软件/D3D11通过，Release输出 `build/qt/bin/Release`，ABI1.74/state44保持。489项双文档最长GUI间隔D3D11 57–58→17–24ms、软件66→16ms，跨工具两档最终37/19ms；非端到端响应，首次628–666ms仍待，见 brush-library-switch-latency 设计/验收。

- 最新笔刷数组编码复用：偏好对象刷新与字节编码分离，已验证预设数组按值缓存Qt Compact编码，最多六处，真实修改失效；文件与ABI1.74/state44保持。512三档字节一致/编辑重开及共享等七项在软件/D3D11通过，Release输出 `build/qt/bin/Release`。489项D3D11热GUI最长间隔54→17ms、双文档189→58ms（独立57），软件58→17/154→66；非端到端时延。首次独立541ms、整组曾2989ms未定位，不能称冷路径完成；见 brush-library-switch-latency 设计/验收。

- 最新大液化分页结果历史：长任务 prepare_result_in 保留同一PagePool的不可变结果身份，历史计引用元数据/步数，像素走原驻留与暂存预算；超元数据或未托管回退编码。仅接完成液化，不含会话字段/变换。真实液化结果4、存储10、公开液化11/历史3/换页3及原生1通过；75MiB热物化五次中位581.150→0.113ms（非UI/冷IO）。软件Qt2/D3D11 Qt3、四ABI/生产Clippy/Release通过，输出 `build/qt/bin/Release`，ABI1.74/state44保持；扩大测试Clippy既有4项阻断，见长任务资源预算验收；其他类型、冷加载、16K整图和暂存不足淘汰重试仍待。

- 最新多文档选择增量恢复：已验证快照只允许顶层/reset/profile选中项差异，整对象精确比较，其余完整校验；保持预览和多选、零目录重建/持久化反馈。512项三档及四共享回归在软件/D3D11通过，489真实配置双文档D3D11最长GUI间隔189ms、热单文档54ms、首轮776ms，仍有停顿；Release通过，ABI1.74/state44保持。见 brush-library-switch-latency 设计/验收；不同上下文广播、冷加载与热历史待。

- 最新真实笔刷切换回传修复：489预设配置/资源副本复现默认pressureDynamics显式/省略差异引起每次GUI整库restore约350–410ms；共享语义比较、后台先比同票据输入，等价不回传恢复。重复切换D3D11最长GUI间隔389→60ms、软件62ms，整库变化0；两后端新2+共享4及Release通过，ABI1.74/state44保持。首次冷路径仍约836–843ms、跨文档全量广播/历史热缓存待；见 brush-library-switch-latency 设计/验收，不能称所有切换瞬时完成。

- 最新液化资源预算：`docs/tools/long-task-resource-budget-design.md` / 对应 validations；paint-ops 自适应批次规划与 fallible 按需采样，液化暂存超限拆批/单瓦片超限 CPU 流式后备，取消独立8192输出瓦片限制并受文档配额约束。算子24/核心3+11/软件Qt1与D3D11 Qt2/四ABI/Clippy/Release通过，ABI1.74保持，输出 `build/qt/bin/Release`。8K/16K只验证范围规划，未验收整图耗时；笔刷切换、热历史、其他长任务迁移仍待，不能称全部大图路径已解决。

- 最新变换框忽略附带蒙版：初始框合并非蒙版源，附带蒙版不扩框、不按遮挡alpha缩框；显示/原分层变换保持，显式蒙版独立编辑仍按自身范围。核心新8组合/预览9/公开7/原生1，软件Qt1/D3D11 Qt1、四ABI/Clippy/格式/Release通过；ABI1.74保持，旧EXE运行锁定产物为 `build/qt/bin/Release/drawverse-next.exe`。见变换设计/验收。

- 最新笔刷列表尾条删除：用户红框的固定24px根拖放footer已删，自然空白透明拖放保留；原真实多选/重排/菜单用例改验证空白落点，软件/D3D11通过。见画笔菜单拖动设计/验收；Release旧EXE运行锁定，新主程序为 `build/qt/bin/Release/drawverse-next.exe`，无ABI/算法变化。

- 最新复杂移动有界预览：`docs/tools/layer-move-bounded-fallback-design.md` / 对应验收；固定蒙版/半透明或蒙版祖先等CPU回退只采最长边512的最近邻帧，源样本≤输出16倍、区段≤4096、sRGB近似≤1色阶，结束恢复完整显示。paint-render23/原生新旧各1、软件Qt7/D3D11 Qt7、四ABI/Clippy/格式/完整Release通过；固定3840×2160 CPU渲染中位394.346→42.927ms，非UI时延。ABI1.74/state44保持、无新增GPU算子；Release输出 `build/qt/bin/Release`，未验收实体连续手感或任意深组。

- 最新组内子层移动常驻：移动显示快照可消去无蒙版、100%opacity/fill的Normal/PassThrough祖先，真实层级和瓦片保持；Normal祖先禁止背景相关子树。两级组内单层+链接蒙版/兄弟半透明逐像素与五类拒绝，核心移动10/液化3/变换8/原生1，软件Qt5/D3D11 Qt5、四ABI/Clippy/格式/Release通过。见移动设计/验收，ABI1.74不变；祖先蒙版/半透明、固定自身蒙版等仍CPU回退，实体4K时延未验收。

- 最新笔刷选择缓存：仅选中项变更保留JSON预设/reset/隔离数组、按需更新selected，不再广播presetChanged；真实参数/资源/组织/上下文/restore失效。512项三档选择与隔离/共享/实际起笔六项软件、两项D3D11及Release通过。448项同机两轮总338/361→294/284ms、GUI最长38/31→29/28ms，选择本身仍波动，原包手感未验收。见大目录延迟设计/验收，ABI1.74/state44保持。

- 最新方向盘局部草稿：主/第二笔尖手势只更新轻量图元与数值，释放才提交一次参数并刷新完整笔触预览；连续20次真实鼠标移动零设置/预览失效信号，释放各一次，旧形状/同步两项软件 Qt 与 D3D11 通过，Release 输出 `build/qt/bin/Release`。见笔尖方向盘设计/验收，ABI1.74/state44 保持。

- 最新资源滚轮隔离：共享 BrushResourcePicker 的 WheelHandler 消费边界/快速滚轮与触摸板动量，网格外正常滚设置页；100资源/双向100事件/pixelDelta/动量/外层对照在软件 Qt 和 D3D11 通过，无 QML 警告，Release 输出 `build/qt/bin/Release`。见资源滚轮设计/验收；无 ABI/格式/GPU 变化，实体触摸板待验收。

- 最新图层行驻留：`docs/layers/layer-row-residency-design.md` / 对应验收；按 ID 更新/移动 ListModel 行，蒙版引用仅换 ID 时转移，busy 链条保持亮度。真实点击/移动/undo/redo 对象身份与像素行为在软件 Qt 和 D3D11 各通过，Release 输出 `build/qt/bin/Release`；ABI1.74 保持，组内/固定蒙版核心回退仍待优化。

- 新描边复刻算子第一步：`docs/brush/core/brush-stroke-replay-outline-design.md` / 对应validations；paint-ops::replay_outline按实际印章两尺度与四通道灰度max差集形成描边，允许溢出控制主体几何裁剪，64×64/8Mi像素有界，私有候选失败不改输入。新7/包回归16及Clippy/格式通过；尚未接实际笔触/预览/设置/旧算法移除，不是新描边功能完成，ABI1.74/state44保持，无新增GPU。

- 最新完整子树变换预览：transform-resident-preview设计/验收；根层链接自身蒙版、完整Normal/独立穿透根组、连续根多选捕获不可变原始源与世界选框，拖动CPU网格/Qt GPU代理，确认原分层分别采样。核心8+2+1+1+1、原生1、软件Qt5/D3D11 Qt5、四ABI/Clippy/Release通过；2000px双蒙版组Warp六输入核心中位8625→5.63ms，非UI时延。ABI1.74保持，输出 `build/qt/bin/Release`；组内单层/固定蒙版/非连续/选区/首次资产与实体持续性能仍待，预合成半透明/Warp存在近似。

- 最新长任务进度：`docs/tools/long-task-progress-design.md` / `docs/validations/tools/long-task-progress-validation.md`；液化完成/变换完成/整图翻转按实际瓦片与批次报告分阶段计数，未知总量不定进度，默认1000ms/设置0–10000ms，各文档与浮动窗口独立显示。ABI1.74/bit62/72B只读快照，不等actor与publication；state44/VBR保持。核心三类像素/历史及低预算失败1、原生1/actor1、软件Qt2/D3D11 Qt2、四ABI/Clippy/Release通过，输出 `build/qt/bin/Release`；既有有界瓦片复用，未验收任意OS OOM/跨平台/实体持续性能。

- 最新完整子树移动常驻源：根组/链接蒙版/连续根多选完整后序子树静态合成，位移只改四顶点；Normal隔离组内非Dissolve混合/剪贴可保留，透传仅独立Normal组合。源与瓦片身份保持；组内单层/固定蒙版/非连续/背景相关组合/置入仍回退，首次资产待。核心2+7/液化3/变换6/原生1、D3D11新3+共享2及软件新3+共享2/四ABI/Clippy/Release通过，ABI1.73不变，输出 `build/qt/bin/Release`；见移动设计/验收，未测4K实体连续时延。

- 最新直接历史跳转：参考PhotoCraft私有游标后一次恢复，文档/变换/液化一次请求/一条修订，只发布最终状态并保留redo；文档和液化读取失败回滚，generation/revision/scope拒绝陈旧请求。ABI1.73 bit61/新32字节PaintHistorySelect，state44/behavior36/VBR1.0保持。核心4/原生1/软件Qt工作流1与真实UI2、D3D11 UI1/焦点回归1、四ABI/Clippy/Release通过，输出 `build/qt/bin/Release`；见历史标签设计/本轮清单验收，冷页IO仍与跨越记录相关，未跑全量/跨平台/实体持续性能。

- 最新双重笔刷来源/纹理：来源与覆盖组合置顶共用，已有来源隐藏独立资源并继承源纹理，删除源后快照保留，主体纹理不改；SecondaryTip复用CPU TextureDab，state44/behavior36/dual-tip-texture、ABI1.72 bit60，旧编码保持。核心19/IO5/原生2/软件Qt5行、四ABI/Clippy/Release通过，输出 `build/qt/bin/Release`；详情见本轮清单/验收，无新增GPU计算，未跑全量/跨平台/实体持续输入。

- 最新历史笔刷删除：按最后指令移除八格/恢复/完整参数快照/偏好写出及即时广播，旧字段忽略；ABI1.69 End 回执保留，仅记录成功且改变像素的最近颜色。工作流7行含 path/sdf 与隔离/迁移通过，Release 输出 `build/qt/bin/Release`；ABI1.71/state43/behavior35/VBR1.0 不变，详情见本轮清单/验收。早期最近八笔条目仅为历史记录。

- 最新无关组/蒙版常驻源：普通 Normal 根层的变换/移动/液化按分割边界捕获完整上下子树，无关组和蒙版不再全局关闭预览；目标自身蒙版、组内、多目标、选区、上方剪贴/特殊混合仍待。Rust2+既有4+7、D3D11两行和软件三行/Clippy/Release通过，ABI1.71不变，输出 `build/qt/bin/Release`；见 transform-resident-preview 设计/验收，未测4K实体连续时延。

- 最新笔尖方向盘：`docs/brush/ui/brush-tip-shape-control-design.md` / `docs/validations/brush/ui/brush-tip-shape-control-validation.md`；参考 PhotoCraft 短轴手柄投影交互，主/可编辑第二笔尖共用角度与圆度控件、数值双向同步，已有来源继承且隐藏。真实鼠标/参数恢复两项及整数回归在软件通过，前两项 D3D11 通过并检查截图，Release 输出 `build/qt/bin/Release`；ABI1.71/state/VBR 不变，未跑全量/跨平台/实体设备。

- 最新SDF描边实际接入：普通13已接通灰度形状/A+场+遮罩缓存、等距线max印章、真实笔触/预览和算法选择，保留建路径默认；自身/已有图片笔尖与独立纹理复用采样，尾部回放失效缓存。state43/behavior35/stroke-outline-sdf、ABI1.71，旧41/42保持；核心15+内部3/IO5/原生2/工作流31（软件GPU专项跳过1）/D3D11 SDF1/旧Qt3/四ABI/Clippy/Release通过，产物 `build/qt/bin/Release`。实际笔刷和SDF仍CPU，Qt硬件显示/轮廓为GPU，GPU总入口已补齐代码与范围；50%几何/细区域无内等距线/8Mi像素与6400万采样边界见SDF设计/验收，未跑全量/跨平台/实体持续性能。

- 最新自身描边独立纹理：普通13第二步，建路径自身来源支持独立资源选择/异步导入/空间深度/开关移除，主纹理不改；角色4捕获来源和目标，state42/behavior34/outline-self-texture、ABI1.70，旧state41/behavior33保持。IO4/新Qt2旧1/原生2/工作流30（软件GPU专项跳过1）/四ABI/Clippy/Release通过，产物 `build/qt/bin/Release`；详情见 `docs/validations/brush/core/brush-sdf-outline-validation.md`。新SDF仍只有算子，实际笔触/缓存/算法选择及最终GPU说明尚待，未跑全量/跨平台/实体输入。

- 最新SDF描边算子（第一步）：`docs/brush/core/brush-sdf-outline-design.md` / `docs/validations/brush/core/brush-sdf-outline-validation.md`；paint-ops提供负内正外距离场、等距线弧长印章、max及灰度布尔，旧建路径复用EDT；8Mi像素/262144线段与印章/6400万采样上限。新算子10/旧核心13/原生2/Qt3/Clippy与Release通过，ABI1.69及state41/VBR不变，输出 `build/qt/bin/Release`。新SDF设置、实际笔触/缓存及自身独立纹理尚待接入，没有新增GPU计算，勿视作普通13已完成。

- 最新最近八笔参数：搜索下八格按成功且改变像素的End回执记录起笔预设/混合，完整参数去重、同笔不同尺寸独立，显示笔尖+直径、点击恢复/删除后新副本/重启保持。ABI1.69准确publication增量回执（共享4096序列）；Qt待确认4096项/16MiB、单记录≤2MiB/总偏好≤32MiB，VBR/state/DTO保持。跨文档即时同步并保持各工具上下文，无额外保存反馈。原生1/分页6/新Qt5、工作流28（软件跳过GPU1）、旧共享4/四ABI/Clippy/Release通过，输出 `build/qt/bin/Release`；详情见本轮分项设计/验收。

- 最新笔刷系统隔离：设置→偏好三档，默认共用/仅每预设大小不透明度流量/完整参数与所选笔刷混合；三工具上下文共用目录与资源，Qt偏好独立version1/每套≤2MiB/总输入≤12MiB，state/VBR/ABI1.68不变。新增Qt3、工作流整组23（软件跳过GPU1）、旧共享工具4与Release通过，输出 `build/qt/bin/Release`；更改级别以当前配置初始化，详情见本轮分项设计/验收。

- 最新4K选区变换CPU优化：扫描行覆盖/已遮罩源瓦片缓存、开始捕获原边界、普通画布内零位移栅格按源选区与目标并集物化，复用白底/未改瓦片；目标选区mask裁剪。固定4096×2160约256px套索六输入中位数1444→157ms，最终像素摘要相同，非UI时延。核心10+7/原生2/Qt4K1与D3D11共享5/四ABI/Clippy/Release通过，输出 `build/qt/bin/Release`；ABI1.68不变，选区常驻GPU与实体连续输入仍待，详情见本轮分项设计/验收。

- 最新蒙版链接：核心 mask_linked 默认开启，真实链条按钮/历史/复制/ORA 持久化，解除后所有者/组移动与变换跳过蒙版；显式蒙版独立移动，绝对坐标不再重复叠加。ABI1.68 group kind5/查询，旧 ORA4/5 保持、解除链接写6/7。核心7+1/共享变换7/图层组蒙版21/IO4/原生2/Qt链条1与多选1/四ABI/Clippy/Release通过，输出 `build/qt/bin/Release`；详情见本轮分项设计/验收。

- 最新多选图层移动：当前多选归并最高层子树，绝对位移预览/取消与最终单历史共用 layer_move；ABI1.67 复制多目标数组。核心5/原生新1旧1/Qt1、四ABI/Clippy/Release通过，输出 `build/qt/bin/Release`；多目标仍 CPU 回退，GPU 与首次移动性能另行优化，详情见 2026-10-20 分项设计/验收。

- 最新常驻 GPU 蚂蚁线：`docs/tools/selection-resident-ants-design.md` / `docs/validations/tools/selection-resident-ants-validation.md`；CPU 选区几何、Qt 静态遮罩与动画 shader，文档宽高≤4096/实际 GPU 后端启用，软件/更大文档回退。真实 D3D11 交织 4K 选区 12 帧导航栅格化 1→1，软件 2 项/几何 4 项与 Release 通过，ABI1.65 不变，产物 `build/qt/bin/Release`。初始栅格化、低倍近似及显示内存边界见设计；未跑全量/跨平台/实体输入。

- 2026-10-20 用户问题清单：`docs/planning/2026-10-20-bugfix-design.md` / `docs/validations/planning/2026-10-20-bugfix-validation.md`；直接 master 分项验证、提交与推送，用户验收输出 `build/qt/bin/Release`。自由变换开始失败/取消需同时通知 active 与 busy 变化，避免核心已关闭但 QML 覆盖层残留；其余项以清单和验收记录为准。
- 最新整图水平/垂直翻转：transform 共享整数源取样与单 LayerEdit 历史，全部图层/蒙版/选区镜像，保留隐式白底、画布外像素与置入源；核心3/原生1/Qt1与共享变换5、四ABI/Clippy/Release通过。ABI1.66，既有 transform kind 新增9/10，输出 `build/qt/bin/Release`；长任务进度仍待，详情见本轮分项设计/验收。
- ABI 1.65 历史：现有 group request 增补 RENAME_LAYER=4，节点名称以图层属性历史保存，原位改名需等待首次点击的异步选层完成再聚焦输入框；详情同上述分项设计/验收。早期条目版本为各次历史记录。

- 最新Warp折叠一致性：`docs/tools/transform-resident-preview-design.md` / `docs/validations/tools/transform-fold-consistency-validation.md`；参考PhotoCraft最终三角栅格，预览/CPU物化共享145×145拓扑与行序覆盖，移除Warp单初值Newton（避免多根换面/透明洞），选区同步覆盖；Qt有序透明通道One/Zero写入，避免深度首面锁定与alpha重复。核心5/变换7/原生2、真实D3D11双功能（含不透明/半透明折叠）、软件Qt与四ABI/Clippy/完整Release通过，ABI1.64不变，产物 `build/verify-ui/bin/Release-wgpu-transform/`。预览代理/边缘采样仍近似；未跑全量/跨平台/实体连续输入。

- 最新自由变换常驻预览：`docs/tools/transform-resident-preview-design.md` / `docs/validations/tools/transform-resident-preview-validation.md`；ABI1.64，参考PhotoCraft不可变源/前向细分，单目标普通栈含置入图片拖动只算CPU几何，Qt GPU纹理合成，确认才原全精度取样；复用液化/移动静态协议，145×145网格/2048代理与视口上限，选区/复杂组合/软件保留像素路径。固定512px六输入Warp核心约1190→1.83ms，2000px约5.71ms，不含UI显示；Rust3+6/原生2及共享回归、真实GPU/软件Qt、四ABI/Clippy/格式/生成头与完整Release通过，产物 `build/verify-ui/bin/Release-wgpu-transform/`。未跑全量/跨平台/实体连续手感，预览近似与边界见设计。

- 最新套索/魔棒/油漆桶性能：`docs/tools/selection-complexity-performance-design.md` / `docs/validations/tools/selection-complexity-performance-validation.md`；魔棒与桶共用CPU有界扫描线算子/按行分类，桶批量瓦片写入；轮廓沿暴露边追踪缓存，纯选区修订复用核心像素与Qt图像纹理身份，实际绘画仍刷新。固定Release中位数4K空白桶1996→194ms，1024魔棒单层连续247→53ms/四层423→98ms，碎片轮廓35.4→0.67ms；不代表UI延迟或任意复杂图。核心区域4/工具18/原生4/公开4、Qt几何3/软件8/硬件5、四ABI/Clippy/格式/生成头/完整Release通过；产物 `build/verify-ui/bin/Release-wgpu-selection/`。ABI1.63与格式不变，光标线宽默认0已确认；CPU蚂蚁线与复杂多路径布尔组合仍保留，未跑全量/跨平台/实体手感。

- 最新图层实时移动：`docs/tools/layer-move-preview-design.md` / `docs/validations/tools/layer-move-preview-validation.md`；ABI1.63，鼠标/Ctrl拾取/平板共用绝对位移会话、16ms最新输入、松手单历史、Esc/失焦取消；普通栅格栈复用液化原图/上下层常驻纹理与四顶点，组/蒙版/特殊混合/置入图片/软件保留像素回退。只捕获外观基线，共用源按栈分区避免重复克隆；实际窗口 rendererInterface 判定后端并通知QML，修复软件被全局API误判为GPU导致停帧。Rust7项/硬件Qt4项/软件Qt5项、四ABI、Clippy/格式/生成头与Release通过；产物 `build/verify-ui/bin/Release-wgpu-move/`。未跑全量/跨平台/实体输入，初始代理构建与持续手感待实机复核。

- GPU 加速总入口：`docs/platform/gpu-acceleration-design.md`（使用、算子与 Qt 显示职责、代码示例、已加速/未加速功能）。本轮导航/轮廓：`docs/interface/canvas-navigation-adaptive-cursor-design.md` / `docs/validations/interface/canvas-navigation-adaptive-cursor-validation.md`；缩放动画、惯性、ROI 周边预取与 view1 概览兜底，Qt 有界局部 shader 自适应黑白轮廓、软件黑白回退，无 ABI/格式变化；真实 GPU5/软件6项、四ABI、Release通过，产物 `build/verify-ui/bin/Release-wgpu-navigation/`。既有跨文档颜色断言在旧/新产物均失败，详情见验收；未跑全量/跨平台/实体输入，连续手感与占用待实机复核。

- 最新液化常驻纹理预览：`docs/tools/liquify-resident-preview-design.md` / `docs/validations/tools/liquify-resident-preview-validation.md`；ABI1.62，兼容普通栅格栈拖动只更新共享 Elastic 场/有界网格，Qt复用原图与上下层纹理，完成才全精度物化；CPU/软件与复杂组合保留像素路径，直径支持15000px。RTX4060六输入核心与网格发布约11–17ms，收笔约0.4–0.5ms；真实GPU/软件各一Qt与核心/ABI最小回归、四ABI及Release通过。异步取帧用例约73ms含50ms测试轮询，不等同实际帧延迟；连续手感/占用待实机复核。未跑全量/跨平台/实体输入，源码与设备边界见设计。

- 最新 GPU 算子与弹性液化：`docs/tools/liquify-gpu-operators-design.md` / `docs/validations/tools/liquify-gpu-operators-validation.md`；分支 `feat/wgpu-support`，ABI1.61，默认 auto/可见 CPU 回退；2–16px 自适应场/紧凑块读回/不可变源 CPU 与 GPU 各 2048 瓦片缓存及预热/有界最新输入，保持画布外恢复与精确历史。初版实机2000px性能未达标；同机RTX4060 Vulkan六输入约2.4s→0.20s，预热约45ms，实际连续手感/GPU占用待用户复核。最小Rust25项/Qt两项与Release通过；原EXE运行锁定，新版位于 `build/verify-ui/bin/Release-wgpu/`。未重跑全量/跨平台/实体设备，既有全量图层断言失败见验收。

- 最新液化2000px优化：`docs/tools/liquify-bounded-parallel-design.md` / `docs/validations/tools/liquify-bounded-parallel-validation.md`；无选区大区域最多两个工作线程/共享8192瓦片计数/合计256源pin，原子发布与像素保持；2048×1536六输入推移1.49s→0.77s、旋转1.79s→0.87s。核心6+11项/Qt两项/四ABI/完整Release与Clippy通过，持续输入手感仍待实机复核，未跑全量/跨平台/设备/GPU。

- 最新大目录起笔延迟：`docs/brush/performance/brush-library-switch-latency-design.md` / `docs/validations/brush/performance/brush-library-switch-latency-validation.md`；Qt缓存偏好/执行快照与准确同步票据，原生只改选择时复用成功不可变预设，保持冷资源与来源生命周期。448项样本同步起笔最长24ms降至13–14ms，Qt7项/原生67项与Release通过；未复现用户原包强卡顿，仍需实机复核。

- 最新参数/资源布局：`docs/brush/ui/brush-settings-resource-layout-design.md` / `docs/validations/brush/ui/brush-settings-resource-layout-validation.md`；公共ParameterSlider统一同排标题/加减数值与下排滑条，三类图片资源置顶/展开方阵，重命名和建组为面板旁独立Window；Windows Release及8项Qt通过，未跑全量/跨平台/设备/GPU。

- 最新整笔描边 Qt：[设计](docs/brush/ui/brush-stroke-outline-ui-design.md) / [验收](docs/validations/brush/ui/brush-stroke-outline-ui-validation.md)；state41/ABI1.60/VBR1.0完整镜像、三来源、三位置、相对/像素宽度、第二颜色、白色预览/真实像素/单历史与新目录资源往返/保存重开，Windows Release与三项Qt通过。未跑全量/设备/跨平台/GPU。

- 当前契约入口：`contracts/README.md`，仅展示当前规范；开发迭代 schema 位于 `core/crates/paint-io/tests/fixtures/schema-history/`，仅供回归。当前笔刷文件为 VBR 1.0；VBR 读取按实际字段补全、校验和执行，不以 minor 判断功能，ABI/state/behavior 版本独立。

- 源码目录入口：`docs/platform/source-layout-design.md`；Rust 保留现有 crate 分工，内部按 `brushes` / `document` / `layers` / `history` / `pixels`、格式和 ABI 职责归档；C++ 见 `ui/src/{app,documents,canvas,engine,brushes,workspace,platform,color}`，项目头使用相对 `ui/src` 的显式目录路径；QML 按功能面板/应用装配/公共控件分组，跨目录显式 import，Theme 位于 `ui/qml/theme`，入口为 `qrc:/qml/app/Main.qml`，资源清单以 `ui/CMakeLists.txt` 为准。构建/运行记录见 `docs/validations/platform/source-layout-validation.md`，完整验证入口仍为 `python tools/check.py`。

- 文档组织：总入口为 `docs/README.md`。设计与说明按领域放入 `docs/brush/{core,dynamics,dual,wet,abr,vbr,library,input,ui,texture,performance}`、`docs/tools`、`docs/layers`、`docs/document`、`docs/interface`、`docs/platform` 或 `docs/planning`。所有独立验收文档统一放入 `docs/validations/` 下对应的领域目录，入口为 `docs/validations/README.md`；文件名保持 `*-design.md` / `*-validation.md`，并更新相关相对链接。

- 最新许可与 Release 目录权限：原创代码与文档采用根目录 `LICENSE` 的 CC BY-NC-SA 4.0，含非商业限制，第三方依赖仍遵循各自许可；README 标题下声明开发版本、备份和手动保存警告。构建将许可附带到 Release；桌面复制后 os error 5 已定位为可继承的低完整性标签，只恢复指定 Release 副本的中完整性，配置/笔刷资源与系统临时目录策略不变。设计说明见 `docs/platform/build-reproducibility.md`，实测见 `docs/validations/platform/release-directory-permissions-validation.md`。

- 最新笔刷预览轮转/ABR双笔刷间距：`docs/brush/ui/brush-preview-roundtrip-design.md` / `docs/validations/brush/ui/brush-preview-roundtrip-validation.md`；预览临时状态版本改为只升不降，`BrushLibrary::restore`按`f32`实际精度递归比较数字并保留等价状态的预览令牌，修复湿混/平滑/起收笔/双笔刷调整后“已渲染但回调被丢弃”；ABR第二笔尖间距改读`dualBrush.Brsh.Spcn`，30%不再被外层`dualBrush.Spcn=100`覆盖。组合真实预览Qt用例、ABR转换用例及Windows Release构建通过；未跑全量/跨平台/设备/GPU。

- 最新VBR 1.0兼容：`docs/brush/vbr/vbr-format-1.0-compatibility-design.md` / `docs/validations/brush/vbr/vbr-format-1.0-compatibility-validation.md`；VBR manifest写出固定`major:1, minor:0`，读取接受所有1.x并按实际字段执行；预设递归补全缺项，可选对象缺失/null保持关闭，重算行为版本与能力但不改已有正确顺序和值；旧双笔刷可正常关闭，禁用菜单项不再悬停展开。核心VBR/来源/压力/颜色/设备/散布/形状/平滑/双色/尺寸/湿混目标及Windows Release/Qt双笔刷最小用例通过；历史Schema仅作旧字段说明，未跑全量/跨平台/设备/GPU。

- 最新画笔菜单与展开组落点：`docs/brush/ui/brush-menu-drag-refinements-design.md` / `docs/validations/brush/ui/brush-menu-drag-refinements-validation.md`；画笔DropArea改为显式drop参数和固定panel作用域，消除拖动时`root is not defined`；右键菜单使用独立Window弹层，动态首项消除顶部空隙，文案统一为导出该组笔刷/该笔刷/选中笔刷，无组时移动到组禁用；画笔和图层在展开组标题下半部拖入均成为组内第一项。Windows Qt三项及Release完整构建通过，未跑全量/跨平台/设备/GPU。

- 最新ABR大包与画笔多选导出：`docs/brush/ui/brush-library-abr-multiselect-export-design.md` / `docs/validations/brush/ui/brush-library-abr-multiselect-export-validation.md`；ABR大直径/间距/圆度按Photoshop范围读取后近似钳制到原生上限，第二笔刷Double数量、纹理normal/高级模式修复；画笔库支持Ctrl/Shift画笔与组混选、多选禁画、选中项成组，工具栏导出移除并改为右键导出画笔/组/选中项，选中组导出保留完整子树。用户34MB/485画笔/304资源ABR实际Qt导入、Rust47项、Windows Qt 10项及Release完整构建通过；未跑全量/跨平台/设备/GPU。

- 最新画笔库交互修复：`docs/brush/ui/brush-library-interaction-fixes-design.md` / `docs/validations/brush/ui/brush-library-interaction-fixes-validation.md`；纹理预览临时状态统一按`textureStateVersion`提升至state21/30/31/33，修复带深度纹理的ABR预览不可用；画笔组支持点击折叠/展开，搜索忽略折叠，模型重建保持滚动位置；画笔库和图层列表拖拽时支持滚轮滚动及上下缘持续自动滚动。用户ABR临时验证、纹理版本单测、组树/滚动和双列表滚轮四项通过，Windows Release完整构建通过；未跑全量/跨平台/设备/GPU。

- 最新画笔导入导出分组：`docs/brush/library/brush-import-export-groups-design.md` / `docs/validations/brush/library/brush-import-export-groups-validation.md`；文件导入按文件名创建外层组，VBR `ordered-package-v1` 的内部组树重映射后嵌入其中，ABR 因无组织信息仅获得文件名组；单笔刷导出保留所在完整组路径，全量导出保留完整组树。Windows Release Qt 多层组单笔刷往返及全量导出导入两项通过，未跑全量/跨平台/设备/GPU。

- 最新ABR 16位/传递抖动/图片笔尖/画笔库拖动：`docs/brush/abr/abr-16bit-investigation.md`、`docs/brush/abr/abr-transfer-jitter-design.md`、`docs/brush/performance/brush-image-tip-performance-design.md`；ABR sampled 16位raw/RLE按大端全范围量化到8位覆盖；ABR `prVr`/`opVr.jitter`接入DrawVerse确定性flow/opacity随机源（`szVr.jitter`保持形状动态）、converter13/handoff12/state37/VBR 1.0/behavior29，传递页提供三通道滑条；非完全覆盖图片笔尖增加轴对齐/边缘快路径及固定基准（中位数约3.34s，程序笔尖约2.37s）；画笔库位置调整按图层列表模式加入组中心边框/上下插入线/边缘自动滚动/空区根目录，并用`preventStealing`暂停拖动期间列表滚动。核心传递抖动1、ABR导入12、state5/vbr4/vbr_image12及Windows Qt 6项通过，Release/ABI/Clippy/格式通过；真实鼠标跨行拖放、PS随机序列、实体设备/跨平台/GPU仍待。

- 最新双色删除与模式收窄：`docs/brush/ui/brush-mode-cleanup-design.md` / `docs/validations/brush/ui/brush-mode-cleanup-validation.md`；设置页删除“双色绘制”，旧state/VBR/ABR的`dualBrush.color`与`wetMix.dualColor`在Qt载入时剥离并按单色处理；纹理仅保留`multiply/subtract/darken/overlay/colorDodge/colorBurn/linearBurn/hardMix/linearHeight/height`（高度采用Krita公式），双重笔刷仅保留`multiply/darken/overlay/colorDodge/colorBurn/linearBurn/hardMix/linearHeight`，退役模式回落`multiply`。核心纹理7/双笔刷11、IO纹理15/散布9及Windows Qt 8项通过，Release/ABI/格式通过；未跑全量check、跨平台/设备/GPU。

- 最新湿混性能实机验收：2026-10-18 Windows用户实机复核确认湿混性能暂且通过，见`docs/validations/brush/wet/brush-wet-field-performance-validation.md`；剩余专项性能缩小为非完全覆盖图片笔尖和实体输入，普通大笔刷小间隔已另行通过。该结论不代表所有笔刷组合、硬件/跨平台或GPU完成。

- 最新大笔刷小间隔实机验收：2026-10-18 Windows用户以约130px直径、1%间距对比，DrawVerse纯CPU路径比同场景Photoshop更流畅；双方均存在卡顿，用户确认普通大笔刷小间隔性能通过，见`docs/validations/brush/performance/brush-large-dense-performance-validation.md`。该结论不覆盖非完全覆盖图片笔尖或实体输入专项优化；湿混已由后续实机复核暂定通过，也不代表跨平台/GPU完成。

- 最新双笔刷覆盖模式：`docs/brush/dual/brush-dual-coverage-modes-design.md` / `docs/validations/brush/dual/brush-dual-coverage-modes-validation.md`；新增颜色加深/线性加深/颜色减淡/线性减淡/叠加/柔光/强光/排除/实色混合/鲜明光/线性光/点光/划分共13种覆盖组合，`DualMode` 复用纹理公式并保留 Subtract/主零覆盖语义；state36/VBR 1.0/behavior28/`dual-cover-extended`、ABR `BlnM` 短码/名称及Qt下拉接通。核心11/IO9/ABR34和Windows Qt 5项通过，Release/Clippy/格式通过；Hue/Saturation/Color/Luminosity、随机抖动、16位、硬件/跨平台/GPU仍待。

- 最新双色首层混合快路径：`docs/brush/dual/brush-dual-color-performance-design.md` / `docs/validations/brush/dual/brush-dual-color-performance-validation.md`；首层贡献直接解析到透明底，非`Dissolve`保留源、`Dissolve`用同一哈希决定源/透明，再执行一次目标混合；27模式/两前后顺序/0部分完整覆盖率参考单元通过，`brush_dual_color_performance` 六组RGBA摘要保持，同机中位数约8.2–11.6ms（之前约8.7–12.9ms），不外推所有笔刷/Qt/设备。

- 最新散布设备控制：`docs/brush/dynamics/brush-scatter-device-design.md` / `docs/validations/brush/dynamics/brush-scatter-device-validation.md`；主/第二笔尖散布幅度/数量接入渐隐、笔压、倾斜、方向、笔旋转，ABR `bVTy=1/2/3/6/8`/`Mnm`/`fStp`、state35/VBR 1.0/behavior27/`scatter-controls`、Qt双散布页及handoff11接通；核心单元5/主真实笔触8/第二真实笔触1/IO7/ABR34和Windows Qt 5项（含初始化清理）通过，Release构建/workspace check/格式通过。旧默认与旧state/VBR字节保持；Hue/Saturation/Color/Luminosity、ABR随机抖动、16位、跨平台/硬件/GPU仍待。

- 最新ABR计算双笔刷：`docs/brush/abr/abr-dual-brush-design.md` / `docs/validations/brush/abr/abr-dual-brush-validation.md`；computed/sampled双笔刷形状、图片资源、独立间距、贡献模式、散布/数量、逐印章随机Flip及固定flipX/flipY接入converter11/handoff10、源version3/VBR 1.0及Qt state12；第二笔尖设备控制已在后续散布阶段接通。

- 最新笔刷渐隐：`docs/brush/dynamics/brush-fade-design.md` / `docs/validations/brush/dynamics/brush-fade-validation.md`；`fade-v1`按印章步数渐隐并作为尺寸/流量/不透明度独立乘数，state34/VBR 1.0/behavior26、ABR `bVTy=1`/converter10/handoff9、Qt步数与三曲线页接通；核心像素、交换、ABR映射、源基线及Windows Qt三项通过。散射/双笔尖步数和随机抖动仍待后续。

- 最新纹理溶解：`docs/brush/texture/brush-texture-dissolve-design.md` / `docs/validations/brush/texture/brush-texture-dissolve-validation.md`；`dissolve`以确定性覆盖率二值抖动近似，state33/VBR 1.0、ABR `Dslv`、Qt 21项下拉和保存重开接通；核心抖动、纹理交换15项及Windows Qt两项通过。颜色空间Dissolve/Hue/Saturation/Color/Luminosity仍待后续。

- 最新ABR设备来源：`docs/brush/abr/abr-device-source-design.md` / `docs/validations/brush/abr/abr-device-source-validation.md`；`bVTy=3/6/8`映射倾斜/方向/旋转并写入来源矩阵，converter9/handoff8、源记录独立版本校验与 VBR 1.0 按字段读取、Qt handoff恢复接通；ABR转换30项、六类动态21项、源基线及Windows Qt合成handoff通过。渐隐/抖动、未知控制、16位笔尖和完整PS等价仍待后续。

- 最新动态来源矩阵：`docs/brush/dynamics/brush-dynamics-matrix-design.md` / `docs/validations/brush/dynamics/brush-dynamics-matrix-validation.md`；尺寸、流量和整笔不透明度各自选择All或速度/切向压力/距离/倾斜/旋转/方向之一，默认All保持六来源相乘；state32/VBR 1.0/behavior25及`dynamics-source-matrix`、Qt三下拉/预览/保存重开、核心与IO最小测试和Windows Release Qt两项通过。ABR动态来源、实体设备手感、跨平台与GPU仍待后续。

- 最新纹理模式补全：`docs/brush/texture/brush-texture-mode-design.md` / `docs/validations/brush/texture/brush-texture-mode-validation.md`；在已有16种基础上新增`vividLight`、`linearLight`、`pinLight`、`divide`，总计20种覆盖率组合，state31版本校验与 VBR 1.0 按字段读取；ABR `BlnM` 对应代码及Qt下拉/状态重开/纹理与ABR回归通过，严格Clippy通过。Dissolve/Hue/Saturation/Color/Luminosity、16位笔尖和完整PS像素核对仍待后续。

- 最新高级纹理模式：`docs/brush/texture/brush-texture-mode-design.md` / `docs/validations/brush/texture/brush-texture-mode-validation.md`；新增`linearBurn`、`colorDodge`、`linearDodge`、`overlay`、`softLight`、`hardLight`、`exclusion`、`hardMix`八种覆盖率组合，总计16种，state30版本校验与 VBR 1.0 按字段读取；ABR `BlnM` 的`linearBurn`/`CDdg`/`linearDodge`/`Ovrl`/`SftL`/`HrdL`/`Xclu`/`hardMix`已映射，真实ABR导入、Qt下拉/状态重开、严格Clippy通过。其他未定义模式、16位笔尖和完整PS像素核对仍待后续。

- 最新笔触方向动态：`docs/brush/dynamics/brush-direction-design.md` / `docs/validations/brush/dynamics/brush-direction-validation.md`；`direction-v1`把输入段方向角按整圈归一化后驱动尺寸、流量和整笔不透明度三条PCHIP/线性曲线，无位移时保持恒等，默认关闭；state29/VBR 1.0/behavior24及Qt“传递”页开关/曲线接通，真实像素、保存重开和QML绑定通过。ABR方向/渐隐来源、任意动态矩阵、实体设备/跨平台和GPU仍待后续。

- 最新笔旋转动态：`docs/brush/dynamics/brush-rotation-design.md` / `docs/validations/brush/dynamics/brush-rotation-validation.md`；`rotation-v1`把设备旋转角按整圈归一化后驱动尺寸、流量和整笔不透明度三条PCHIP/线性曲线，无旋转能力为0，默认关闭；state28/VBR 1.0/behavior23及Qt“传递”页开关/曲线接通，真实像素、保存重开和QML绑定通过。ABR旋转/渐隐来源、任意动态矩阵、实体设备/跨平台和GPU仍待后续。

- 最新倾斜幅度动态：`docs/brush/dynamics/brush-tilt-design.md` / `docs/validations/brush/dynamics/brush-tilt-validation.md`；`tilt-v1`把设备二维倾斜长度按90度归一化后驱动尺寸、流量和整笔不透明度三条PCHIP/线性曲线，无倾斜能力为0，默认关闭；state27/VBR 1.0/behavior22及Qt“传递”页开关/曲线接通，真实像素、保存重开和QML绑定通过。ABR倾斜/渐隐来源、任意动态矩阵、实体设备/跨平台和GPU仍待后续。

- 最新大笔刷密集笔触性能：`docs/brush/performance/brush-large-dense-performance-design.md` / `docs/validations/brush/performance/brush-large-dense-performance-validation.md`；普通不透明画笔、100%硬橡皮、纹理、图片笔尖和灰度双笔刷在活动coverage账本达到完全覆盖后跳过重复像素计算，限制于普通混合且无动态颜色/彩色双笔刷/湿混/涂抹/蒙版/起收笔的稳定路径；固定2048×2048/半径128/240输入基准中位数约1.33s降至0.37s、RGBA摘要一致，画笔/橡皮/纹理等价性、纹理/不透明度/文档/分页/双笔刷/动态回归及格式/Clippy通过。无ABI/格式变化；彩色双笔刷、湿混、实体输入和GPU仍待后续。

- 最新笔触距离动态：`docs/brush/dynamics/brush-distance-design.md` / `docs/validations/brush/dynamics/brush-distance-validation.md`；`distance-v1`按活动笔触累计文档距离驱动尺寸、流量和整笔不透明度三条PCHIP/线性曲线，默认关闭/1000 px，主笔尖和配对第二笔尖共享尺寸因子；state26/VBR 1.0/behavior21及Qt“传递”页开关/上限/曲线接通，真实像素、保存重开和QML绑定通过。ABR距离来源、任意动态矩阵、实体设备/跨平台和GPU仍待后续。

- 最新切向压力动态：`docs/brush/dynamics/brush-tangent-design.md` / `docs/validations/brush/dynamics/brush-tangent-validation.md`；`tangent-v1`将设备-1..1切向压力归一化后驱动尺寸、流量和整笔不透明度三条PCHIP/线性曲线，默认关闭，主笔尖和配对第二笔尖共享尺寸因子；state25/VBR 1.0/behavior20及Qt“传递”页开关/曲线接通，真实像素、保存重开和QML绑定通过。实体非零切向压力、ABR来源、任意动态矩阵、跨平台和GPU仍待后续。

- 最新速度动态：`docs/brush/dynamics/brush-speed-design.md` / `docs/validations/brush/dynamics/brush-speed-validation.md`；`speed-v1`以文档像素/秒驱动尺寸、流量和整笔不透明度三条PCHIP/线性曲线，默认关闭/2000 px/s，主笔尖和配对第二笔尖共享尺寸因子；state24/VBR 1.0/behavior19及Qt“传递”页开关/上限/专用曲线接通，真实像素、双笔刷回归、保存重开和设置页交互通过。ABR速度来源、任意动态矩阵、实体设备/跨平台和GPU仍待后续。

- 最新逐印章双色：`docs/brush/dual/brush-dual-stamp-design.md` / `docs/validations/brush/dual/brush-dual-stamp-validation.md`；`interleaved-stamp-v1` 按主/第二印章顺序叠加且整笔不透明度一次，state22/VBR 1.0/behavior17/ABI1.56及Qt设置/真实预览/像素/持久化接通。第二色固定使用背景色且无UI开关；橡皮和普通涂抹忽略颜色、手指绘画涂抹用双色，湿混下的涂抹/橡皮不处理。独立路径拒绝；无全量tools/check.py或跨平台验收。

- 最新时间喷枪：`docs/brush/wet/brush-airbrush-design.md` / `docs/validations/brush/wet/brush-airbrush-validation.md`；预设级`time-interval-v1`（1–1000ms，默认50）接通核心定时沉积、state23/VBR 1.0/behavior18/ABI1.57及Qt计时器，按住不动逐tick加深且单历史；仅支持干画笔/橡皮，普通涂抹/湿混不启用。无全量tools/check.py或跨平台验收。

- 最新画笔组拖放：`docs/brush/library/brush-groups-drag-design.md` / `docs/validations/brush/library/brush-groups-drag-validation.md`；Qt 画笔库支持预设/组按位置插入、组内追加及根目录放入，C++ 原子拒绝环/非法目标；state20/VBR 1.0/ABI1.55与格式不变。真实鼠标跨行拖放未自动化，无全量tools/check.py或跨平台验收。

- 最新完整范围导出：`docs/brush/library/brush-groups-export-design.md` / `docs/validations/brush/library/brush-groups-export-validation.md`；Qt 全部预设/组导出 ordered-package-v1，内置 UUID 映射/组织投影/来源包含检查及 ABI1.55 真实写入通过；Qt 分组包副本树导入仍待接，格式版本不变。

- 最新分组包导入：`docs/brush/library/brush-groups-import-design.md` / `docs/validations/brush/library/brush-groups-import-validation.md`；VBR 经 ABI1.55 分组读入口返回 ordered-package-v1，Qt 重映射预设/组 ID 并合并整树，同名/空组保留、平包与湿混图片回归通过，启动期导入排队；state/VBR/ABI 不变。无全量tools/check.py或跨平台验收。

- 最新ABR纹理/VBR 1.0：`docs/brush/abr/abr-texture-design.md` / `docs/validations/brush/abr/abr-texture-validation.md`；本地有界8/16位raw/RLE `patt`解码/内容哈希、`Txtr`参数与预设索引关联、`TxtC`空间、八种覆盖模式（含`CBrn`颜色加深）、亮度/对比度、最低深度/逐印章深度动态、纹理来源version2及Qt来源恢复、converterVersion8/handoff v7、Qt state11/21、VBR 1.0已接；真实图案笔触/历史、旧ABR、公开ABI与严格Clippy通过。其余未定义模式仍未执行，16位笔尖仍拒绝。

- 最新纹理模式：`docs/brush/texture/brush-texture-mode-design.md` / `docs/validations/brush/texture/brush-texture-mode-validation.md`；核心和 Qt 支持七种纹理覆盖组合模式，旧 state/VBR 缺省为 normal，真实纹理交换与 Windows Qt 回归通过。ABR 的 `patt`/`Txtr` 已接入这七种模式，未定义模式仍保留来源并报告；无 ABI 主版本变化。

- 最新分组Qt只读树：`docs/brush/library/brush-groups-qt-state-design.md` / `docs/validations/brush/library/brush-groups-qt-state-validation.md`；ABI1.54 state20 在 Qt 与持久化之间完整镜像，画笔库面板显示有序组树、层级缩进并在搜索时保留父组；Windows Release 与四项 Qt 最小测试通过。建组/改名/拖动/解组、Qt 分组包合并/范围导出仍待接，ABR 纹理内部执行映射仍待接。

- 最新分组包原生交接：`docs/brush/library/brush-groups-package-design.md` / `docs/validations/brush/library/brush-groups-package-validation.md`；ABI1.55/GROUP_PACKAGE完整ordered-package-v1/2MiB与投影校验、容量无安装/完整资源来源读写/原子保存及准确state20准备串联；IO33/原生新1/旧保护5/旧公开3/Schema及Windows Release/Clippy/4 ABI/旧Qt2通过。旧四包保护保留，state/VBR不变；Qt副本合并/镜像/重置基准/分组管理仍待接。

- 最新分组状态原生入口：`docs/brush/library/brush-groups-abi-design.md` / `docs/validations/brush/library/brush-groups-abi-validation.md`；ABI1.54/GROUP_STATE明确state20准备，准确票据/完整关闭参数/图片来源/冷重建与mutate保持，公开新2/原生1/旧保护5/旧公开3及Windows Release/Clippy/4 ABI/旧Qt2通过。旧四准备/包保护保留；分组包专用交接/组织操作/Qt与重置基准仍待接，state/VBR不变。

- 最新分组资源发布：`docs/brush/library/brush-groups-persist-design.md` / `docs/validations/brush/library/brush-groups-persist-validation.md`；候选合并预检/PNG与来源先落盘后采用、故障保持目录与共享文件、实际新目录state和两笔像素载色历史恢复，IO40/Clippy/Windows Release/4 ABI/旧Qt2通过。失败可留合法未引用公共缓存，未做多文件回滚/GC；完整原生交接/Qt及重置基准全链仍待接，无ABI/state/VBR变化。

- 最新分组目标库合并：`docs/brush/library/brush-groups-merge-design.md` / `docs/validations/brush/library/brush-groups-merge-validation.md`；完整子树/指定位置/跨角色ID与合并深度容量资源预检、单修订及失败保留身份，VBR实际资源/来源与两笔载色历史内存桥已接；核心新4旧11/修订1、IO21及Windows Release/Clippy/4 ABI/旧Qt2通过。资源耐久事务/完整原生交接/Qt仍待接，无ABI/state/VBR变化。

- 最新分组VBR 1.0：`docs/brush/library/brush-groups-vbr-design.md` / `docs/validations/brush/library/brush-groups-vbr-validation.md`；组织清单/空组顺序及原子整树副本ID映射、三图片/ABR来源与两笔像素载色历史保持，IO33/旧四读写入口保护及Schema/Windows Release/Clippy/4 ABI/旧Qt2通过。平包旧字节保持，无ABI/state变化；目标库合并/完整原生交接/Qt仍待接，不宣称分组用户入口完成。

- 最新分组state20：`docs/brush/library/brush-groups-state-design.md` / `docs/validations/brush/library/brush-groups-state-validation.md`；有界组织DTO/空组及顺序、关闭参数/三图片与ABR来源新目录两笔往返，IO20/四旧入口保护及Schema/Windows Release/Clippy/4 ABI/旧Qt2通过。平目录字节保持，旧原生有组明确拒绝；VBR分组清单/整树映射/原生/Qt仍待接，无ABI或VBR版本变化。

- 最新笔刷分组Rust目录：`docs/brush/library/brush-groups-design.md` / `docs/validations/brush/library/brush-groups-kernel-validation.md`；稳定ID/有序树及DFS、原子移动解组/级联保护、同组复制导入与在途捕获，6新集成+1修订边界/5旧目录、IO5/原生2及Windows Release/Clippy/4 ABI/旧Qt2通过。旧编码有组明确拒绝，分组IO/原生/Qt仍待接，不宣称用户入口完成；无ABI/格式版本变化。

- 最新图片光标Qt入口：`docs/brush/ui/brush-cursor-image-ui-design.md` / `docs/validations/brush/ui/brush-cursor-image-ui-validation.md`；默认关闭/25%透明度全局保存，ABI1.53白色PNG与当前/在途主副4身份、独立轮廓/工具优先和失败回退，实际叠加/双载图捕获与开关恢复/多文档重启通过；Windows新2旧4 Qt/环境倍率2新2、Release/4 ABI通过。无ABI/格式变化，基础图片模式不预演随机/纸纹/最终混色，整体硬件手感用户验收。

- 最新图片光标原生入口：`docs/brush/ui/brush-cursor-image-design.md` / `docs/validations/brush/ui/brush-cursor-image-validation.md`；白色/alpha双线性面积积分、奇数比例/薄笔尖/最大资源及8项有界PNG缓存，ABI1.53/bit48查询与容量/共享生命周期通过；内核5/资源9/公开3/旧轮廓4和Clippy通过。无state/VBR变化，Qt偏好/显示/当前与在途身份尚待接，不宣称用户功能完成。

- 最新湿混双色贡献场性能：`docs/brush/wet/brush-wet-field-performance-design.md` / `docs/validations/brush/wet/brush-wet-field-performance-validation.md`；几何/RGBA同瓦片共用覆盖计算、零覆盖跳过颜色采样，四场/日记/COW与饱和继续混色保持；162组旧两遍对照及相关回归通过，8组Release摘要相同/耗时减少4.1%至29.5%，不外推所有笔刷。无ABI/格式变化，单印章/总输入预算和后续大笔刷优化保持。

- 最新输入分页Actor：`docs/brush/input/brush-input-paging-actor-design.md` / `docs/validations/brush/input/brush-input-paging-actor-validation.md`；实际Move/End有界续页、完成水位/安全笔尾发布/单历史双载色采用与共同邮箱限额，页间取消/New/stop和End屏障保持；原64px公开API/Qt新目录交换重开、任务8/新Actor6及相关旧回归、Windows Release/Clippy/4 ABI通过。无ABI/格式变化，单印章/总输入预算与大笔刷优化后续范围保持；整体设备手感由用户验收。

- 最新输入分页Document：`docs/brush/input/brush-input-paging-document-design.md` / `docs/validations/brush/input/brush-input-paging-document-validation.md`；真实主副像素/双载色及活动尾有界重放、一次滤波/无输出不重放、结束/单历史与失败取消整笔回滚，原64px新Rust入口及一次性参考通过；13新Document/10调度与相关旧核心原生回归、Windows Release/Clippy/ABI通过。Actor/公开Qt分页尚未接，不宣称画布问题已修复；无ABI/格式变化，整体设备手感由用户验收。

- 最新输入分页数值/预算：`docs/brush/input/brush-input-paging-design.md` / `docs/validations/brush/input/brush-input-paging-kernel-validation.md`；原始段主副有序游标/事务页/设备与弧长余量、保持8M/8192页预算及有界总输入，8项调度+14400组旧预算对照、相关湿混/起收笔回归及Windows Release/Clippy/ABI通过；尚未接Document像素/活动笔尾及Actor，不宣称修复画布64px密集输入。用户已恢复推进；Windows整体设备手感验收由用户自行进行，agent仅必要代码回归。

- 最新湿混双色Qt入口：`docs/brush/wet/brush-wet-dual-ui-design.md` / `docs/validations/brush/wet/brush-wet-dual-ui-validation.md`；state19完整镜像/关闭值/复制重置基准，ABI1.52新准备与VBR 1.0入口、两色手动装载捕获及独立白色预览、真实两笔/历史/图片新目录交换重开；3新6旧Qt/环境倍率2/Release/ABI通过。密集图片单次64px输入实测预算回滚，有序分块尚待实现；原输入偶发问题未定位。剩余功能见`docs/planning/brush-system-remaining.md`，用户要求本阶段完成后等待指示。

- 最新湿混双色原生入口：`docs/brush/wet/brush-wet-dual-abi-design.md` / `docs/validations/brush/wet/brush-wet-dual-abi-validation.md`；ABI1.52/WET_DUAL_COLOR、state19准确准备/VBR 1.0包与旧图片重建、原子双角色info/reload及共用标称尺寸、真实两笔像素/载色/历史；新3旧6公开API/10核心/7原生及Windows Release/Clippy/ABI/Qt通过，旧入口保护保持；Qt镜像与组合尚待接，前述输入偶发问题未定位。

- 最新湿混双色保存交换：`docs/brush/wet/brush-wet-dual-io-design.md` / `docs/validations/brush/wet/brush-wet-dual-io-validation.md`；wetMix.dualColor独立载色策略/state19/VBR 1.0/behavior16，完整关闭值/严格标记、真实两笔双载色/三图片用途/来源及新目录往返，6项新IO/旧三类入口拒绝和16项旧Rust回归、Schema/Windows Release/ABI/Qt通过；完整原生/Qt组合仍待接，旧入口保护保留。

- 最新湿混双色 Engine：`docs/brush/wet/brush-wet-dual-engine-design.md` / `docs/validations/brush/wet/brush-wet-dual-engine-validation.md`；真实Session完整双载色捕获/成功整对采用与单载体保留副色，6项新Actor/11项旧原生回归及Release/ABI通过；Qt涂抹通过、参数输入首次偶发失败单项复核通过（未定位），详见验收。保存/原生准备/Qt组合仍待接，无版本变化。

- 最新湿混双色实际绘画：`docs/brush/wet/brush-wet-dual-document-design.md` / `docs/validations/brush/wet/brush-wet-dual-document-validation.md`；Rust明确入口/完整双载色结束、两几何两RGBA场/共用27模式及饱和继续混色、四场与双载色起收笔回放/实际网格预算与单历史失败回滚，10项新函数/39项旧回归及Windows Release/ABI/Qt通过；Engine完整归还/IO/ABI/Qt组合仍待接，界面仍拒绝，无版本变化。

- 最新湿混双载色状态：`docs/brush/wet/brush-wet-pair-design.md` / `docs/validations/brush/wet/brush-wet-pair-validation.md`；WetInk主/第二独立二维载体与整对WetTool租约/清洗装载，5项新数值及21项旧回归、Windows Release/ABI/Qt通过；尚未接Document双色贡献/Engine完整返回/起收笔检查点及IO/Qt，当前组合仍拒绝，无ABI/格式变化。

- 最新空图层栈：`docs/layers/empty-layer-stack-design.md` / `docs/validations/layers/empty-layer-stack-validation.md`；最后图层/整组/批量可删空，active=0与透明棋盘格、历史/新建/空栈粘贴、ORA零层往返和透明PNG；Windows最小Rust/Qt及ABI通过，无布局/格式版本变化。

- 最新起收笔 Qt：`docs/brush/wet/brush-taper-ui-design.md` / `docs/validations/brush/wet/brush-taper-ui-validation.md`；state18完整镜像/关闭值/复制重置基准、ABI1.51准备与VBR 1.0入口、距离和独立平滑尺寸/opacity曲线、真实白色预览/在途捕获/像素历史及重开，Windows最小Qt通过；完整ABR/16位、湿混双色等仍待后续。

- 最新工具栏传递参数：`docs/brush/ui/brush-toolbar-transfer-design.md` / `docs/validations/brush/ui/brush-toolbar-transfer-validation.md`；画笔/橡皮当前预设的流量、不透明度及各自压感直接同步，涂抹下拉改细分隔线，Windows 最小三项 Qt 通过，无 ABI/格式变化。

- 最新起收笔原生入口：`docs/brush/wet/brush-taper-abi-design.md` / `docs/validations/brush/wet/brush-taper-abi-validation.md`；ABI1.51/STROKE_TAPER、显式state18准备与VBR 1.0包读写、准确捕获/旧图片修订重建和真实两笔/载色/历史已验证；旧入口保护保留，Qt镜像/基准/白色预览和设置页仍待接。

- 最新起收笔保存交换：`docs/brush/wet/brush-taper-io-design.md` / `docs/validations/brush/wet/brush-taper-io-validation.md`；不可变预设/state18/VBR 1.0/behavior15、完整关闭曲线和严格舍入前验证、真实两笔/载色/来源与全新目录交换通过；ABI1.50旧入口暂拒新对象防Qt丢值，完整原生/Qt镜像和重置基准仍待接。

- 最新起收笔实际绘画：`docs/brush/wet/brush-taper-document-design.md` / `docs/validations/brush/wet/brush-taper-document-validation.md`；Rust明确入口/真实活动笔尾瓦片与贡献场恢复、单双流/双色/湿混/涂抹、单历史和预算/暂存失败整笔回滚，10项新像素用例/相关旧回归及Windows Release/ABI/Qt通过；预设IO/原生/Qt参数仍待接，无ABI或格式变化。

- 最新起收笔数值与调度：`docs/brush/wet/brush-taper-kernel-design.md` / `docs/validations/brush/wet/brush-taper-kernel-validation.md`；独立尺寸/opacity曲线、全局弧长与双流身份、事务计划及有界活动笔尾，10项Rust与Clippy通过；尚未接Document像素/历史/IO/ABI/Qt，不宣称画布起收笔完成，无版本变化。

- 最新白色预览与范围修订：`docs/brush/ui/brush-white-preview-design.md` / `docs/validations/brush/ui/brush-white-preview-validation.md`；用户取消试画/代码和入口已清理，所有自动预览固定白色及正常模式/保持真实alpha，换当前颜色不重算；真实画布颜色不变，无ABI或格式变化。

- 最新平滑/缩放/颜色保存：`docs/brush/ui/brush-follow-color-zoom-design.md` / `docs/validations/brush/ui/brush-follow-color-zoom-validation.md`；用户确认保留拖绳、抬笔停在已绘末端不补线，Ctrl++/Ctrl+-主窗浮窗与文本保护，前景/背景及历史色偏好重启保持/多文档同步；无ABI或格式版本变化。

- 最新ABR设备形状导入：`docs/brush/abr/abr-device-conversion-design.md` / `docs/validations/brush/abr/abr-device-conversion-validation.md`；converter7/record1/交接JSON6/VBR 1.0，方向/旋转/倾斜角度与圆度/独立动态和旧六代保持；PS设备公式与下限换算标近似，关闭设备仅来源保留，16位/其他设备继续后续，无C ABI或state新版本。

- 最新Windows输入/菜单修复：`docs/brush/input/brush-windows-ink-hover-design.md` / `docs/brush/input/tablet-menu-routing-design.md` / `docs/validations/brush/input/windows-input-menu-validation.md`；笔位置所有权隔离Quick缓存Hover、原窗口光标先保存后切换、鼠标/笔菜单坐标路由与透明模态屏障；兼容窗口不激活防失焦关闭，实际Windows最小结果见验收，无ABI/格式变化。

- 最新ABR形状抖动导入：`docs/brush/abr/abr-shape-conversion-design.md` / `docs/validations/brush/abr/abr-shape-conversion-validation.md`；converter6/record1/交接JSON5/VBR 1.0，尺寸角度圆度随机与最低比例、独立压力散布颜色/完整关闭配置、旧五代来源保持；PS随机及下限叠加标近似，设备形状控制/16位等继续后续，无C ABI或state新版本。

- 最新ABR颜色动态导入：`docs/brush/abr/abr-color-conversion-design.md` / `docs/validations/brush/abr/abr-color-conversion-validation.md`；converter5/record1/交接JSON4/VBR 1.0，前景背景与HSV/逐笔逐印章和完整关闭配置、独立压力散布与旧四代来源保持；PS颜色空间/随机明确近似，纯度/设备颜色控制及16位等继续后续，无C ABI或state新版本。

- 最新ABR散布导入：`docs/brush/abr/abr-scatter-conversion-design.md` / `docs/validations/brush/abr/abr-scatter-conversion-validation.md`；converter4/record1/交接JSON3/VBR 1.0，真实数量Double与单/双轴/数量抖动、完整关闭/旧三代基准保持、四代交换及Windows Qt新库像素；PS随机分布标近似，设备散布/16位等继续后续，无C ABI或state新版本。

- 当前验收优先级（用户2026-10-15确认）：主要针对Windows平台；跨平台验收以后再做，不作为当前阶段提交/推送门槛。继续维护跨平台架构，实际验证范围如实记录。

- 最新真实ABR语料/对齐：`docs/brush/abr/abr-real-corpus-design.md` / `docs/validations/brush/abr/abr-real-corpus-validation.md`；8BIM节间/尾部四字节填充、旧无填充兼容，CC0 Myer148与两个MIT Photoshop探针完整导入/来源/像素历史及Qt新库VBR；无版本变化，16位字节序分歧与后续证据见`docs/brush/abr/abr-16bit-investigation.md`，仍未实现16位。

- 最新ABR笔压动态：`docs/brush/abr/abr-pressure-design.md` / `docs/validations/brush/abr/abr-pressure-validation.md`；转换器v3/来源record1/交接JSON2/VBR 1.0，尺寸流量opacity独立压感与最低比例、旧v1/v2字节/基准保持、三代混合/降级拒绝及真实Qt导入重置/像素/重开；无C ABI或state新版本，完整ABR动态/16位、湿混双色、收尖等继续后续。

- 最新数位笔悬停修复：`docs/brush/input/brush-tablet-hover-fix-design.md` / `docs/validations/brush/input/brush-tablet-hover-fix-validation.md`；双色页移除两个预设按钮/既有预设保留，直接入点与笔触后Stylus Hover跟随、设备几何保持/原生窗口光标有界接管与恢复、环境缩放1/2相关Qt回归；无ABI或格式变化。

- 最新ABR整笔不透明度：`docs/brush/abr/abr-opacity-design.md` / `docs/validations/brush/abr/abr-opacity-validation.md`；转换器v2/来源record1/VBR 1.0，非100%真实整笔上限/密集程序与图片、v1原基准/报告及记录字节不重算、新旧混合最高minor/降级拒绝、原生prepared与Qt导入/重置/新库交换；无ABI/state变化，完整ABR动态/16位、双色湿混及收尖等继续后续。

- 最新二维湿混涂抹：`docs/brush/wet/brush-wet-smudge-design.md` / `docs/validations/brush/wet/brush-wet-smudge-validation.md`；独立R共用载体/强度压感与工具取样、手指自动载色/透明搬运/蒙版线性灰度、真实像素/分页组合/原生两笔及Qt历史，追加原生高DPI悬停计时输入隔离与重复回归（环境倍率非硬件DPR）；无ABI/格式变化，湿混双色/橡皮、收尖/非分隔与高级ABR等继续后续。

- 最新湿混Qt入口：`docs/brush/wet/brush-wet-qt-design.md` / `docs/validations/brush/wet/brush-wet-qt-validation.md`；ABI1.50/state17/VBR 1.0完整镜像/独立基准、湿混分类与通用控件、手动载色/清洗/主文档共享工具、真实独立有色预览及DPI1/2交互/像素/持久化；新版准备/库包接通，旧接口保护保留；双色湿混/Erase/Smudge、收尖/非分隔及高级ABR继续后续。

- 最新湿混原生入口：`docs/brush/wet/brush-wet-abi-design.md` / `docs/validations/brush/wet/brush-wet-abi-validation.md`；ABI1.50/共享工具句柄与64所有者上限、准确准备手动载色/清洗/状态/跨会话绑定、state17/库包显式wet入口与真实两笔/资源交接；旧入口继续拒绝防旧Qt丢值，完整Qt镜像/基准/有色预览与双色湿混继续后续。

- 最新湿混工具所有权：`docs/brush/wet/brush-wet-owner-design.md` / `docs/validations/brush/wet/brush-wet-owner-validation.md`；共享WetTool/独占租约、原生Engine配置起笔/历史成功采用、取消/预算失败/New/stop释放与跨文档Busy、真实两笔参考像素/载色；无ABI变化，外部工具句柄/准备桥/Qt及双色湿混继续后续。

- 最新湿混保存交换：`docs/brush/wet/brush-wet-io-design.md` / `docs/validations/brush/wet/brush-wet-io-validation.md`；state17/VBR 1.0/behavior14、完整关闭策略与六比例、真实两笔载色/像素/历史及图片来源保持、Schema与合法非法示例；ABI1.49原生准备/包入口暂拒防旧Qt丢值，完整桥/Qt与双色湿混继续后续。

- 最新湿混统一设置：`docs/brush/wet/brush-wet-settings-design.md` / `docs/validations/brush/wet/brush-wet-settings-validation.md`；Rust不可变预设/编译快照、配置起笔自动清洗装载与跨笔保色/路径重置/尺寸重采样、真实像素与历史；无ABI/格式变化，旧IO非默认拒绝，完整湿混交换/桥/Qt与双色组合继续后续。

- 最新湿混颜色动态：`docs/brush/wet/brush-wet-color-design.md` / `docs/validations/brush/wet/brush-wet-color-validation.md`；逐印章准备/沉积后线性HSV、alpha与原始载色保持、真实分页组合及故障回滚、旧算法20736组位级核对；无ABI/格式变化，湿混IO/桥/Qt与双色组合继续后续。

- 最新湿混文档内核：`docs/brush/wet/brush-wet-document-design.md` / `docs/validations/brush/wet/brush-wet-document-validation.md`；独立Rust起笔/结束返回载色、真实逐印章取样/分页RGBA贡献与opacity上限、选区/蒙版/锁及单历史、失败/滤波预算回滚；无ABI/格式变化，参数IO/桥/Qt与颜色动态/双色组合仍待后续。

- 最新二维湿混载色内核：`docs/brush/wet/brush-wet-carrier-design.md` / `docs/validations/brush/wet/brush-wet-carrier-validation.md`；真实二维预乘RGBA拾色/混合/沉积/距离消耗、清洗重载与准备/提交快照、64阶有界网格及数值像素回归；无ABI/格式变化，尚未接Document/历史/预设/Qt，不宣称湿混工具已完成。

- 最新可选复合光标：`docs/brush/ui/brush-composite-cursor-ui-design.md` / `docs/validations/brush/ui/brush-composite-cursor-ui-validation.md`；ABI1.49消费/默认关闭的全局开关、主第二动态与图片轮廓/在途身份/最多4个后台请求、关闭不生成第二轮廓、DPR1/2和数位笔伴随Hover/鼠标接管；无格式变化，随机印章预演/非分隔/收尖/湿混等继续后续。

- 最新复合笔尖原生几何：`docs/brush/input/brush-composite-pose-design.md` / `docs/validations/brush/input/brush-composite-pose-validation.md`；ABI1.49/64字节双姿态DTO、同prepared捕获/独立压力设备/相对及固定边宽/图片与有效溢出标志，实际印章尺寸及C/C++布局回归；无state/VBR变化，Qt双轮廓尚待接入，不宣称完整复合光标。

- 最新自动描边预设：`docs/brush/ui/brush-auto-outline-design.md` / `docs/validations/brush/ui/brush-auto-outline-validation.md`；双色页创建跟随压感/固定边宽独立用户预设，配对硬圆/+8px/背景后方/完整基准与库上限，真实压力边宽/交叉整笔opacity/捕获及VBR历史；无ABI/格式变化，复合光标、非分隔、收尖与湿混等继续后续。

- 最新双色原生Qt入口：`docs/brush/ui/brush-dual-color-ui-design.md` / `docs/validations/brush/ui/brush-dual-color-ui-validation.md`；ABI1.48/state16/VBR 1.0、完整关闭配置/27贡献模式/实际两色预览、起笔捕获/历史及保存交换，重置预设仅来源与兼容页；解除原生准备/库包临时拒绝，自动描边与复合光标等继续后续。

- 最新双色重算性能：`docs/brush/dual/brush-dual-color-performance-design.md` / `docs/validations/brush/dual/brush-dual-color-performance-validation.md`；变化行位图OR/精确像素重算、before与私有写入/快照、实际区域预算及1%密集段、648组真实瓦片历史一致与Release基准；无ABI变化，双色原生Qt/描边及复合光标继续后续。

- 最新双色双笔刷IO：`docs/brush/dual/brush-dual-color-io-design.md` / `docs/validations/brush/dual/brush-dual-color-io-validation.md`；state16/VBR 1.0/behavior13、稳定27模式名称与关闭配置、相对/平滑/PCHIP真实两笔及图片来源保持；ABI1.47原生/Qt暂拒新色彩配置，重算性能/入口/描边与复合光标继续后续。

- 最新双色双笔刷内核：`docs/brush/dual/brush-dual-color-core-design.md` / `docs/validations/brush/dual/brush-dual-color-core-validation.md`；真实两场分页/起笔before隔离合成、前后/背景/溢出与min软裁切、27贡献/工具混合/一次整笔opacity、选区/蒙版/锁及存储原子回滚，旧IO非默认明确拒绝；无ABI变化，格式/原生Qt/描边与复合光标继续后续。

- 最新双笔刷相对尺寸原生Qt：`docs/brush/ui/brush-dual-size-ui-design.md` / `docs/validations/brush/ui/brush-dual-size-ui-validation.md`；ABI1.47/state15/VBR 1.0、居中偏移/跟随与固定宽度/标称尺寸说明、关闭/重置与最高版本、真实预览和在途像素/VBR重开；解除临时入口拒绝，双色合成/溢出/描边与复合光标继续后续。

- 最新双笔刷尺寸IO：`docs/brush/dual/brush-dual-size-io-design.md` / `docs/validations/brush/dual/brush-dual-size-io-validation.md`；state15/VBR 1.0/behavior12、严格嵌套sizing/关闭与独立保留、组合时间平滑真实两笔与最高版本、ABI1.46原生准备/库包入口暂拒防旧Qt丢值；原生Qt/双色合成继续后续。

- 最新双笔刷相对尺寸内核：`docs/brush/dual/brush-dual-size-design.md` / `docs/validations/brush/dual/brush-dual-size-validation.md`；独立/相对直径偏移、跟随/固定宽度、实际主动态与独立流明确定义映射、保守工作预算和旧IO非默认拒绝；无ABI变化，格式/原生Qt/双色合成与描边继续后续。

- 最新低流量混合性能：`docs/brush/wet/brush-mix-performance-design.md` / `docs/validations/brush/wet/brush-mix-performance-validation.md`；f32覆盖精确停滞时跳过重复颜色混合、保留纸纹与工具翻转，27模式/颜色动态/双路径真实瓦片与历史对照，软边反复覆盖Release基准；无ABI或文件格式变化。

- 最新时间平滑原生Qt：`docs/brush/ui/brush-smoothing-ui-design.md` / `docs/validations/brush/ui/brush-smoothing-ui-validation.md`；ABI1.46/state14/VBR 1.0、完整位置/压力/速度/稳定距离分类、真实时间预览/在途捕获/保存重开与重置、解除准备/库包临时拒绝；收尖/双色/湿混/高级ABR继续后续。

- 最新时间平滑IO：`docs/brush/dynamics/brush-smoothing-io-design.md` / `docs/validations/brush/dynamics/brush-smoothing-io-validation.md`；state14/VBR 1.0/behavior11、f64完整关闭配置/默认旧字节、压感与双路径/图片来源真实时间笔触往返；ABI1.45准备/库包暂拒新配置防旧Qt丢字段，原生桥/Qt与收尖继续后续。

- 最新混合性能/菜单与预览：`docs/brush/wet/brush-mix-performance-design.md` / `docs/validations/brush/wet/brush-mix-performance-validation.md`；圆尖平方距离/饱和主与第二覆盖跳过、整笔像素/历史保持、工具菜单跨原生窗置顶与Qt6.5回退、预览固定正常及跨文档缓存保持，无ABI变化。

- 最新自动平滑压感：`docs/brush/dynamics/brush-pressure-smooth-design.md` / `docs/validations/brush/dynamics/brush-pressure-smooth-validation.md`；ABI1.45/state13/VBR 1.0/behavior10、统一PCHIP/纯原生图形采样、旧曲线保留/编辑自动平滑、关闭配置与重置基准，重置按钮在参数末尾。
- 最新画笔混合Qt：`docs/brush/ui/brush-blend-ui-design.md` / `docs/validations/brush/ui/brush-blend-ui-validation.md`；27模式独立工具状态/在途捕获、共享持久化及真实有色底预览，双色扩展继续后续。

- 最新画笔混合原生捕获：`docs/brush/core/brush-blend-abi-design.md` / `docs/validations/brush/core/brush-blend-abi-validation.md`；ABI1.44/16字节独立模式DTO/新prepared导出、两色与模式复制/准确票据及历史、旧入口正常兼容；Qt模式下拉/偏好/有色底预览待接，不改state/VBR。

- 最新画笔颜色混合内核：`docs/brush/core/brush-blend-design.md` / `docs/validations/brush/core/brush-blend-validation.md`；27模式/整笔贡献与before、蒙版可见灰度/透明锁精确alpha、文档坐标溶解/预算与历史；无ABI变化，工具状态/原生Qt下拉与真实预览待接，双色及自动描边随后。

- 最新重置预设：`docs/brush/ui/brush-reset-design.md` / `docs/validations/brush/ui/brush-reset-validation.md`；设置按钮/完整工厂与副本导入基准、独立有界resetState持久化/重开及原生规范化、资源恢复与在途捕获；无ABI变化，双色新字段随后接入。

- 最新时间平滑内核：`docs/brush/dynamics/brush-smoothing-design.md` / `docs/validations/brush/dynamics/brush-smoothing-validation.md`；4ms网格/位置速度响应/独立压力/拖绳与有界终点、双路径共用/累计预算/分页历史；无ABI变化，非默认旧IO拒绝，状态/VBR/Qt与收尖待接，普通画笔混合模式按用户优先级先推进。

- 已确认双笔刷扩展（待实施）：`docs/brush/dual/brush-dual-color-extension-design.md`；第二颜色、相对像素尺寸/溢出/正常双色/前后顺序，起笔两色捕获、整笔分开累积合成和两种压感边宽；含自动描边条件，已再次发令继续；补充笔刷设置“重置预设”按钮及完整持久基准。

- 最新双笔刷原生Qt：`docs/brush/ui/brush-dual-ui-design.md` / `docs/validations/brush/ui/brush-dual-ui-validation.md`；ABI1.43/state12/VBR 1.0、四子页/第二图片捕获/独立曲线动态、真实比例预览与像素历史、关闭保存及资源并集交换；库package解除临时拒绝，组合光标/平滑/湿混/ABR高级映射继续后续。

- 最新双笔刷准备桥：`docs/brush/dual/brush-dual-bridge-design.md` / `docs/validations/brush/dual/brush-dual-bridge-validation.md`；第二独占/三角色共享和关闭配置交接、发布/mutate采用、在途保留与闲置源释放、state12准确重建/真实像素历史；无ABI变化，完整Qt和资源导出继续后续。

- 最新双笔刷IO：`docs/brush/dual/brush-dual-io-design.md` / `docs/validations/brush/dual/brush-dual-io-validation.md`；state12/VBR 1.0/behavior9、两模型/完整第二动态与资源/真实两笔还原、来源并集/7169条目和状态字节预算；ABI1.42包暂拒绝新对象，原生资源生命周期和Qt继续后续。

- 最新双笔尖独立路径：`docs/brush/dual/brush-dual-path-design.md` / `docs/validations/brush/dual/brush-dual-path-validation.md`；第二直径1%间距/双流调度、独立随机字段/主频率无关、稀疏分页与终点/真实四类写入历史、预算及存储失败原子回滚；无ABI变化，旧配对保持，保存交换和原生Qt继续后续。

- 最新双笔尖覆盖核心：`docs/brush/dual/brush-dual-design.md` / `docs/validations/brush/dual/brush-dual-validation.md`；paired-dab-v1六模式/独立第二压力设备与形状散布、共享图片预算/预检与真实四类写入历史；旧state/来源拒绝非默认对象，无ABI变化，独立路径间距/IO与原生Qt继续后续。

- 最新纹理原生Qt：`docs/brush/ui/brush-texture-ui-design.md` / `docs/validations/brush/ui/brush-texture-ui-validation.md`；ABI1.42/state11/VBR 1.0、捕获目标PNG导入/纹理分类/关闭编辑、统一真实预览与像素历史、同源图片与原始来源交换/窄列可见；解除库package临时拒绝，双笔刷/平滑/湿混和ABR动态继续后续。

- 最新纹理准备桥：`docs/brush/texture/brush-texture-bridge-design.md` / `docs/validations/brush/texture/brush-texture-bridge-validation.md`；纹理独占/共用与关闭交接、在途保持/闲置源和mip释放、state11准确重建及真实像素历史；无ABI变化，Qt和公开包入口继续后续。

- 最新纹理IO：`docs/brush/texture/brush-texture-io-design.md` / `docs/validations/brush/texture/brush-texture-io-validation.md`；state11/VBR 1.0/behavior8、预设资源绑定/共享周期mip/预算预检、同源PNG与真实两笔/来源保持、6145条目边界；ABI1.41包暂拒绝新对象，原生Qt待接。

- 最新纹理内核：`docs/brush/texture/brush-texture-design.md` / `docs/validations/brush/texture/brush-texture-validation.md`；文档纸纹/笔尖局部、周期边界与面积mip/专用覆盖、资源共享预算/真实四类写入及历史；旧IO拒绝新配置，资源IO/VBR/原生Qt待接，无ABI变化。

- 最新颜色动态原生Qt：`docs/brush/ui/brush-color-ui-design.md` / `docs/validations/brush/ui/brush-color-ui-validation.md`；ABI1.41/背景DTO与旧入口兼容、两色捕获/state10/VBR 1.0/模型、颜色分类/关闭可编辑/真实预览与像素/历史；解除package临时拒绝，纹理/双笔刷/湿混与ABR动态继续后续。

- 最新颜色动态IO：`docs/brush/core/brush-color-io-design.md` / `docs/validations/brush/core/brush-color-io-validation.md`；state10/VBR 1.0/behavior7、严格线性HSV模型/完整种子/关闭频率、真实组合两笔及来源保持；ABI1.40包入口暂拒绝新对象，原生背景与Qt待接。

- 最新颜色动态内核：`docs/brush/dynamics/brush-color-dynamics-design.md` / `docs/validations/brush/dynamics/brush-color-dynamics-validation.md`；两色/线性HSV/整笔与逐印章、分页颜色贡献与透明度上限、真实蒙版/手指载色/历史；旧IO拒绝新配置，保存/VBR/原生背景与Qt待接，无ABI变化。

- 最新形状抖动原生Qt：`docs/brush/ui/brush-shape-jitter-ui-design.md` / `docs/validations/brush/ui/brush-shape-jitter-ui-validation.md`；ABI1.40/state9/VBR 1.0、形状动态子页/真实预览与捕获/完整种子关闭值、最高目录版本/图片来源往返；解除package临时拒绝，其他动态与湿混继续后续。

- 最新形状抖动IO：`docs/brush/dynamics/brush-shape-jitter-io-design.md` / `docs/validations/brush/dynamics/brush-shape-jitter-io-validation.md`；state9/VBR 1.0/behavior6、独立对象/完整种子/舍入前范围、真实组合两笔还原/图片来源保持；ABI1.39包入口暂拒绝保护旧Qt，原生Qt继续后续。

- 最新形状抖动内核：`docs/brush/dynamics/brush-shape-jitter-design.md` / `docs/validations/brush/dynamics/brush-shape-jitter-validation.md`；不可变大小/角度/圆度与下限、独立随机域/实际散布子印章、程序图片和四类写入/名义预算与历史；旧IO明确拒绝非默认新配置，状态/VBR/原生Qt继续后续，无ABI变化。

- 最新散布原生Qt：`docs/brush/ui/brush-scatter-ui-design.md` / `docs/validations/brush/ui/brush-scatter-ui-validation.md`；ABI1.39/state8/VBR 1.0、PS分类/真实统一预览与捕获笔触、完整种子/关闭值/目录最大版本、图片来源往返；解除package临时拒绝，其他动态与湿混继续后续。

- 最新散布IO：`docs/brush/dynamics/brush-scatter-io-design.md` / `docs/validations/brush/dynamics/brush-scatter-io-validation.md`；state8/VBR 1.0/behavior5、独立散布对象/完整u64种子、真实两笔还原与原始来源保持；ABI1.38包入口暂拒绝新对象保护旧Qt，原生Qt继续后续。

- 最新散布内核：`docs/brush/dynamics/brush-scatter-design.md` / `docs/validations/brush/dynamics/brush-scatter-validation.md`；确定性计数器/单轴双轴/数量抖动、输入拆分一致、共用四类写入/实际涂抹载色、展开预算预检与原子历史；旧IO明确拒绝非默认散布，保存/原生Qt继续后续，无ABI变化。

- 最新实时动态光标：`docs/brush/input/brush-live-pose-design.md` / `docs/validations/brush/input/brush-live-pose-validation.md`；ABI1.38/可复用捕获Pose、非阻塞原生几何/未知悬停压力/压力大小、Qt旋转倾斜方向/在途图片身份与DPI；其余动态/湿混继续后续。

- 最新设备动态Qt入口：`docs/brush/ui/brush-device-ui-design.md` / `docs/validations/brush/ui/brush-device-ui-validation.md`；ABI1.37/state7/VBR 1.0、形状动态参数/真实预览和旋转笔触/复制持久化、解除package临时拒绝；实时动态光标和其余动态湿混继续后续。

- 最新设备笔尖动态IO：`docs/brush/core/brush-device-io-design.md` / `docs/validations/brush/core/brush-device-io-validation.md`；state7/VBR 1.0/behavior4、独立tipDynamics/严格枚举范围/逐像素恢复、压力与图片来源共存；现有package入口暂拒绝新数据防Qt丢失，原生Qt与光标继续后续，无ABI变化。

- 最新设备笔尖动态内核：`docs/brush/dynamics/brush-device-dynamics-design.md` / `docs/validations/brush/dynamics/brush-device-dynamics-validation.md`；不可变角度来源/倾斜圆度、每印章共用几何/图片完整边界、短路径旋转与能力回退；IO非默认明确拒绝，状态/VBR/原生Qt/动态光标继续后续，无ABI变化。

- 最新不透明度压感：`docs/brush/dynamics/brush-opacity-pressure-design.md` / `docs/validations/brush/dynamics/brush-opacity-pressure-validation.md`；ABI1.36/v6/VBR 1.0、独立压力局部整笔上限/曲线最低比例、关闭数据保留、Qt真实像素与交换；其余动态/湿混/ABR动态继续后续。

- 最新独立尺寸与整笔不透明度：`docs/brush/dynamics/brush-stroke-opacity-design.md` / `docs/validations/brush/dynamics/brush-stroke-opacity-validation.md`；活动预设拥有实际大小、旧共享断言替换、低不透明度整笔上限与分页覆盖/撤销、数位笔绘画位置优先；独立不透明度压感仍待后续。

- 最新压力曲线入口：`docs/brush/ui/brush-pressure-ui-design.md` / `docs/validations/brush/ui/brush-pressure-ui-validation.md`；ABI1.35、Qt v5/形状动态与传递曲线、原生真实预览与VBR 1.0往返；解除package临时拒绝，其余动态/湿混继续后续。

- 最新压力动态IO：`docs/brush/dynamics/brush-pressure-io-design.md` / `docs/validations/brush/dynamics/brush-pressure-io-validation.md`；v5/有界验证DTO、VBR 1.0/行为2/pressure-curves、旧版本不升级/节点保留/源图共存与真实像素、准确原生准备；旧package暂拒绝压力防丢数据，ABI/Qt曲线继续后续，无ABI变化。

- 最新压力动态内核：`docs/brush/dynamics/brush-pressure-design.md` / `docs/validations/brush/dynamics/brush-pressure-validation.md`；有界不可变曲线/最低比例/精确分段编译、尺寸流量与涂抹强度共用/旧像素与在途快照；旧IO明确拒绝非默认数据，状态ABI/VBR与编辑界面继续后续，无ABI变化。

- 最新VBR来源入口：`docs/brush/vbr/vbr-source-entry-design.md` / `docs/validations/brush/vbr/vbr-source-entry-validation.md`；ABI1.34、原生完整来源交换/当前与基线PNG、待采用资源/基线mip释放、Qt v4/UUID副本/原ABR范围显式选项与切换保护、跨新仓库重开像素报告；分组扩展/完整动态湿混继续后续。

- 最新VBR来源IO：`docs/brush/vbr/vbr-source-design.md` / `docs/validations/brush/vbr/vbr-source-validation.md`；VBR 1.0三角色/原记录SHA/共享原包与当前基线PNG、批次导出/5121条目边界与Schema；旧原生接口明确拒绝防丢来源，原生Qt范围选项/分组动态继续后续，无ABI变化。

- 最新ABR用户入口：`docs/brush/abr/abr-entry-design.md` / `docs/validations/brush/abr/abr-entry-validation.md`；ABI1.33、后台整包交接/准确资源采用、VBR与ABR统一按钮/UUID整包副本、部分数量与来源报告/删除输入后绘画恢复；完整ABR动态/旧参数16位、VBR来源与湿混继续后续。

- 最新ABR整包IO导入：`docs/brush/abr/abr-import-design.md` / `docs/validations/brush/abr/abr-import-validation.md`；6/10全链编译/共享PNG与完整来源批次、单原包保存/总报告边界/顺序与像素验证；原生Qt入口、旧参数/16位与VBR来源继续后续，无ABI变化。

- 最新笔刷来源界面：`docs/brush/ui/brush-source-ui-design.md` / `docs/validations/brush/ui/brush-source-ui-validation.md`；Qt v4保存/原生目录/副本保留来源、后台导入快照与逐项兼容报告/旧结果拒绝；来源VBR导出明确未支持，ABR用户入口与批量来源继续后续，无ABI变化。

- 最新原生来源准备桥：`docs/brush/library/brush-source-bridge-design.md` / `docs/validations/brush/library/brush-source-bridge-validation.md`；ABI1.32、共享来源上下文/v4准确准备、Weak身份凭据/当前与在途独立持有、后台来源报告新校验；Qt/VBR来源/ABR入口继续后续。

- 最新原生来源引用/状态：`docs/brush/library/brush-source-state-design.md` / `docs/validations/brush/library/brush-source-state-validation.md`；来源与执行分离/不可变修订/编辑副本保留、v4显式上下文与批次原始验证；准备桥/Qt/VBR与高频来源复用继续后续，无ABI变化。

- 最新笔刷来源资源：`docs/brush/library/brush-source-design.md` / `docs/validations/brush/library/brush-source-validation.md`；完整原始来源/小型导入快照与报告、SHA身份/不覆盖保存/原字节交换；原生预设与状态/Qt/VBR接入、批量来源IO继续后续，无ABI变化。

- 最新ABR基础转换：`docs/brush/abr/abr-conversion-design.md` / `docs/validations/brush/abr/abr-conversion-validation.md`；严格单位/范围、真实编译/共享覆盖、逐叶支持与未执行报告、外部无动态alpha核对；来源保存/原生Qt入口/动态与PS核验继续后续，无ABI变化。

- 最新ABR样本关联：`docs/brush/abr/abr-association-design.md` / `docs/validations/brush/abr/abr-association-validation.md`；Pascal身份/可变前缀/sampledData严格关联、共享资源/完整来源字段；外部上游诊断像素匹配但资产不分发，参数报告/用户入口继续后续，无ABI变化。

- 最新ABR描述符：`docs/brush/abr/abr-descriptor-design.md` / `docs/validations/brush/abr/abr-descriptor-validation.md`；有界typed tree/单位与引用/显式desc接入、长度与递归拒绝；具体PS参数/样本关联/用户入口/真实授权语料继续后续，无ABI变化。

- 最新ABR分节采样：`docs/brush/abr/abr-sections-design.md` / `docs/validations/brush/abr/abr-sections-validation.md`；6/10×1/2有界分节/记录对齐/共用PackBits、原始来源元数据保留；desc执行/用户入口/16位/真实授权语料继续后续，无ABI变化。

- 最新ABR采样读取：`docs/brush/abr/abr-sampled-design.md` / `docs/validations/brush/abr/abr-sampled-validation.md`；Rust1/2采样原始/PackBits/UTF16与来源字段、整包拒绝与真实孔洞测试；尚未接用户入口，6/10/程序/16位/动态报告和授权真实语料继续后续，无ABI变化。

- 最新VBR图片资源：`docs/brush/vbr/vbr-image-design.md` / `docs/validations/brush/vbr/vbr-image-validation.md`；ABI1.31/VBR 1.0、完整PNG身份/资源清单/整包编译/多资源交接、Qt跨资源目录导入导出/UUID副本；分组/扩展/ABR/动态湿混继续后续。

- 最新图片笔尖轮廓：`docs/brush/ui/brush-outline-design.md` / `docs/validations/brush/ui/brush-outline-validation.md`；ABI 1.30、0.5阈值/孔洞与多区域/有界几何缓存、Qt当前身份与显示变换、DPI固定描边；超复杂纹理明确简化，图片悬停/动态设备/VBR图片/ABR继续后续。

- 最新图片笔尖Qt接入：`docs/brush/ui/brush-image-ui-design.md` / `docs/validations/brush/ui/brush-image-ui-validation.md`；后台PNG三种解释/捕获目标/持久v3/独立真实预览、图片硬度禁用和程序恢复；图片精确轮廓/VBR图片/ABR/高级动态仍待完成。

- 最新图片笔尖原生资源接口：`docs/brush/library/brush-resource-abi-design.md` / `docs/validations/brush/library/brush-resource-abi-validation.md`；ABI 1.29、后台PNG导入/共享资源目录、v3准确准备、闲置修订释放/重建及在途资源保留；Qt图片控件/VBR图片/ABR仍待接入。

- 最新图片笔尖身份/状态：`docs/brush/library/brush-resource-state-design.md` / `docs/validations/brush/library/brush-resource-state-validation.md`；覆盖不可变PNG身份/v3哈希引用/后台资源恢复与整包编译，旧状态保留；原生资源目录/Qt/VBR图片/ABR仍待接入。

- 最新图片笔尖资源IO：`docs/brush/library/brush-resource-design.md` / `docs/validations/brush/library/brush-resource-validation.md`；SHA-256内容身份/不覆盖原子PNG、8/16位覆盖转换与量化标记、Weak缓存/活跃mip预算，资源ID原生库/Qt接入与VBR图片/ABR仍待后续。

- 最新 Qt / 原生笔刷修订桥：`docs/brush/library/brush-catalog-bridge-design.md` / `docs/validations/brush/library/brush-catalog-bridge-validation.md`；ABI 1.28、后台状态迁移/独立编译池/准确准备记录、Rust起笔等待与Qt共享库、隔离原生预览/有效状态保存；图片资源/ABR/动态/湿混继续后续。

- 最新原生笔刷库 ABI：`docs/brush/library/brush-catalog-abi-design.md` / `docs/validations/brush/library/brush-catalog-abi-validation.md`；ABI 1.27、独立句柄/准确修订/原子目录操作、捕获预设的真实异步笔触及生命周期；Qt 镜像迁移与图片资源接口仍待接入。

- 最新 Rust 笔刷库模型：`docs/brush/library/brush-catalog-design.md` / `docs/validations/brush/library/brush-catalog-validation.md`；验证编译/不可变修订、原子目录操作、旧预设兼容、1024预设/共享资源预算；原生库 ABI 与 Qt 完整迁移仍待接入。

- 最新图片笔尖内核：`docs/brush/core/brush-mask-design.md` / `docs/validations/brush/core/brush-mask-validation.md`；不可变覆盖资源/mip、每印章采样计划、孔洞/非方形/旋转完整边界、资源快照与四类写入共用，图片 UI/VBR 资源/ABR 尚待接入，无 ABI 变化。

- 最新笔尖光标偏好：`docs/brush/ui/brush-cursor-settings-design.md` / `docs/validations/brush/ui/brush-cursor-settings-validation.md`；实际/简化轮廓、中心/悬停/线宽/颜色、全局保存不进入 VBR、多文档笔刷状态同步与单一写入者移交，无 ABI 变化。

- 最新 VBR 程序子集：`docs/brush/vbr/vbr-procedural-design.md` / `docs/validations/brush/vbr/vbr-procedural-validation.md`；ABI 1.26、Rust 有界 ZIP/JSON 与原子 IO、程序预设 UUID/能力/单位、后台导入导出、冲突副本与整包发布；资源/ABR 暂未实现。

- 笔刷系统：`docs/planning/brush-system-design.md` / `docs/validations/planning/brush-system-validation.md` / `docs/brush/vbr/vbr-format-design.md`；总体分阶段实施，ABR/VBR 等后续范围见验收计划。
- 最新程序笔尖：`docs/brush/core/brush-tip-design.md` / `docs/validations/brush/core/brush-tip-validation.md`；ABI 1.25、共用编译覆盖、硬度/圆度/角度/流量与压感、旧圆笔兼容、预设 v2 原子恢复、PS 分类面板/真实预览/旋转椭圆光标。

- 最新 Ctrl 选层与临时移动：`docs/tools/ctrl-layer-pick-design.md` / `docs/validations/tools/ctrl-layer-pick-validation.md`；ABI 1.24、异步像素命中/原子移动/锁与蒙版、图层搜索首次点击焦点、自由变换共用紧凑输入框。

- 最新图片与变换：`docs/tools/transform-design.md` / `docs/validations/tools/transform-validation.md`；ABI 1.23、嵌入原始图片模型、Ctrl+T/八点控制/透视与网格、组/多选/选区像素原子变换、会话撤销重做、栅格化与 ORA v5 原图恢复。

- 最新液化历史：`docs/tools/liquify-history-design.md` / `docs/validations/tools/liquify-history-validation.md`；ABI 1.22、整笔触会话撤销/重做、设置步数与校验暂存、历史域和滚动恢复、Ctrl+Z/Ctrl+Shift+Z、[ ] 液化尺寸。

- 最新液化性能：`docs/tools/liquify-performance-design.md` / `docs/validations/tools/liquify-performance-validation.md`；连续预览非屏障、瓦片批处理与相邻取样缓存、共用选区覆盖缓存保留坐标、紧凑复选框与标题分隔，无 ABI 变更。

- 最新搜索与快捷键：`docs/layers/layer-search-tools-design.md` / `docs/validations/layers/layer-search-tools-validation.md`；组层级搜索、文本编辑与画布焦点区分、快捷键恢复、液化仅滤镜入口及 Enter/Esc、图层拷贝粘贴仅快捷键。

- 最新液化：`docs/tools/liquify-design.md` / `docs/validations/tools/liquify-validation.md`；ABI 1.21、滤镜入口、工具详情栏与图标、稀疏位移场与原始取样、事务历史、保存参数、文档锁定与浮窗隐藏恢复。

- 最新图层操作：`docs/layers/layer-actions-design.md` / `docs/validations/layers/layer-actions-validation.md`；ABI 1.20、右键/主菜单、原子复制粘贴/子树合并、缩略图编辑焦点、Delete 与文本焦点保护、下边缘拖放修复。

- 最新组穿透：`docs/layers/group-pass-through-design.md` / `docs/validations/layers/group-pass-through-validation.md`；ABI 1.19、组专用混合模式 27，子层对组外下层合成，opacity/fill/蒙版与剪贴、预览/撤销/ORA；新建组默认穿透、菜单置于正常上方。

- 最新吸色取色环：`docs/tools/eyedropper-ring-design.md` / `docs/validations/tools/eyedropper-ring-validation.md`；按下拖动显示新旧颜色半环与灰色外圈、I/Alt/数位笔共用、显示开关、无 ABI 变更。

- 最新涂抹：`docs/brush/wet/smudge-design.md` / `docs/validations/brush/wet/smudge-validation.md`；ABI 1.18、独立 R 工具复用笔刷、强度/压感/合成取样/手指绘画、最低 1% 间隔；参数说明统一悬停 1 秒。

跨平台绘画应用：Rust 内核 → 稳定 C ABI → C++20 / Qt 6.5+ UI；Python 插件最后实施。

- 最新魔棒：`docs/tools/wand-design.md` / `docs/validations/tools/wand-validation.md`；ABI 1.17、四种组合/七种取样尺寸/默认容差 10/AA/连续/所有图层、灰度掩码与真实轮廓、历史和持久化。

- 最新油漆桶：`docs/tools/bucket-design.md` / `docs/validations/tools/bucket-validation.md`；ABI 1.16、不透明度/容差（默认 0）/AA/连续/所有图层取样、选区与锁、单条原子历史、鼠标/数位板和浮动画布。

- 最新选区绘画性能：`docs/tools/selection-performance-design.md` / `docs/validations/tools/selection-performance-validation.md`；框选/套索按行覆盖与有界缓存、画笔和橡皮共用、保留 AA/羽化/组合像素结果，无 ABI 变更。

- 最新共用选区与套索：`docs/tools/lasso-selection-design.md` / `docs/validations/tools/lasso-selection-validation.md`；ABI 1.15、自由/多边形套索、四种组合与羽化/AA 共用、路径索引与历史/ORA、鼠标和数位板采集。

- 最新菜单悬停与多选建组：`docs/interface/menu-hover-grouping-design.md` / `docs/validations/interface/menu-hover-grouping-validation.md`；ABI 1.14、统一指针预览、原生菜单完整坐标转发、首行跨边回归、多选一次建组与撤销。

- 最新面板折叠尺寸保持：`docs/interface/panel-fold-sizing-design.md` / `docs/validations/interface/panel-fold-sizing-validation.md`；布局 v6 可选 retainedWidths，折叠仅压缩自身、工具栏宽度优先、浮窗收窄、相邻宽高与展开尺寸保持、显式调整解除保持。

- 最新菜单分类与图层批量修复：`docs/layers/menu-layer-batch-design.md` / `docs/validations/layers/menu-layer-batch-validation.md`；ABI 1.13、共享悬停侧向子菜单、组删除双选择、原子多选删除/拖放、拖入剪贴链自动继承、图层面板首开异步创建。
- 最新遗留修复与吸色：`docs/planning/deepseek-recovery-design.md` / `docs/validations/planning/deepseek-recovery-validation.md`；ABI 1.12、I 独立吸色与 Alt 临时吸色、DPI/视口坐标修复、剪贴链插层/基底变化、缩略图独立通知、剩余面板高度和恢复拖动。GPU/液化/.vbr/图层复制合并本轮暂缓。
- 当前阶段与验证命令：`README.md`、`docs/planning/roadmap.md`。
- 最新真实历史标签：`docs/document/history-labels-design.md` / `docs/validations/document/history-labels-validation.md`；ABI 1.10、真实工具名称/图标、裁剪后不显示伪初始状态；20ms 逐条历史回放保留。
- 最新面板细化与混合悬停预览：`docs/interface/panel-refinement-design.md` / `docs/validations/interface/panel-refinement-validation.md`；ABI 1.9、浮窗拆分保留尺寸、颜色即时交换、共享紧凑控件、向外半透明描边、数位板悬停轮廓。替代 HEX 双击输入与延迟颜色交换。
- 检出/构建规则：`.gitattributes` / `docs/platform/build-reproducibility.md`；文本 LF，生成头严格字节检查，每个工作树独立 Cargo target，tools/check.py 包含真实 Git 检出测试。
- 架构、UI 停靠与兼容性：`docs/platform/architecture.md`。
- 正式 C ABI：`core/crates/paint-ffi/src/abi/api.rs` / `ui/include/paint_api.h`；契约在 `contracts/ffi/abi.md`。
- `contracts/ffi/history/paint-api-v1.draft.h` 仅为历史设计；FFI 方案/验收在 `docs/platform/ffi-design.md`。
- 当前文件 IO 与布局 v2 清理：`docs/document/io-design.md` / `docs/validations/document/io-validation.md`；ABI 1.2 文件任务、Qt 文件菜单，笔记已退役。
- 当前实现：`core/crates/paint-core/src`、`core/crates/paint-ffi/src`、`ui/src`、`ui/qml` 与对应测试。
- 最新 UI 设计/验收：`docs/brush/ui/qml-ui-design.md` / `docs/validations/brush/ui/qml-ui-validation.md`；用户指定 QML，旧 Widgets 设计已被取代。
- 最新参考图 UI / 停靠：`docs/brush/ui/reference-ui-design.md` / `docs/validations/brush/ui/reference-ui-validation.md`；无边框紧凑工作区、原生浮窗、工具条拖放、布局 v4。用户要求每个模块验证后 commit 并推送 origin/master，持续推进；Python 插件最后。
- 最新自由停靠：`docs/interface/flexible-docking-design.md` / `docs/validations/interface/flexible-docking-validation.md`；画布、面板组和工具条共用主窗/浮窗分割树，空分支收缩，布局 v5，替代固定左右列；只有一个可移换父项的主画布实例。
- 最新文档工作区 / 面板列：`docs/document/document-workspace-design.md` / `docs/validations/document/document-workspace-validation.md`；多个独立 PaintCoreClient 文档、画布浮窗标签、工具列统一图标折叠、实际原生窗口拖动、布局 v6。画布与工具面板互不接收，主窗保留 #17191C 文档留空区。
- 最新常驻图标面板列：`docs/interface/persistent-panel-rails-design.md` / `docs/validations/interface/persistent-panel-rails-validation.md`；每个图标独立打开、允许多开、空白与失焦不收回、最近点击置顶、侧边导航锚定与独立缩放、浮动列背景/高度及归位状态保留。替代此前唯一临时窗及空白关闭。
- 最新工具列与画布交互：`docs/interface/workspace-tools-design.md` / `docs/validations/interface/workspace-tools-validation.md`；工具整列停靠、36–72px 宽度与拆分缩回、仅鼠标磁吸/外侧提示、折叠按钮拖动、布局像素位置保持、菜单置顶、画笔/橡皮独立大小与固定预览、颜色单双击、轮廓与跨面板空格平移；顶部 #05080A 75%，外描边 #63666B。
- 最新面板分组与统一渲染：`docs/interface/panel-categories-design.md` / `docs/validations/interface/panel-categories-validation.md`；外侧整列停靠、内侧同组并列，布局 v6 可选 row 角色；图标按组整窗展开与居中分隔，双击收回不展开列，浮动列仅顶层栏透明，导航红框与共享内容/调色板，历史面板无撤销/重做按钮（逐条跳转保留）。
- 最新面板状态与信息栏：`docs/interface/panel-presentation-design.md` / `docs/validations/interface/panel-presentation-validation.md`；同图标再次点击收回，独立宽高/侧边位置随布局 v6 保存；描边 #1B1C1F、小标签圆角、顶部 #05080A（浮动 75% 不透明度），紧凑底栏与中心缩放/适配取代工具条适合窗口按钮。
- 最新图标列 / 临时面板：`docs/interface/icon-rail-design.md` / `docs/validations/interface/icon-rail-validation.md`；主工作区边缘优先归位、唯一图标折叠模式与状态继承、图标直接拖出、侧边临时窗切换/滑动/空白关闭、单画布浮窗一行标题。
- 最新窗口恢复 / 边缘吸附：`docs/interface/window-recovery-design.md` / `docs/validations/interface/window-recovery-validation.md`；最小化恢复无边框与最大化状态、唯一临时面板外部输入/失焦关闭、统一 24px 最近边吸附和窄边发光、默认双列布局，参考图预设已删除。
- 最新任务栏 / 原生标题残留：`docs/interface/taskbar-window-design.md` / `docs/validations/interface/taskbar-window-validation.md`；显式系统最小化能力、主窗非客户区绘制过滤、恢复后真实合成像素回归、灰黑细分隔线。
- 最新菜单底层圆角模糊：`docs/interface/menu-blur-layer-design.md` / `docs/validations/interface/menu-blur-layer-validation.md`；独立非交互原生窗口、HostBackdropBrush 与合成视觉圆角裁剪、最小化前隐藏、最大化拖动恢复普通窗口并跟随鼠标。
- 最新几何选区：`docs/tools/selection-design.md` / `docs/validations/tools/selection-validation.md`；ABI 1.8、有界几何、笔刷/擦除/蒙版限制、历史与 ORA v4、QML 静态轮廓。
- 最新画笔库 / 设置关联：`docs/brush/ui/brush-library-design.md` / `docs/validations/brush/ui/brush-library-validation.md`；共享活动预设、真实异步引擎预览、直径/间距单位转换、后台保存；高级笔尖未实现。
- 当前任务/CPU 视口与异步 ABI：`docs/platform/task-render-design.md` / `docs/validations/platform/task-render-validation.md`，Qt 不再使用 C++ 绘画队列或整张文档缓存。
- 最新绘画性能修复：`docs/platform/painting-performance-design.md` / `docs/validations/platform/painting-performance-validation.md`；有界 Weak 瓦片缓存、稀疏输出、QML 图层通知与原生性能基准。
- 最新剪贴/并排蒙版/预览修复：`docs/layers/clipping-design.md` / `docs/validations/layers/clipping-validation.md`；ABI 1.7，Alt 层间点击、动态连续同级剪贴、ORA v3、边缘 LOD 正确采样。
- 最新蒙版/拖放/默认背景：`docs/layers/masks-design.md` / `docs/validations/layers/masks-validation.md`；ABI 1.6，图层/组蒙版、拖放排序/归组、深度、隐式白色“背景”、中文命名、ORA 原始树与兼容预览。
- 最新图层组/透明背景：`docs/layers/groups-design.md` / `docs/validations/layers/groups-validation.md` / `docs/document/transparency-design.md`；ABI 1.5、隔离嵌套、锁继承、结构历史/子树删除、嵌套 ORA、QML 折叠/浮窗与棋盘格。蒙版已在 ABI 1.6 完成。
- 图层控制：`docs/layers/layers-design.md` / `docs/validations/layers/layers-validation.md`；ABI 1.4、27 种混合、三种锁、位置/填充、可见行后台预览、ORA 扩展，fx 未实施。
- 最新暂存盘配置：`docs/document/storage-settings-design.md` / `docs/validations/document/storage-settings-validation.md`；paint-storage、ABI 1.3、后台检查与租约清扫，设置重启生效。
- 最新文档分页：`docs/document/document-paging-design.md` / `docs/validations/document/document-paging-validation.md`；256MiB 为像素驻留缓存，冷页磁盘后备，快照/历史保持不可变身份。
- 最新历史存储：`docs/document/history-storage-design.md` / `docs/validations/document/history-storage-validation.md`；大笔触无损编码与磁盘暂存，不再按内存历史预算拒绝笔触。

约束：Rust 不依赖 Qt；UI 只经 PaintCoreClient 使用 C ABI；Python 不访问 Qt/UI 线程。
任何新模块先写设计与验收条件，再实现。禁止用空实现或假成功代替未支持的功能。
所有行为修改补对应层回归测试（Rust / Qt Test / pytest）；尚未存在的层不创建虚假测试。
仓库统一验证入口：`python tools/check.py`（默认含 Qt，--qt-prefix 可指定 SDK）；--core-only 仅表示核心部分验证，不允许跳过失败项后报告完整通过。
ABI 破坏性修改升主版本；生成头由 cbindgen 管理，不手工修改生成物。
只有 paint-ffi 允许经过审查的 unsafe；caller-memory 读写集中在 pointers.rs，回调注明安全约束。
不要把开发机绝对路径写入构建配置；不要提交构建输出或工具链。
