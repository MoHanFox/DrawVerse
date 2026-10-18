# ABR 大包兼容、画笔多选与选中导出验收

2026-10-18，Windows，对应[设计](../../../brush/ui/brush-library-abr-multiselect-export-design.md)。

- 用户提供的 `MOHAN b1.2.abr` 为 34,236,324 字节，实际经过 Windows Qt 导入成功：485 个画笔、304 个资源全部进入 `MOHAN b1.2` 文件组。
- Rust `abr_conversion` 35 项、`abr_import` 12 项通过；新增极端直径/间距/圆度钳制与报告测试，既有错误范围和批次原子性测试已更新并保持通过。
- Qt `brushMultiSelectionGroupsMixedItemsAndExportsSelectedTree` 通过：Ctrl 多选提示、混选成组、组内去重、选中组子树导出、纯画笔平包导出和再导入。
- Qt `brushMultiSelectionBlocksClientPainting` 通过：多选时 `beginStroke` 被拒绝，恢复单选后可以正常绘画。
- Qt `brushSelectedGroupVbrRoundtrip` 通过：右键组导出所走的选中导出 API 生成实际 VBR，并在新库中恢复组和子画笔。
- Qt `brushLibraryShiftControlSelectionAndRootReorder` 通过：Ctrl/Shift 选择、紧凑右键菜单、无组根级拖动和真实 DropArea 路径无 QML 警告。
- 既有组树/滚动、拖拽滚轮、纹理预览版本和 ABR 来源范围测试通过。Windows Release 完整构建成功。

未运行全量测试、跨平台、实体数位板或 GPU 验收。
