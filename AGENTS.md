# DrawVerse

- 最新原生来源准备桥：`docs/brush-source-bridge-design.md` / `docs/brush-source-bridge-validation.md`；ABI1.32、共享来源上下文/v4准确准备、Weak身份凭据/当前与在途独立持有、后台来源报告新校验；Qt/VBR来源/ABR入口继续后续。

- 最新原生来源引用/状态：`docs/brush-source-state-design.md` / `docs/brush-source-state-validation.md`；来源与执行分离/不可变修订/编辑副本保留、v4显式上下文与批次原始验证；准备桥/Qt/VBR与高频来源复用继续后续，无ABI变化。

- 最新笔刷来源资源：`docs/brush-source-design.md` / `docs/brush-source-validation.md`；完整原始来源/小型导入快照与报告、SHA身份/不覆盖保存/原字节交换；原生预设与状态/Qt/VBR接入、批量来源IO继续后续，无ABI变化。

- 最新ABR基础转换：`docs/abr-conversion-design.md` / `docs/abr-conversion-validation.md`；严格单位/范围、真实编译/共享覆盖、逐叶支持与未执行报告、外部无动态alpha核对；来源保存/原生Qt入口/动态与PS核验继续后续，无ABI变化。

- 最新ABR样本关联：`docs/abr-association-design.md` / `docs/abr-association-validation.md`；Pascal身份/可变前缀/sampledData严格关联、共享资源/完整来源字段；外部上游诊断像素匹配但资产不分发，参数报告/用户入口继续后续，无ABI变化。

- 最新ABR描述符：`docs/abr-descriptor-design.md` / `docs/abr-descriptor-validation.md`；有界typed tree/单位与引用/显式desc接入、长度与递归拒绝；具体PS参数/样本关联/用户入口/真实授权语料继续后续，无ABI变化。

- 最新ABR分节采样：`docs/abr-sections-design.md` / `docs/abr-sections-validation.md`；6/10×1/2有界分节/记录对齐/共用PackBits、原始来源元数据保留；desc执行/用户入口/16位/真实授权语料继续后续，无ABI变化。

- 最新ABR采样读取：`docs/abr-sampled-design.md` / `docs/abr-sampled-validation.md`；Rust1/2采样原始/PackBits/UTF16与来源字段、整包拒绝与真实孔洞测试；尚未接用户入口，6/10/程序/16位/动态报告和授权真实语料继续后续，无ABI变化。

- 最新VBR图片资源：`docs/vbr-image-design.md` / `docs/vbr-image-validation.md`；ABI1.31/VBR1.1、完整PNG身份/资源清单/整包编译/多资源交接、Qt跨资源目录导入导出/UUID副本；分组/扩展/ABR/动态湿混继续后续。

- 最新图片笔尖轮廓：`docs/brush-outline-design.md` / `docs/brush-outline-validation.md`；ABI 1.30、0.5阈值/孔洞与多区域/有界几何缓存、Qt当前身份与显示变换、DPI固定描边；超复杂纹理明确简化，图片悬停/动态设备/VBR图片/ABR继续后续。

- 最新图片笔尖Qt接入：`docs/brush-image-ui-design.md` / `docs/brush-image-ui-validation.md`；后台PNG三种解释/捕获目标/持久v3/独立真实预览、图片硬度禁用和程序恢复；图片精确轮廓/VBR图片/ABR/高级动态仍待完成。

- 最新图片笔尖原生资源接口：`docs/brush-resource-abi-design.md` / `docs/brush-resource-abi-validation.md`；ABI 1.29、后台PNG导入/共享资源目录、v3准确准备、闲置修订释放/重建及在途资源保留；Qt图片控件/VBR图片/ABR仍待接入。

- 最新图片笔尖身份/状态：`docs/brush-resource-state-design.md` / `docs/brush-resource-state-validation.md`；覆盖不可变PNG身份/v3哈希引用/后台资源恢复与整包编译，旧状态保留；原生资源目录/Qt/VBR图片/ABR仍待接入。

