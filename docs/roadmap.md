独立笔刷尺寸与整笔不透明度：[设计](brush-stroke-opacity-design.md) / [验收](brush-stroke-opacity-validation.md)。每预设大小、低不透明度整笔上限、数位笔位置保护已完成；独立不透明度压力控制继续后续。

压力曲线入口：[设计](brush-pressure-ui-design.md) / [验收](brush-pressure-ui-validation.md)。ABI1.35、Qt v5与VBR1.3接入；其余动态/湿混继续后续。

# 模块顺序与验收

压力动态IO：[设计](brush-pressure-io-design.md) / [验收](brush-pressure-io-validation.md)。v5状态/VBR1.3及准确准备已接有界曲线；继续ABI能力与PS曲线编辑界面，再推进其余动态湿混。

压力动态：[设计](brush-pressure-design.md) / [验收](brush-pressure-validation.md)。尺寸/流量压力曲线与最低比例已接统一Rust执行；继续状态/原生与VBR保存、PS分类编辑，再推进其余动态及湿混。

VBR来源入口：[设计](vbr-source-entry-design.md) / [验收](vbr-source-entry-validation.md)。ABI1.34，来源资源交换/原始包范围选项/跨仓库保存与报告已接原生Qt；继续分组扩展及高级动态湿混。

VBR来源IO：[设计](vbr-source-design.md) / [验收](vbr-source-validation.md)。1.2有界来源资源与当前/基线资源已实现；继续原生Qt明确原包范围导出及完整交换。

ABR用户入口：[设计](abr-entry-design.md) / [验收](abr-entry-validation.md)。ABI1.33，6/10基础整包已接原生Qt并保留来源报告；完整兼容/VBR来源/动态湿混继续后续。

ABR整包IO导入：[设计](abr-import-design.md) / [验收](abr-import-validation.md)。整包转换/编译/图片及来源批次已完成；继续原生Qt用户入口与VBR来源。

笔刷来源界面：[设计](brush-source-ui-design.md) / [验收](brush-source-ui-validation.md)。v4与后台原生报告已接入Qt；实际ABR入口/批量来源与VBR选择性来源继续后续。

原生来源准备桥：[设计](brush-source-bridge-design.md) / [验收](brush-source-bridge-validation.md)。ABI1.32，共享目录/v4准备/来源报告与有生命周期校验凭据已实现；继续Qt来源与实际ABR入口。

原生来源引用/状态：[设计](brush-source-state-design.md) / [验收](brush-source-state-validation.md)。目录保留/v4状态/批次验证已实现；原生准备桥/Qt/VBR与高频来源复用继续后续。

笔刷来源资源：[设计](brush-source-design.md) / [验收](brush-source-validation.md)。完整来源/导入时快照/报告资源存取已实现。

ABR基础转换：[设计](abr-conversion-design.md) / [验收](abr-conversion-validation.md)。严格参数映射/真实编译/逐叶报告已实现；外部无动态baseline alpha核对通过，但实际样本为部分转换；原生Qt入口/动态继续后续。

ABR样本关联：[设计](abr-association-design.md) / [验收](abr-association-validation.md)。Pascal身份/共享覆盖/严格关联已实现；公开外部样本alpha逐像素核对通过但不分发。

ABR描述符：[设计](abr-descriptor-design.md) / [验收](abr-descriptor-validation.md)。typed tree及显式desc读取已接入；PS具体参数/真实授权语料继续后续。

ABR分节采样：[设计](abr-sections-design.md) / [验收](abr-sections-validation.md)。6/10子版本1/2有界分节与采样已实现；16位/真实授权语料继续后续。

ABR采样读取：[设计](abr-sampled-design.md) / [验收](abr-sampled-validation.md)。1/2原始与PackBits采样/有界读取已实现；描述符/程序及动态映射继续后续。

VBR图片资源：[设计](vbr-image-design.md) / [验收](vbr-image-validation.md)。ABI1.31/VBR1.1，完整资源清单/跨资源目录导入导出已接Qt；分组/扩展/动态湿混继续后续。

图片笔尖轮廓：[设计](brush-outline-design.md) / [验收](brush-outline-validation.md)。ABI 1.30，基础0.5覆盖/孔洞与多区域/有界缓存与Qt描边已完成；半透明悬停、复杂细节简化及动态设备继续后续。

图片笔尖Qt接入：[设计](brush-image-ui-design.md) / [验收](brush-image-ui-validation.md)。PNG导入/捕获目标/v3持久/真实预览已完成；VBR图片资源继续后续。

