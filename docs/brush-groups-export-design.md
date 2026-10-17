# 画笔库完整范围导出

2026-10-18，在 ABI1.55 的 `package_write_groups` 和 state20 组织树基础上增加 Qt“导出全部画笔与分组”。导出不再只取当前预设，而是按当前画笔库顺序生成完整 `ordered-package-v1`：`package.brushes` 覆盖全部预设，`organization` 保留根目录、组、空组和顺序；内置预设的稳定 ID 映射为 VBR UUID，组织中的预设引用同步重映射。

每个预设沿用现有单笔刷导出字段和资源引用；主笔尖、第二笔尖、纹理、ABR 来源及行为能力保持。若任一预设包含原始 ABR 来源而用户未选择包含来源，导出拒绝而不是生成缺失来源的包。无组织树时生成显式平根组织；有组织树时保留现有组 ID，并验证导出预设投影。

`PaintCoreClient::exportAllBrushes` 在后台调用 `paint_brush_library_package_write_groups`，由 Rust 原子校验完整包、资源/来源、2MiB handoff 和 VBR 保存。面板底部新增“导出全部画笔与分组”按钮；来源选项弹窗在全部导出时检查所有预设。ABI/state/VBR 版本不增加。

边界：本阶段完成完整范围导出和文件写入；Qt 读取分组包后合并到当前库的副本树入口仍待接，现有导入入口继续按旧平包/单树规则处理。
