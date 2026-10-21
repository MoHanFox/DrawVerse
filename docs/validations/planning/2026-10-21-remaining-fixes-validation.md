# 原始清单与 UI 回归统一修复验收

## H1：多类长任务共享结果历史

自由变换、整图翻转、栅格化、油漆桶接入PagePool结果引用，保持选区、置入源及原始瓦片身份；未托管/跨池或元数据超预算回退原编码。选区按实际内存计账。核心结果六项/历史五项/直接跳转三项/进度一项/公开变换七项/原生三项，软件和D3D11各九行真实Qt回归、四ABI/严格Clippy/格式/生成头/完整Release通过。75MiB固定样本热物化中位275.930→0.129ms，未包含界面显示或冷IO；标准drawverse.exe更新于17:16，ABI1.75/state45保持。详见[历史验收](../document/history-storage-validation.md#2026-10-21长任务共享结果历史扩展)。H1仍保留冷读取时延、液化会话字段及完整8K/16K验收；T2复杂回退、M1持续手感、P1首次QML/资源准备继续待办。

2026-10-21。[范围及当前状态](../../planning/2026-10-21-remaining-fixes-design.md)。

## 建立清单

逐项对照用户《新建 文本文档 (4).txt》、实现设计及验收，区分已完成、部分优化、只完成算子和未归因回归。U1–U4 的原失败见 welcome-workspace-validation，U2 独立复核仍失败，见 collapsed-panel-flyout-merge-validation。未把这些失败归类为既有问题。

本提交仅建立追踪入口，无运行时修改；未宣称上述待办已完成。后续在这里追加每项根因、改动、验证及限制。

## U3：快捷键列表旧断言

根因是测试仍要求早期26条，当前35条包含图层复制/粘贴、复制、合并、成组、删除、全选/取消/反选、移动工具九项。核对Main快捷键后，测试验证九项名称与键位、所有行三列非空及名称唯一、模型与实际行数一致，保留渲染像素和打开/关闭验证；没有删除产品快捷键迁就旧断言。

软件和D3D11下 `editMenuOpensKeyboardShortcutList` 各通过（含初始化清理3 passed），零QML警告。日志 artifacts/remaining-u3-{software,d3d11}.txt。Release/全部Qt及四ABI目标构建通过，生成头一致；无生产代码或ABI变化。其余待办保持。

## U2：异步图层面板创建

PanelContent异步创建LayersPanel期间，自定义属性尚未全部就绪；LayerLock内部Connections提前连接selected/kind时收到不存在信号警告，Instantiator的菜单项提前读取selected.mask。锁图标改为控件自身的属性变更处理，图标尚未创建时不请求绘画；菜单在selected尚未就绪时禁用。没有屏蔽Connections警告或改成同步加载。

`panelTabsRetractWithoutExpandingRailsAndKeepSharedPresentation` 在软件和D3D11通过，原7条QML警告消失，保留所有折叠、跨窗口、源栏状态断言；日志 artifacts/remaining-ui-probe.txt、remaining-u2-d3d11.txt。Release与Qt目标构建通过，ABI/state不变。退出仍有4个异步创建对象诊断，由U5继续跟踪；没有把该诊断称为已解决。

## U4 窄笔刷设置面板（2026-10-21）

方向盘固定横排的最小宽度使左列设置超宽，大小输入框点击落入画布。BrushTipShapeControl 在宽度小于220px时改为纵排；ScrollView 最小/隐式宽度为0，由外部面板分配，消除独立创建时的内容宽度循环。标准预览固定大小/不透明度，旧测试改验此契约；不同预设各自保留参数，切换默认压力圆笔验12px半径，再切回细线笔验20px半径。

原 `brushLibrarySettingsAndEnginePreviewsShareSelectionAcrossWindows` 在软件与D3D11各通过一次，保留真实40px输入、跨窗选中同步、标准预览/间距变化、实际透明像素、单历史及保存重开验证。日志：`artifacts/pixel-slider-ui-software.txt` / `artifacts/pixel-slider-ui-d3d11.txt`。方向盘两项软件通过；D3D11随S1最终共享回归记录。完整Release构建通过。

同批附加的旧液化事务用例软件通过、D3D11在3953行等待CPU frameRevision失败；未删除断言或标记通过，GPU常驻显示路径不保证逐笔CPU图像，该用例仍需专门分析。U1/U5及算法剩余项目保持待办。

## O1 新描边可操作链路

核心按实际主体印章复刻四通道max差集；Qt删除旧算法选择及独立建路径调度，接入允许溢出、实际笔触与预览、state45/VBR37及旧设置迁移。核心新5/旧11/内部3、IO7、原生1、Qt workflow软件/D3D11各8行、UiTests各3项、四ABI、严格Clippy/格式及Release通过。详见[描边验收](../brush/core/brush-stroke-replay-outline-validation.md)。原生旧Path/Sdf仍为兼容/参考执行保留，不将O1的“删除旧算法代码”标为完成；T1/T2/M1/H1/P1及U1/U5仍待。

## S1 大范围像素滑条

共用有符号对数坐标与指数反函数，跨度≥512px时启用，保留1px加减与精确输入。软件/D3D11各7项通过，含真实输入/鼠标/键盘及笔刷、液化模型双向同步；Release构建通过。详见[设计](../../interface/pixel-slider-exponential-design.md)与[验收](../interface/pixel-slider-exponential-validation.md)。U4共享方向盘两项D3D11也通过。U5透明交换链提示在3行真实拖放仍存在，默认alpha缓冲配置并未消除提示，因此撤回该无效候选，不称其修复。

## O1 旧原生执行清理完成

删除Path/SDF实际执行、缓存和核心选择接口；旧格式只在IO严格解析，绑定时产出Replay，规范目录写回state45。核心19、纯算子7、IO新旧迁移7/共享24、原生三模型1、软件/D3D11工作流各6行及UI各3、四ABI、严格Clippy/格式/生成头与Release均通过。旧主程序正在运行而链接失败，保留用户进程，新产物为 `build/qt/bin/Release/drawverse-next.exe`。详见描边验收末节；O1标完成，其余T1/T2/M1/H1/P1、U1/U5仍待。

## T1 普通根层局部选区常驻预览

一次捕获原始选中像素/余像素，原位保持精确原图；持续变换只改CPU几何并由Qt GPU显示，确认按原始完整源生成结果。支持像素半透明、羽化及图层偏移；图层属性opacity/fill非1、自身蒙版、组内/多目标仍归T2。核心12/公开7/原生2、软件/D3D11各10行及硬件Warp独立复跑3次、四ABI/Clippy/格式/生成头/Release通过。

4096×2160约256px套索六次Warp核心中位334.561→1.556ms，最终摘要相同；确认从0.319→61.887ms，因为取样转移到确认阶段。该计时不含真实显示/首次资产，不能写成交互时延。截图夹具的ready生命周期和历史深度作用域两次失败均已定位改正，未删产品行为断言。详细证据和限制见[变换验收末节](../tools/transform-resident-preview-validation.md)。新Release程序仍为drawverse-next.exe。T2、M1/H1/P1、U1/U5与实体持续性能继续待办。

## U6 文件成功文字残留

移除通用“文件操作完成”，文件成功清空旧通知，失败与保存关闭保护保持。白底/透明真实新建直接关闭、实际保存ORA再关闭、打开不存在文件报错，软件/D3D11各2行通过；Release构建通过。D3D11退出仍有1个异步对象诊断，继续归U5。见[欢迎页验收末节](../interface/welcome-workspace-validation.md)。

## T2 第一项：组内完整普通目标

移动和变换共用可消去祖先分割，变换覆盖不可变原层时保留显示parent；两级Normal/PassThrough祖先、链接蒙版、半透明目标及静态兄弟可常驻，真实结构不改。核心变换13/公开7/共享移动3/液化3、原生3、Qt复合变换软件/D3D11各7行及共享移动7/4K选区2通过；硬件焦点混合批次未能激活窗口，独立同例通过，原失败保留。四ABI、严格Clippy/格式/生成头/Release通过。

2000px带灰度链接蒙版、两级Normal组的六次Warp核心中位18335.863→5.087ms，确认355.272→3231.376ms，最终摘要相同；不含首次资产或UI显示。标准 `build/qt/bin/Release/drawverse.exe` 已可写且更新。详见[变换验收末节](../tools/transform-resident-preview-validation.md)。固定/祖先蒙版、非连续等T2剩余项及M1/H1/P1、U1/U5保持待办。


## U5：隐藏窗口孵化和浮窗透明 surface

PanelContent 检查真实窗口可见与客户端关闭状态；卸载仍在 Loading 的内容时先完成已启动的 deferred 控件，再卸载。LayersPanel 自身完成后才填充子委托，浮动列展开/折叠保持透明 surface。软件及D3D11的欢迎生命周期、白底/透明保存关闭、原生拖动合并三来源共各6行通过；显式检查关闭后的Loader不激活，并将残留创建/cancel/alpha诊断作为测试失败，零上述诊断。完整Release通过，标准drawverse.exe已更新。见欢迎页验收及 artifacts/batch-workflow-{software,d3d11}.txt；U1与原始性能项目继续推进。


## T2：组内局部选区常驻

两级Normal/PassThrough下的局部套索接入共享祖先显示分割，核心预览14/公开7/共享移动3/液化3、软件与D3D11各5行4K实际预览通过；六种确认插值、历史/取消及晚启用与CPU一致。固定4K六次Warp中位345.593→1.528ms，最终摘要一致，仅核心计时；完整Release/四ABI/严格Clippy/格式与生成头通过。详见变换验收末节，固定/祖先蒙版、非连续等复杂T2范围及首次资产持续时延仍待。


## U1：浮动画布跨窗口纹理源解绑

先纠正原回归对Ctrl+Y的旧假设并等待真实缩放终值，以实际静止中键停止惯性；继而D3D11复现跨窗ShaderEffectSource仍持有旧窗口，画布标签激活后留在主窗口。CanvasPane先隐藏解绑、再重设parent，完整工作流软件及D3D11均通过；原有鼠标/像素/撤销重做/手写笔/窗口身份和关闭保护断言保持。日志canvas-window-{software,d3d11}.txt，有两条测试故意发送窗外捕获鼠标的QtTest提示，未屏蔽；GPU窗口归属诊断消失。首开图层异步软件/硬件心跳20/27ms，原80层测试通过。Release已更新，详见欢迎页验收。


## P1：state45 等价目录误判修复

等价比较上界44与已支持45不一致，导致原生回传及文档间偏好反复完整恢复。补齐45，保留46/非法低版本拒绝与真实参数变更恢复。确定性回归软件/D3D11各通过；真实489项约2MB目录，两后端各80次选择的本地/双文档/跨工具隔离通过，双文档目录重建16→0，热轮GUI最大间隔16/23ms，双文档17/18ms。首轮仍374/367ms，冷restore168/179ms，不能标P1全完成。Release已更新；证据与计时限制见笔刷切换延迟验收。

## U5 补充收尾与批量共享回归

额外原分类双击回归发现直接销毁仍遗留2个创建计数（artifacts/batch-panels-software.txt），已在 PanelContent.Component.onDestruction 完成 Loading 后卸载。原测试增加 failOnWarning，不屏蔽或等待：最终软件与D3D11各2项（快速收起分类、跨窗共享笔刷设置）通过，欢迎生命周期/白底透明关闭/原生拖动三来源/state45等价目录各7行通过，无残留/cancel/alpha或QML警告。日志 artifacts/panel-destroy-final-{software,d3d11}.txt 与 panel-destroy-shared-{software,d3d11}.txt。

共享复合移动与变换回归两后端各14行通过（artifacts/batch-shared-transform-{software,d3d11}.txt）。最终完整Release与生成头检查通过，标准 build/qt/bin/Release/drawverse.exe 更新于2026-10-21 14:32。未跑全量check、跨平台或实体持续输入；T2复杂回退、M1/H1及P1冷准备继续保留待办。

## P1 启动配置准备移出GUI

完整Qt校验及快照编码移入BackendWorker，传递无QObject的不可变结果，发布前拒绝陈旧共享偏好、关闭后丢弃，非法候选保留现状。新跨线程配置/工具/颜色/光标/失败原子性及共享工作流各11行、UI各3项在软件/D3D11通过；完整Release/生成头通过。489项完整同步恢复187/178ms，已准备配置GUI应用0.171/0.177ms，首轮其他首次显示心跳仍469/361ms，P1只更新部分完成。证据与撤回异步面板候选见笔刷切换验收末节；ABI/state保持。

## 2026-10-21 液化会话位移场历史分块读写

会话字段每块改为4KiB有界读写，保持原小端浮点编码、完整校验和及两侧有限/范围校验。读取仅为需要恢复的一侧分配位移数组，未使用的一侧仍逐值校验。不是字段引用历史迁移，也没有新增无预算缓存。

- 核心液化单元16通过/2手工项忽略、公开液化11通过/1忽略；新增逐位字节兼容、截断/非法浮点与有效校验和但未使用侧损坏的原子回滚测试。
- 显式Release字段负载：64块、每个检查点前后共4MiB、4检查点、五次中位数。六次直接跳转累计跨20条：内存174.236→109.497ms，磁盘后备274.228→174.277ms；单检查点提交内存6.743→5.938ms、磁盘17.187→17.591ms，磁盘写入基本持平。磁盘处于OS热缓存，且只测字段处理，不代表UI、完整像素重采样或物理冷盘速度。
- 原生液化1、软件Qt液化发布/事务2及直接历史1通过；D3D11液化发布/常驻网格2及直接历史1通过。D3D11事务用例在3955行等待focus模式像素帧修订失败，两次复现；旧build/qt的改动前程序同项同位置也失败，记录为既有问题，不能称该项通过。
- 四ABI、paint-core/paint-ffi所有目标严格Clippy、fmt、完整Release及Debug应用构建通过。标准输出build/bin/Release/drawverse.exe；ABI1.75/state45不变。

日志：artifacts/liquify-fields-{before,after,core,public,native,clippy,abi}.txt及对应ui/workflow/build日志。未验收完整8K/16K、冷盘或实体持续交互。套索液化、翻转历史、结束编辑焦点及变换首次资源准备按用户新增问题继续处理。

## 2026-10-21 编辑结束快捷键恢复

用户报告结束液化/变换仍需点击画布。新增editingCompletionRestoresShortcuts在实际选项控件持有焦点、另一原生窗口激活时结束会话，旧代码transform-cancel-main稳定失败于宿主未激活。修复保留待恢复标记，等busy/窗口/可见状态满足后完成焦点恢复。

最终用例实际修改变换几何或液化像素后，通过选项确定/取消，主窗口/浮动文档共8行；结束后不调用focus辅助、不点击画布，直接E/B/M均成功。软件及D3D11各8行与欢迎页文档生命周期1行通过，零QML警告。日志artifacts/edit-focus-before.txt及edit-focus-final-{software,d3d11}.txt。完整标准Release构建通过，build/bin/Release/drawverse.exe更新；ABI1.75/state45保持。本项不宣称其他性能问题已完成。

## 2026-10-21 首拖CPU代理准备提前至进入工具

新增Actor回归确认：带羽化套索的BEGIN完成前，真实拖动sourceId的资产已缓存，和初始精确图sourceId不同；首次UPDATE直接返回同一Arc字节身份，三次初始/变形往返均复用两套，CANCEL清空。该回归1项、核心变换预览14项、paint-core/paint-ffi所有目标严格Clippy及fmt通过。

实际4K Qt套索affine/Warp及两级Normal/PassThrough五行、原工具完成快捷键一行，软件/D3D11各6行通过，含静止余像素、纹理身份、取消及文档历史。完整标准Release/生成头、四ABI通过，输出build/bin/Release/drawverse.exe。日志artifacts/transform-preparation-{actor,core,clippy,software,d3d11,abi,build}.txt。

没有将缓存命中或核心计时称为首拖端到端延迟；Qt显卡纹理首上传、复杂CPU回退及实体持续性能仍需验收。ABI1.75/state45保持。

## 2026-10-21 翻转名称及共享冷页历史恢复

图像菜单及自由变换纯水平/垂直反射分别记录kind39/40和中文名称，局部历史按相邻确认几何、永久历史按初末几何判断；复合变化保持自由变换。ABI1.76添加常量，DTO及state45/VBR不变，Qt最低1.76。

共享Inline/Paged历史先按Tile身份去重，冷页仅校验原编码checksum，不解码、不触碰LRU；恢复只换不可变引用，显示时再按需读取。保持两侧错误校验与失败原子性。新测试验证同一冷页在before/after只验证一次、验证不加载/淘汰像素、后续实际读取和损坏拒绝。既有桶历史失败测试仍注入不再使用的独立编码写入，改为1块驻留时注入真实PagePool写失败，像素/修订/历史回滚及恢复undo/redo保持。

固定1024密集图、256瓦片、驻留16、5次中位数，六次历史往返核心368.722→214.985ms。磁盘后备处于OS热缓存，仍有完整校验IO，不含GUI/显示或冷盘时间，不承诺任意大图瞬时切换。日志artifacts/flip-history-{before,after}.txt。

Release核心单元192通过、4手工项忽略、1项耗时mix测试过滤；公开paging3/history_storage3/transform7/liquify11通过（液化1既有手工项忽略），原生整图翻转及变换各1，严格Clippy/fmt/生成头通过。实际Qt图像/自由变换×水平/垂直四行，均反复直接历史恢复精确像素；4K选区变换五行及结束工具快捷键一行，软件/D3D11各10行通过。四ABI及完整标准Release通过，build/bin/Release/drawverse.exe更新。

首次Debug宽范围单元运行发现上述旧桶断言及新冷页测试在把受损页读热后的测试顺序错误；均修正并在Release回归通过。Debug耗时mix用例主动停止，未称全量通过。最终日志artifacts/flip-history-core-release.txt、flip-history-public.txt、flip-history-native.txt、flip-history-transform-native.txt、flip-history-final-{software,d3d11}.txt、flip-history-abi.txt及final-build/clippy/header-check。未跑跨平台、实体连续时延及16K整图。
