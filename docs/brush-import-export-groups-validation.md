# 画笔导入导出保留分组验收

2026-10-18，Windows，对应[设计](brush-import-export-groups-design.md)。验证范围为本阶段新增的 Qt 文件导入导出分组行为。

- `brushPackageImportWrapsFileNameAndKeepsInnerGroups` 通过：单笔刷位于 `Inner/Nested` 时导出，再以文件名 `Imported File` 导入，根目录出现外层组，外层组内保留 `Inner`，`Inner` 内保留 `Nested`，`Nested` 内保留导入画笔。
- `brushAllPackageExportPreservesTreeAndStableIds` 通过：导出全部画笔后以文件名 `all` 导入目标库，外层文件名组与原有 `Export group` 同时存在；重复导入继续保留原有组树，并增加独立外层组。
- 两条用例使用 Windows Release Qt 测试目标，临时目录置于 `artifacts/tmp-ui`；Release 构建成功。没有运行全量测试、实体数位板、跨平台或 GPU 验收。

ABR 导入使用同一外层组逻辑，但 ABR 文件不携带 DrawVerse 分组；带组织的 VBR 才可恢复内部组树。