- 最新图片笔尖资源IO：`docs/brush-resource-design.md` / `docs/brush-resource-validation.md`；SHA-256内容身份/不覆盖原子PNG、8/16位覆盖转换与量化标记、Weak缓存/活跃mip预算，资源ID原生库/Qt接入与VBR图片/ABR仍待后续。

- 最新 Qt / 原生笔刷修订桥：`docs/brush-catalog-bridge-design.md` / `docs/brush-catalog-bridge-validation.md`；ABI 1.28、后台状态迁移/独立编译池/准确准备记录、Rust起笔等待与Qt共享库、隔离原生预览/有效状态保存；图片资源/ABR/动态/湿混继续后续。

- 最新原生笔刷库 ABI：`docs/brush-catalog-abi-design.md` / `docs/brush-catalog-abi-validation.md`；ABI 1.27、独立句柄/准确修订/原子目录操作、捕获预设的真实异步笔触及生命周期；Qt 镜像迁移与图片资源接口仍待接入。

- 最新 Rust 笔刷库模型：`docs/brush-catalog-design.md` / `docs/brush-catalog-validation.md`；验证编译/不可变修订、原子目录操作、旧预设兼容、1024预设/共享资源预算；原生库 ABI 与 Qt 完整迁移仍待接入。

- 最新图片笔尖内核：`docs/brush-mask-design.md` / `docs/brush-mask-validation.md`；不可变覆盖资源/mip、每印章采样计划、孔洞/非方形/旋转完整边界、资源快照与四类写入共用，图片 UI/VBR 资源/ABR 尚待接入，无 ABI 变化。

- 最新笔尖光标偏好：`docs/brush-cursor-settings-design.md` / `docs/brush-cursor-settings-validation.md`；实际/简化轮廓、中心/悬停/线宽/颜色、全局保存不进入 VBR、多文档笔刷状态同步与单一写入者移交，无 ABI 变化。

- 最新 VBR 程序子集：`docs/vbr-procedural-design.md` / `docs/vbr-procedural-validation.md`；ABI 1.26、Rust 有界 ZIP/JSON 与原子 IO、程序预设 UUID/能力/单位、后台导入导出、冲突副本与整包发布；资源/ABR 暂未实现。

- 笔刷系统：`docs/brush-system-design.md` / `docs/brush-system-validation.md` / `docs/vbr-format-design.md`；总体分阶段实施，ABR/VBR 等后续范围见验收计划。
- 最新程序笔尖：`docs/brush-tip-design.md` / `docs/brush-tip-validation.md`；ABI 1.25、共用编译覆盖、硬度/圆度/角度/流量与压感、旧圆笔兼容、预设 v2 原子恢复、PS 分类面板/真实预览/旋转椭圆光标。

- 最新 Ctrl 选层与临时移动：`docs/ctrl-layer-pick-design.md` / `docs/ctrl-layer-pick-validation.md`；ABI 1.24、异步像素命中/原子移动/锁与蒙版、图层搜索首次点击焦点、自由变换共用紧凑输入框。

- 最新图片与变换：`docs/transform-design.md` / `docs/transform-validation.md`；ABI 1.23、嵌入原始图片模型、Ctrl+T/八点控制/透视与网格、组/多选/选区像素原子变换、会话撤销重做、栅格化与 ORA v5 原图恢复。

- 最新液化历史：`docs/liquify-history-design.md` / `docs/liquify-history-validation.md`；ABI 1.22、整笔触会话撤销/重做、设置步数与校验暂存、历史域和滚动恢复、Ctrl+Z/Ctrl+Shift+Z、[ ] 液化尺寸。

- 最新液化性能：`docs/liquify-performance-design.md` / `docs/liquify-performance-validation.md`；连续预览非屏障、瓦片批处理与相邻取样缓存、共用选区覆盖缓存保留坐标、紧凑复选框与标题分隔，无 ABI 变更。

- 最新搜索与快捷键：`docs/layer-search-tools-design.md` / `docs/layer-search-tools-validation.md`；组层级搜索、文本编辑与画布焦点区分、快捷键恢复、液化仅滤镜入口及 Enter/Esc、图层拷贝粘贴仅快捷键。

