# 画笔库完整范围导出验收

2026-10-18，Windows，对应[设计](brush-groups-export-design.md)。本阶段只新增完整范围导出，不改 ABI/state/VBR。

- Qt `brushAllPackageExportPreservesTreeAndStableIds` 通过：ordered-package-v1、全部预设数量、内置 ID 不含 `round-*`、UUID 唯一、组数量与组织树一致；真实 `PaintCoreClient::exportAllBrushes` 经 ABI1.55 写入 `.vbr` 成功。
- 既有画笔组增删/排序、双色、喷枪和画笔库面板加载回归通过；Windows Release 构建及 C/C++ ABI 烟测通过。

边界：完整范围导出已由后续[分组包导入验收](brush-groups-import-validation.md)完成闭环；没有运行全量 `tools/check.py` 或跨平台验收。