图片笔尖原生资源接口：[设计](brush-resource-abi-design.md) / [验收](brush-resource-abi-validation.md)。ABI 1.29，共享资源目录/v3准备/闲置修订回收已接入。

图片笔尖身份与状态：[设计](brush-resource-state-design.md) / [验收](brush-resource-state-validation.md)。v3持久引用与实际绘画已接Rust。

图片笔尖资源IO：[设计](brush-resource-design.md) / [验收](brush-resource-validation.md)。有界PNG/哈希/持久文件及预算已完成，继续将身份接入笔刷状态和用户图片工具。

Qt / 原生修订桥：[设计](brush-catalog-bridge-design.md) / [验收](brush-catalog-bridge-validation.md)，ABI 1.28。画布与预览已接原生定义，旧参数后台迁移与有效状态保存已完成；继续图片持久资源与 VBR/ABR 接入。

原生笔刷库：[ABI设计](brush-catalog-abi-design.md) / [验收](brush-catalog-abi-validation.md)，ABI 1.27。已接独立库与实际笔触快照；继续推进 Qt 目录镜像/资源存储、VBR图片与ABR。

Rust 笔刷库模型：[设计](brush-catalog-design.md) / [验收](brush-catalog-validation.md)。不可变预设和原子库操作/资源预算已完成；下一阶段接库 ABI、Qt 镜像、持久图片资源及 ABR。

图片笔尖内核：[设计](brush-mask-design.md) / [验收](brush-mask-validation.md)。资源与实际像素路径已完成；接下来完成 Rust 库/持久资源身份，VBR 图片和 ABR 导入。

笔尖光标：[设置设计](brush-cursor-settings-design.md) / [验收](brush-cursor-settings-validation.md)。全局显示偏好与多文档同步/保存已完成；图片笔尖轮廓和硬件动态待后续引擎阶段。

VBR 程序子集：[设计](vbr-procedural-design.md) / [验收](vbr-procedural-validation.md)，ABI 1.26。已实现程序预设包导入/导出与严格验证；图片资源、ABR 与其他模块继续后续阶段。

笔刷系统第一阶段：[程序笔尖设计](brush-tip-design.md) / [验收](brush-tip-validation.md)，ABI 1.25。已接共用程序笔尖、PS 分类设置、真实预览与椭圆光标；VBR、ABR、图片笔尖、动态与湿混继续逐模块实施。

已实现置入图片 → 独立原始图片模型 → 基础自由变换与栅格化，见 [设计](transform-design.md) / [验收](transform-validation.md)。CPU 处理与变换描述分离；GPU 和文字仍为后续模块。

液化：[设计](liquify-design.md) / [验收](liquify-validation.md)，ABI 1.21；滤镜入口、画布事务、工具详情栏与图标、推拉/膨大/扭曲/重建、参数保存与压感、文档锁定和浮窗隐藏恢复。

组穿透：[设计](group-pass-through-design.md) / [验收](group-pass-through-validation.md)，ABI 1.19，组专属穿透模式与嵌套/蒙版/剪贴/ORA 支持。

吸色取色环：[设计](eyedropper-ring-design.md) / [验收](eyedropper-ring-validation.md)，复用显示帧取样，鼠标/Alt/数位笔拖动预览新旧颜色。

涂抹：[设计](smudge-design.md) / [验收](smudge-validation.md)，ABI 1.18；独立工具复用画笔管线、强度压感、合成取样、手指绘画与 1% 间隔。

魔棒：[设计](wand-design.md) / [验收](wand-validation.md)，ABI 1.17；真实灰度选区、平均取样、四种组合、轮廓和历史/持久化。

每轮先补设计，再实现、运行测试并记录结果；已交付第 1–5 阶段，QML 已接 Rust 任务与 CPU 视口渲染；插件层最后完成。

最新共用选区与套索：[设计](lasso-selection-design.md) / [验收](lasso-selection-validation.md)，ABI 1.15；矩形/椭圆与自由/多边形套索共用新建、添加、减去、交叉、羽化与消除锯齿。

框选与套索内画笔/橡皮性能优化：[设计](selection-performance-design.md) / [验收](selection-performance-validation.md)；选区按行覆盖与有界缓存，保留原路径、AA、羽化与组合规则，无 ABI/文件格式变更。

