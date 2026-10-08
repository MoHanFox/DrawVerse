# DrawVerse

跨平台绘画应用：Rust 内核 → 稳定 C ABI → C++20 / Qt 6.5+ UI；Python 插件最后实施。

- 当前阶段与验证命令：`README.md`、`docs/roadmap.md`。
- 架构、UI 停靠与兼容性：`docs/architecture.md`。
- 正式 C ABI：`core/crates/paint-ffi/src/api.rs` / `ui/include/paint_api.h`；契约在 `contracts/abi.md`。
- `contracts/paint-api-v1.draft.h` 仅为历史设计；FFI 方案/验收在 `docs/ffi-design.md`。
- 当前文件 IO 与布局 v2 清理：`docs/io-design.md` / `docs/io-validation.md`；ABI 1.2 文件任务、Qt 文件菜单，笔记已退役。
- 当前实现：`core/crates/paint-core/src`、`core/crates/paint-ffi/src`、`ui/src`、`ui/qml` 与对应测试。
- 最新 UI 设计/验收：`docs/qml-ui-design.md` / `docs/qml-ui-validation.md`；用户指定 QML，旧 Widgets 设计已被取代。
- 最新参考图 UI / 停靠：`docs/reference-ui-design.md` / `docs/reference-ui-validation.md`；无边框紧凑工作区、原生浮窗、工具条拖放、布局 v4。用户要求每个模块验证后 commit 并推送 origin/master，持续推进；Python 插件最后。
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
