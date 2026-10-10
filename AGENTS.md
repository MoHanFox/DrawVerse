# DrawVerse

跨平台绘画应用：Rust 内核 → 稳定 C ABI → C++20 / Qt 6.5+ UI；Python 插件最后实施。

- 当前阶段与验证命令：`README.md`、`docs/roadmap.md`。
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