油漆桶：[设计](bucket-design.md) / [验收](bucket-validation.md)，ABI 1.16；独立不透明度、默认容差 0、AA、四邻接连续/全局填充、可见合成取样仅写活动层、选区与锁、原子撤销历史。

最新菜单悬停与多选建组：[设计](menu-hover-grouping-design.md) / [验收](menu-hover-grouping-validation.md)，ABI 1.14；混合预览按列表指针位置统一更新，原生菜单完整鼠标坐标转发，首行跨边移动保持菜单链，多选一次建组与撤销。

最新面板收起尺寸修复：[设计](panel-fold-sizing-design.md) / [验收](panel-fold-sizing-validation.md)；工具条与相邻面板不自动补位，浮窗收窄，展开宽度可随布局 v6 保存恢复。

最新菜单与批量图层修复：[设计](menu-layer-batch-design.md) / [验收](menu-layer-batch-validation.md)，ABI 1.13；悬停侧向分类菜单、组删除保留内容/删除子树选择、原子多选删除/拖放与剪贴链继承，图层面板首开异步创建。

最新遗留修复：[设计](deepseek-recovery-design.md) / [验收](deepseek-recovery-validation.md)，保留 ABI 1.12；独立工具栏吸色、缩放/DPI 坐标、剪贴链重排、面板剩余高度和缩略图通知。GPU、液化、.vbr 与图层复制/合并按用户确认另立模块。

真实历史名称：[设计](history-labels-design.md) / [验收](history-labels-validation.md)，ABI 1.10 提供保留历史的真实工具类型与裁剪边界，名称/图标不再从撤销深度猜测，逐条回放保留。

最新界面细化：[设计](panel-refinement-design.md) / [验收](panel-refinement-validation.md)，ABI 1.9 混合悬停快照、拆分浮窗尺寸保持、即时交换颜色、共享紧凑控件、外描边与数位板悬停轮廓。

| 顺序 | 模块 | 状态 | 验收门槛 |
|---|---|---|---|
| 1 | 架构 / C ABI 草案 / UI 工作区设计 | 已设计 | 分层、所有权、线程、错误与版本有明确契约 |
| 2 | paint-core 最小内核 | 已实现并通过本机验证 | 文档、图层、压感圆笔刷、稀疏瓦片、撤销/重做/预算回归 |
| 3 | paint-ffi + cbindgen | 已实现并通过本机验证 | ABI 1.6.0 向后兼容、staticlib/cdylib、生成头漂移检查、36 项 FFI 回归、C/C++ 四组含异步链接运行 |
| 4 | paint-task + paint-render CPU | 已实现并通过 Windows 本机验证 | 有界优先级、串行写/快照、取消/panic、CPU 合成/稀疏 LOD、异步帧与背压、大画布/视口/导航/撤销回归 |
| 5 | Qt Quick / QML 绘画 UI / 工作区 | 已实现并通过 Windows 本机验证 | 后台 PaintCoreClient、画布、图层/颜色/笔刷/历史/导航、标签组拖放/浮动、自定义面板、布局保存、Qt Test |
| 6 | 平台数位板接入 | Qt 通道已接入，硬件待验证 | 接收 QTabletEvent、压力/倾斜/旋转/切向/橡皮擦/按钮、坐标与两倍 DPI 测试；Wintab 选择及真实硬件矩阵仍待实施 |
| 7 | paint-io 基础格式 | 已实现并通过 Windows 本机验证 | PNG/JPEG/WebP/ORA 回环、16 位导入、EXIF、截断/路径/XML/归档预算、原子保存、ABI 1.2 后台读写与 Qt 文件流程 |
| 7a | paint-storage 配置与清扫 | 已实现并通过 Windows 本机验证 | ABI 1.3、QML 性能设置、重启生效、冷页/历史共享所选目录、空闲检查、真实进程租约与异常清扫回归 |
| 7b | PS 风格图层控制 | 已实现并通过 Windows 本机验证 | 27 种混合、透明/位置/完全锁、opacity/fill、非破坏性移动、可见行缩略图、历史、ORA 扩展、ABI 1.4、Qt 浮窗/DPI 与性能 |
| 7c | 图层组 / 透明画布背景 | 已实现并通过 Windows 本机验证 | 隔离嵌套、分组/解组/重新归组、锁继承、移动/子树删除、结构/磁盘历史、ORA、ABI 1.5、QML 浮窗/折叠、高 DPI 棋盘格 |
| 7d | 蒙版 / PS 风格拖放 / 默认背景 | 已实现并通过 Windows 本机验证 | 图层/组黑白灰蒙版、压感、显隐/撤销/删除、ORA 可编辑回读、隐式白色背景、中文默认命名、停靠/浮窗拖放、ABI 1.6 |
| 8a | 几何选区 | 已实现并通过 Windows 本机验证 | ABI 1.8、矩形/椭圆、组合/反选、绘画约束、历史与 ORA v4、Qt 浮窗/DPI |
| 8 | 深化文档模型 | 部分完成，继续推进 | 几何选区见8a、一般变换；蒙版见 7d；组/三锁/混合/整数移动及相应历史和保存见 7b/7c |
| 9 | paint-color / GPU / PSD | 待实施 | 色彩金图、16/32F、ICC、多屏、GPU/CPU 一致性与失败回退、PSD 支持表 |
| 10 | 跨平台 CI / 测试 / 打包 | 待实施 | Windows MSVC、macOS、Linux，Qt Test，签名/打包方案，CPU-only 可启动 |
| 11 | paint-plugin / Python SDK / 沙箱 | 最后 | pytest、示例滤镜、权限拒绝、超时、崩溃隔离、平台沙箱、声明式自定义面板 |