- 最新液化：`docs/liquify-design.md` / `docs/liquify-validation.md`；ABI 1.21、滤镜入口、工具详情栏与图标、稀疏位移场与原始取样、事务历史、保存参数、文档锁定与浮窗隐藏恢复。

- 最新图层操作：`docs/layer-actions-design.md` / `docs/layer-actions-validation.md`；ABI 1.20、右键/主菜单、原子复制粘贴/子树合并、缩略图编辑焦点、Delete 与文本焦点保护、下边缘拖放修复。

- 最新组穿透：`docs/group-pass-through-design.md` / `docs/group-pass-through-validation.md`；ABI 1.19、组专用混合模式 27，子层对组外下层合成，opacity/fill/蒙版与剪贴、预览/撤销/ORA；新建组默认穿透、菜单置于正常上方。

- 最新吸色取色环：`docs/eyedropper-ring-design.md` / `docs/eyedropper-ring-validation.md`；按下拖动显示新旧颜色半环与灰色外圈、I/Alt/数位笔共用、显示开关、无 ABI 变更。

- 最新涂抹：`docs/smudge-design.md` / `docs/smudge-validation.md`；ABI 1.18、独立 R 工具复用笔刷、强度/压感/合成取样/手指绘画、最低 1% 间隔；参数说明统一悬停 1 秒。

跨平台绘画应用：Rust 内核 → 稳定 C ABI → C++20 / Qt 6.5+ UI；Python 插件最后实施。

- 最新魔棒：`docs/wand-design.md` / `docs/wand-validation.md`；ABI 1.17、四种组合/七种取样尺寸/默认容差 10/AA/连续/所有图层、灰度掩码与真实轮廓、历史和持久化。

- 最新油漆桶：`docs/bucket-design.md` / `docs/bucket-validation.md`；ABI 1.16、不透明度/容差（默认 0）/AA/连续/所有图层取样、选区与锁、单条原子历史、鼠标/数位板和浮动画布。

- 最新选区绘画性能：`docs/selection-performance-design.md` / `docs/selection-performance-validation.md`；框选/套索按行覆盖与有界缓存、画笔和橡皮共用、保留 AA/羽化/组合像素结果，无 ABI 变更。

- 最新共用选区与套索：`docs/lasso-selection-design.md` / `docs/lasso-selection-validation.md`；ABI 1.15、自由/多边形套索、四种组合与羽化/AA 共用、路径索引与历史/ORA、鼠标和数位板采集。

- 最新菜单悬停与多选建组：`docs/menu-hover-grouping-design.md` / `docs/menu-hover-grouping-validation.md`；ABI 1.14、统一指针预览、原生菜单完整坐标转发、首行跨边回归、多选一次建组与撤销。

- 最新面板折叠尺寸保持：`docs/panel-fold-sizing-design.md` / `docs/panel-fold-sizing-validation.md`；布局 v6 可选 retainedWidths，折叠仅压缩自身、工具栏宽度优先、浮窗收窄、相邻宽高与展开尺寸保持、显式调整解除保持。

