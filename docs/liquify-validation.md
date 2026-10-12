# 液化验收

- 核心：推拉、膨大、扭曲与重建实际像素；密度/强度/速度/压力改变结果；重复预览、局部重建与完整重置；取消精确还原且 redo 保留；确认单条历史与 undo/redo；选区、位置/透明锁、偏移/瓦片边界、资源及非法参数原子拒绝。
- FFI：参数边界、指针复制、FIFO 事务、活动 flag、确认/取消和历史。
- Qt：滤镜入口、工具详情栏与图标文字、参数默认及保存恢复、压感图标、鼠标与数位笔、静止膨大/扭曲、主窗/浮动画布、取消/重置/确定、预览不污染保存状态、普通编辑隔离。

执行结果（2026-10-12）：

- `cargo test --manifest-path core/Cargo.toml -p paint-core --test liquify --locked`：7/7 通过，四种变形、参数/压力、原始取样与精确重建、单条历史与 redo 保留、选区/偏移/蒙版/继承锁、资源拒绝、显示专注与无效输入。
- `cargo test --manifest-path core/Cargo.toml -p paint-core --lib liquify --locked`：1/1 通过，注入历史写入失败仍保留可取消的预览，磁盘历史 undo/redo 位级还原。
- `cargo test --manifest-path core/Cargo.toml -p paint-ffi --test async_session liquify --locked`：2/2 通过，DTO/指针校验与立即复制、真实预览、状态/历史、普通编辑 BUSY、文件拒绝结束后可再次正常保存。
- Qt `liquifyCanvasTransactionOptionsAndDocumentLock`：Windows 原生窗口、100%/200% 各通过；滤镜入口、工具图标、顶部详情栏（主窗与浮窗）、专注图层真实渲染、默认参数与重启恢复、鼠标/压力笔与按住膨大、重置/确认/取消、预览不标记 modified、单条历史与精确撤销、禁止文档切换/新建及浮窗隐藏恢复，无 QML 警告。
- 焦点通知拆分后，Qt `layerContextClipboardFocusShortcutsAndLowerEdgeDrop` 在 100%/200% 各通过；文本 Delete 与图层快捷键继续正确。
- `cargo clippy -p paint-core -p paint-ffi --all-targets --locked -- -D warnings` 与 rustfmt 检查通过；将既有剪贴板编解码辅助函数移到测试模块前，修复 items_after_test_module 警告（无行为改动）。
- Release 构建、cbindgen 严格字节检查及 C/C++ 静态/动态 ABI 构建与 4/4 运行通过（176 字节 DTO 布局和液化新导出函数）。
- 本轮执行液化与直接受影响的焦点回归，未运行全量 `tools/check.py`。

