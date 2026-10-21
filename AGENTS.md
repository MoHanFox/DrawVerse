# DrawVerse

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