- 最新菜单分类与图层批量修复：`docs/menu-layer-batch-design.md` / `docs/menu-layer-batch-validation.md`；ABI 1.13、共享悬停侧向子菜单、组删除双选择、原子多选删除/拖放、拖入剪贴链自动继承、图层面板首开异步创建。
- 最新遗留修复与吸色：`docs/deepseek-recovery-design.md` / `docs/deepseek-recovery-validation.md`；ABI 1.12、I 独立吸色与 Alt 临时吸色、DPI/视口坐标修复、剪贴链插层/基底变化、缩略图独立通知、剩余面板高度和恢复拖动。GPU/液化/.vbr/图层复制合并本轮暂缓。
- 当前阶段与验证命令：`README.md`、`docs/roadmap.md`。
- 最新真实历史标签：`docs/history-labels-design.md` / `docs/history-labels-validation.md`；ABI 1.10、真实工具名称/图标、裁剪后不显示伪初始状态；20ms 逐条历史回放保留。
- 最新面板细化与混合悬停预览：`docs/panel-refinement-design.md` / `docs/panel-refinement-validation.md`；ABI 1.9、浮窗拆分保留尺寸、颜色即时交换、共享紧凑控件、向外半透明描边、数位板悬停轮廓。替代 HEX 双击输入与延迟颜色交换。
- 检出/构建规则：`.gitattributes` / `docs/build-reproducibility.md`；文本 LF，生成头严格字节检查，每个工作树独立 Cargo target，tools/check.py 包含真实 Git 检出测试。
- 架构、UI 停靠与兼容性：`docs/architecture.md`。
- 正式 C ABI：`core/crates/paint-ffi/src/api.rs` / `ui/include/paint_api.h`；契约在 `contracts/abi.md`。
- `contracts/paint-api-v1.draft.h` 仅为历史设计；FFI 方案/验收在 `docs/ffi-design.md`。
- 当前文件 IO 与布局 v2 清理：`docs/io-design.md` / `docs/io-validation.md`；ABI 1.2 文件任务、Qt 文件菜单，笔记已退役。
- 当前实现：`core/crates/paint-core/src`、`core/crates/paint-ffi/src`、`ui/src`、`ui/qml` 与对应测试。
- 最新 UI 设计/验收：`docs/qml-ui-design.md` / `docs/qml-ui-validation.md`；用户指定 QML，旧 Widgets 设计已被取代。
- 最新参考图 UI / 停靠：`docs/reference-ui-design.md` / `docs/reference-ui-validation.md`；无边框紧凑工作区、原生浮窗、工具条拖放、布局 v4。用户要求每个模块验证后 commit 并推送 origin/master，持续推进；Python 插件最后。
- 最新自由停靠：`docs/flexible-docking-design.md` / `docs/flexible-docking-validation.md`；画布、面板组和工具条共用主窗/浮窗分割树，空分支收缩，布局 v5，替代固定左右列；只有一个可移换父项的主画布实例。
- 最新文档工作区 / 面板列：`docs/document-workspace-design.md` / `docs/document-workspace-validation.md`；多个独立 PaintCoreClient 文档、画布浮窗标签、工具列统一图标折叠、实际原生窗口拖动、布局 v6。画布与工具面板互不接收，主窗保留 #17191C 文档留空区。
- 最新常驻图标面板列：`docs/persistent-panel-rails-design.md` / `docs/persistent-panel-rails-validation.md`；每个图标独立打开、允许多开、空白与失焦不收回、最近点击置顶、侧边导航锚定与独立缩放、浮动列背景/高度及归位状态保留。替代此前唯一临时窗及空白关闭。
- 最新工具列与画布交互：`docs/workspace-tools-design.md` / `docs/workspace-tools-validation.md`；工具整列停靠、36–72px 宽度与拆分缩回、仅鼠标磁吸/外侧提示、折叠按钮拖动、布局像素位置保持、菜单置顶、画笔/橡皮独立大小与固定预览、颜色单双击、轮廓与跨面板空格平移；顶部 #05080A 75%，外描边 #63666B。
- 最新面板分组与统一渲染：`docs/panel-categories-design.md` / `docs/panel-categories-validation.md`；外侧整列停靠、内侧同组并列，布局 v6 可选 row 角色；图标按组整窗展开与居中分隔，双击收回不展开列，浮动列仅顶层栏透明，导航红框与共享内容/调色板，历史面板无撤销/重做按钮（逐条跳转保留）。
- 最新面板状态与信息栏：`docs/panel-presentation-design.md` / `docs/panel-presentation-validation.md`；同图标再次点击收回，独立宽高/侧边位置随布局 v6 保存；描边 #1B1C1F、小标签圆角、顶部 #05080A（浮动 75% 不透明度），紧凑底栏与中心缩放/适配取代工具条适合窗口按钮。
- 最新图标列 / 临时面板：`docs/icon-rail-design.md` / `docs/icon-rail-validation.md`；主工作区边缘优先归位、唯一图标折叠模式与状态继承、图标直接拖出、侧边临时窗切换/滑动/空白关闭、单画布浮窗一行标题。
- 最新窗口恢复 / 边缘吸附：`docs/window-recovery-design.md` / `docs/window-recovery-validation.md`；最小化恢复无边框与最大化状态、唯一临时面板外部输入/失焦关闭、统一 24px 最近边吸附和窄边发光、默认双列布局，参考图预设已删除。
- 最新任务栏 / 原生标题残留：`docs/taskbar-window-design.md` / `docs/taskbar-window-validation.md`；显式系统最小化能力、主窗非客户区绘制过滤、恢复后真实合成像素回归、灰黑细分隔线。
- 最新菜单底层圆角模糊：`docs/menu-blur-layer-design.md` / `docs/menu-blur-layer-validation.md`；独立非交互原生窗口、HostBackdropBrush 与合成视觉圆角裁剪、最小化前隐藏、最大化拖动恢复普通窗口并跟随鼠标。
- 最新几何选区：`docs/selection-design.md` / `docs/selection-validation.md`；ABI 1.8、有界几何、笔刷/擦除/蒙版限制、历史与 ORA v4、QML 静态轮廓。
- 最新画笔库 / 设置关联：`docs/brush-library-design.md` / `docs/brush-library-validation.md`；共享活动预设、真实异步引擎预览、直径/间距单位转换、后台保存；高级笔尖未实现。
- 当前任务/CPU 视口与异步 ABI：`docs/task-render-design.md` / `docs/task-render-validation.md`，Qt 不再使用 C++ 绘画队列或整张文档缓存。
- 最新绘画性能修复：`docs/painting-performance-design.md` / `docs/painting-performance-validation.md`；有界 Weak 瓦片缓存、稀疏输出、QML 图层通知与原生性能基准。
- 最新剪贴/并排蒙版/预览修复：`docs/clipping-design.md` / `docs/clipping-validation.md`；ABI 1.7，Alt 层间点击、动态连续同级剪贴、ORA v3、边缘 LOD 正确采样。
- 最新蒙版/拖放/默认背景：`docs/masks-design.md` / `docs/masks-validation.md`；ABI 1.6，图层/组蒙版、拖放排序/归组、深度、隐式白色“背景”、中文命名、ORA 原始树与兼容预览。
- 最新图层组/透明背景：`docs/groups-design.md` / `docs/groups-validation.md` / `docs/transparency-design.md`；ABI 1.5、隔离嵌套、锁继承、结构历史/子树删除、嵌套 ORA、QML 折叠/浮窗与棋盘格。蒙版已在 ABI 1.6 完成。
- 图层控制：`docs/layers-design.md` / `docs/layers-validation.md`；ABI 1.4、27 种混合、三种锁、位置/填充、可见行后台预览、ORA 扩展，fx 未实施。
- 最新暂存盘配置：`docs/storage-settings-design.md` / `docs/storage-settings-validation.md`；paint-storage、ABI 1.3、后台检查与租约清扫，设置重启生效。
- 最新文档分页：`docs/document-paging-design.md` / `docs/document-paging-validation.md`；256MiB 为像素驻留缓存，冷页磁盘后备，快照/历史保持不可变身份。
- 最新历史存储：`docs/history-storage-design.md` / `docs/history-storage-validation.md`；大笔触无损编码与磁盘暂存，不再按内存历史预算拒绝笔触。

约束：Rust 不依赖 Qt；UI 只经 PaintCoreClient 使用 C ABI；Python 不访问 Qt/UI 线程。
任何新模块先写设计与验收条件，再实现。禁止用空实现或假成功代替未支持的功能。
所有行为修改补对应层回归测试（Rust / Qt Test / pytest）；尚未存在的层不创建虚假测试。
仓库统一验证入口：`python tools/check.py`（默认含 Qt，--qt-prefix 可指定 SDK）；--core-only 仅表示核心部分验证，不允许跳过失败项后报告完整通过。
ABI 破坏性修改升主版本；生成头由 cbindgen 管理，不手工修改生成物。
只有 paint-ffi 允许经过审查的 unsafe；caller-memory 读写集中在 pointers.rs，回调注明安全约束。
不要把开发机绝对路径写入构建配置；不要提交构建输出或工具链。