后续新增 crate 在真正实现时才加入 workspace，不提前加入空壳。当前有 Rust、C/C++ ABI、Qt Test，Python 插件还未实施。统一 check 默认包括 UI，需要 Qt SDK；明确只测核心时用 --core-only。最新文件/工作区及 UI 验证见 io-validation.md；任务/渲染阶段记录见 task-render-validation.md。CI 可逐步增加，最终打包在第 10 阶段验收；插件完成后再扩展 CI。

绘画性能回退修复已验收：CPU 瓦片缓存/稀疏输出、逐源像素编码、QML 图层通知拆分，保留 ABI 与原画质；本机固定压感输入基准及回归见 painting-performance-validation.md。

大笔触历史限制已修复，采用内存快照 / 无损编码 / 临时磁盘文件，保留整笔撤销与资源预算，见 history-storage-design.md / history-storage-validation.md。live 文档瓦片分页已实现（见 document-paging-design.md / document-paging-validation.md）。paint-storage 的暂存盘设置、空闲空间检查和租约清扫已完成（见 storage-settings-design.md / storage-settings-validation.md），下一模块推进选区等文档模型；插件仍最后实施。

PS 风格图层控制按用户澄清完成（透明像素锁；“样式”仅为混合下拉），见 layers-design.md / layers-validation.md。图层组与透明背景见 groups-design.md / groups-validation.md；蒙版与拖放及默认背景见 masks-design.md / masks-validation.md；下一模块推进选区；fx、特殊 Fill 曲线另行设计，插件最后。


### 7e. 并排蒙版、剪贴蒙版与缩略图边缘

用户要求将像素蒙版放在内容预览右侧，添加 Alt/Option 层间点击剪贴及释放，连续上层共用下方同级基底。核心合成、结构/磁盘历史、缓存、异步 ABI 1.7 和 ORA v3 已实现；旧 ORA v1/v2 继续读取。缩略图边框/留白不再显示棋盘格，部分边缘 LOD 按有效文档像素归一化。设计与验收见 docs/clipping-design.md / docs/clipping-validation.md。

### 7f. 参考图 QML 界面与停靠

紧凑无边框主窗、HSB 色轮、Logo/矢量图标、参考图与双列预设、标签排序/合并/上下拆分、折叠图标临时面板、关闭重开、稳定原生浮窗、工具条拖动浮动归位和布局 v4 已验证。见 reference-ui-design.md / reference-ui-validation.md。下一步按用户补充完成画笔库与当前画笔设置的共享状态与真实预览，然后继续选区、变换、色彩/GPU、PSD、自有格式和 CI；插件最后。每个模块完整验证后独立 commit 并推送 origin/master。

### 7g. 画笔库与当前笔刷设置

共享活动预设、按画笔独立参数、后台保存、自定义预设与真实异步笔触预览已完成。设置页无第二个笔尖网格；大小/间距按直径显示并正确转换核心单位。详见 brush-library-design.md / brush-library-validation.md。下一模块推进矩形/椭圆选区、选区组合与反选、限制绘画及历史；之后一般变换和核心高级笔刷。

### 7h. 画布与工具条的自由停靠

