# 图层操作验收

- Rust：组与蒙版复制、跨文档粘贴、ID 独立、编辑不串像素、原子 undo/redo（含磁盘历史）；单层向下、多选子树、组合并；锁与损坏剪贴板拒绝且状态保持。
- Rust/Qt：拖到组/图层下边缘，组内目标继承父组，自己边缘无变化、真实循环拒绝。
- Qt：右键分支、主菜单、缩略图/蒙版焦点、快捷键及文本输入不被拦截；只运行对应最小回归和构建。

执行结果（2026-10-12）：

- `cargo test --manifest-path core/Cargo.toml -p paint-core --test layer_actions --locked`：3/3 通过；复制包含组/蒙版、跨文档、独立编辑、损坏数据与父锁拒绝、磁盘历史 undo/redo，单层/组/多选合并，下边缘归组与自身边缘无变化。
- `cargo test --manifest-path core/Cargo.toml -p paint-ffi --test async_session layer_actions_copy --locked`：1/1 通过；FIFO、输入立即复制、跨 session、独立历史与无效字段拒绝。
- Release 构建通过，生成头严格检查及 C/C++ 静态/动态 ABI 链接构建通过。
- Qt `layerContextClipboardFocusShortcutsAndLowerEdgeDrop`：Windows 原生窗口、100%/200% 缩放各通过，无 QML 警告。真实右键分支、Ctrl+C/V/J/E、Delete 蒙版与所有者区分、白色预览框/无行外框、文本输入保护、真实下边缘拖放及目标组继承、主菜单复制组与撤销。
- 依用户要求仅执行最小针对性测试，未运行全量 `tools/check.py`。