画布、工具条和工具标签组共用主窗及原生浮窗的二维分割树，四边停靠、标签合并/排序和空分支收缩；移走最后一个面板不留固定列。画布为唯一可移换父项的实例，保留异步绘画、缩放/平移、选区与压感输入；工具条可在任意面板两侧放置。布局 v5 保存比例和窗口几何，v1–v4 迁移保留既有内容。设计与验收见 flexible-docking-design.md / flexible-docking-validation.md。仅 UI 层修改，Python 插件仍最后。


### 7i. 多文档工作区与面板列

多个独立文档通过已有 PaintCoreClient / C ABI session 实施，主区与浮窗标签保留各自绘画/历史/文件任务和视口。画布/工具互拒，上边缘归位，#17191C 留空文档区；工具列顶部唯一导航栏、整体图标折叠、1px 分隔线与实际浮窗拖动。布局 v6 迁移旧工具布局；关闭/退出处理全部未保存文档。设计与验收见 document-workspace-design.md / document-workspace-validation.md；Rust/ABI 未改，Python 最后。

### 7j. 图标列与临时面板交互修复

主工作区边缘优先接收工具归位，画布独占/空白工作区均可拖回面板。面板列只保留图标折叠，新增元素继承状态；图标直接拖出，侧边临时窗正确切换历史/图层，空白关闭且可上下滑动。移除文档条新建加号，单画布浮窗一行标题，多画布时显示活动文档窗口名和标签行。见 icon-rail-design.md / icon-rail-validation.md；仅 UI 修改。

### 8a. 几何选区

矩形/椭圆、组合/反选、文档坐标绘画限制、统一历史、ABI 1.8、ORA v4、Qt 鼠标/数位笔/浮动工具条/高 DPI 已验证。无选区不创建轮廓绘制项，笔刷走专用快路径；600点同环境基准保持原有CPU与交互更新水平。见 selection-design.md / selection-validation.md。后续推进一般变换、核心高级笔刷、色彩/GPU、PSD、自有格式及跨平台构建；插件最后。

## 窗口恢复与统一边缘吸附（2026-10-09）

主窗最小化恢复保留无边框、内容及最大化状态；原生圆角裁剪不处理 iconic HWND。图标临时面板全应用唯一，外部鼠标/触控/数位笔与失焦关闭；旧窗延迟关闭不影响新窗。工具面板四边与主区两边使用统一 24px 最近边判定，拖动窗口边界参与，遮挡和自身排除，窄边发光；正文合并改为标题区操作。单画布标题与按钮同色、图标栏背景统一 surface、删除工具条拖动提示。标签在栏内优先交换位置，越出栏再拆为浮窗，图标临时面板同样支持。参考图布局 API 与自动初始化删除，双列停靠作为默认布局，保存的用户布局仍可恢复。设计与验收见 [window-recovery-design.md](window-recovery-design.md) / [window-recovery-validation.md](window-recovery-validation.md)。

主窗补齐系统最小化/最大化能力，并仅对其 HWND 过滤非客户区绘制，防止恢复后系统标题框写入半透明菜单。真实合成像素回归覆盖普通/最大化恢复后的模糊和圆角；工具面板及面板列统一灰黑细分隔。设计与验收见 [taskbar-window-design.md](taskbar-window-design.md) / [taskbar-window-validation.md](taskbar-window-validation.md)。

菜单底层矩形模糊进一步改为独立非交互 HWND 承载 HostBackdropBrush，通过合成视觉的上方圆角裁剪真实模糊；最小化前隐藏，恢复不参与主窗状态过渡。新增高频黑白格真实屏幕像素回归。按用户澄清，最大化标题栏拖动越过阈值后恢复普通尺寸并继续跟随鼠标，全屏保持状态。笔刷预设选择不再切换工具，画笔与橡皮擦继续共用预设和参数，并验证实际绘画/擦除。见 [menu-blur-layer-design.md](menu-blur-layer-design.md) / [menu-blur-layer-validation.md](menu-blur-layer-validation.md) 及画笔库验收。

## 常驻图标面板列（2026-10-10）

取代唯一临时面板和空白/失焦关闭：同列每个图标可打开独立常驻面板，允许多开，最近点击置顶；显式收回或 Escape 关闭。展开面板独立调整宽高，导航沿列侧边滑动，短浮动图标列不压缩面板高度。图标列背景和空白统一 #1C1E21，保留可调列高；整个浮动列归位为独立主窗列，展开/图标状态与树结构保留。普通标签排序、单图标拖出及布局 v6 保持。见 [persistent-panel-rails-design.md](persistent-panel-rails-design.md) / [persistent-panel-rails-validation.md](persistent-panel-rails-validation.md)。

## 面板状态与紧凑信息栏（2026-10-10）

同图标再次点击收回对应面板，其他打开面板不受影响；独立宽高与来源列侧边偏移通过布局 v6 可选 panelViews 保存，收回再打开和重启恢复继承。描边/分隔统一 #1B1C1F，标签圆角 3px，拖动栏与标签栏背景 #05080A，浮动时背景 75% 不透明，正文不透明。底栏显示工具、共享画笔预设、文档尺寸、图层数，以及中心缩小/放大、单小数百分比和适配；工具条适合窗口按钮删除，菜单/F 保留。见 [panel-presentation-design.md](panel-presentation-design.md) / [panel-presentation-validation.md](panel-presentation-validation.md)。

## 面板分组与统一内容（2026-10-10）

外侧边停靠生成独立整列，内侧边并列属于同组；标签与并列组合一起打开，不同组图标间居中灰黑线。多组常驻、再次点击当前图标或双击顶栏收回，来源列保持折叠；整列拖动/归位/保存保留左右树和状态。浮动展开列仅顶层栏 75% 不透明，下方栏和空白不透明。共用 PanelContent/PanelGroup 与 PanelWindow 调色板，导航仅预览和实时红色视口框，历史移除撤销/重做按钮；按用户后续要求保留历史逐条跳转。见 [panel-categories-design.md](panel-categories-design.md) / [panel-categories-validation.md](panel-categories-validation.md)。

最新工作区调整见 [workspace-tools-design.md](workspace-tools-design.md) 与 [workspace-tools-validation.md](workspace-tools-validation.md)：工具列整列停靠与限定宽度、鼠标磁吸/外侧提示、展开收起的像素位置保持、置顶菜单、独立工具大小、固定真实预览、颜色/轮廓/空格交互。布局 v6 和 ABI 1.8 保持，历史逐条跳转保留。

标签双击只收回当前呈现、不再展开来源图标列（页头与标签一致），并补齐浮动栏 75% 不透明度、文本输入保留空格、历史 20ms 逐条回放的真实断言。见 [panel-tab-collapse-design.md](panel-tab-collapse-design.md) / [panel-tab-collapse-validation.md](panel-tab-collapse-validation.md)；仅 UI/测试修改，ABI 1.10 与布局 v6 不变。

顶部菜单改为 `文件 · 编辑 · 图像 · 图层 · 选择 · 滤镜 · 窗口`（工作区并入窗口、删除视图），编辑→首选项打开左侧分类侧栏对话框（性能 / 历史 / 界面），未实现项按"待支持"标注。历史条数上限做成真实设置：`paint-core` 运行时裁剪最旧记录并标记边界不可达，ABI 1.11 新增 `paint_session_set_history_limit`，`PaintCoreClient.historyLimit` 持久化并即时应用。展开窗不再自动补长，最近颜色只在下笔提交后更新。见 [menus-preferences-design.md](menus-preferences-design.md)、[panel-length-and-snap-design.md](panel-length-and-snap-design.md)。

同一工具的多形态合并到一个槽位：图标右下角小三角提示可切换，右键弹出悬浮面板列出全部形态，点击面板项才切换，点工具栏图标只激活当前形态（选区槽位含矩形/椭圆，套索待支持）。见 [tool-variants-design.md](tool-variants-design.md)；编辑→键盘快捷键只读展示真实绑定。

图层操作（ABI 1.20）：[设计](layer-actions-design.md) / [验收](layer-actions-validation.md)，右键菜单、复制粘贴/合并、缩略图焦点与下边缘拖放修复。


## 统一笔刷系统规划（2026-10-13，待实施）

设计见 [brush-system-design.md](brush-system-design.md)，VBR 契约草案见 [vbr-format-design.md](vbr-format-design.md)，验收条件见 [brush-system-validation.md](brush-system-validation.md)。当前只完成设计，未实现高级笔刷、ABR/VBR 或 GPU。

按统一定义与旧圆笔迁移 → 路径手感/图片笔尖/描边预览 → 纹理/动态/分级 ABR → 湿混与涂抹 → 实测后 wgpu 的顺序推进。设置面板采用左侧分类、右侧参数、底部真实预览；光标描边为全局显示偏好。每阶段仅展示真实实现的能力，不以格式解析成功代替绘画兼容。
