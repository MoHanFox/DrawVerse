# Qt 分组状态验收

日期：2026-10-17。范围是 ABI1.54 state20 在 Qt `BrushLibrary`、`PaintCoreClient` 和设置持久化之间的镜像，不包含分组管理界面或分组包专用操作。

## 已验证

- `BrushLibrary` 对 ordered-tree-v1 组织树进行严格校验：稳定 ID、DFS 顺序、嵌套深度、重复成员、环、孤立成员、容量和空组均受限；非法输入保持原状态。
- 非空组织使用 state20，并完整镜像到 `snapshot`、`catalogSnapshot`、`resetState` 和 `applyNativeCatalog`；显式平组织可规范化为空组织，未分组状态继续保持旧字段行为。
- 复制预设追加到源预设所在的直接组，删除预设只移除叶子并保留空组；重置只恢复参数基准，不移动组织位置。平包和 ABR 导入保留现有组织并将新预设追加到根。
- `PaintCoreClient` 的准备、后台 canonical state 回写和重启恢复均保留 organization 字段，并使用 `paint_brush_library_prepare_state_groups`。

## Windows 最小结果

Release 构建通过。QtTest：

- `brushGroupsState20MirrorBaselinesCopyDeleteAndFlatImports`
- `brushGroupsInvalidTreesAreAtomicAndExplicitFlatNormalizes`
- `brushGroupsClientPreparationPixelsHistoryAndRestart`

结果：5 passed，0 failed，0 skipped（含 init/cleanup）。

## 未完成

分组管理控件、拖动/建组/解组用户入口、Qt 分组包整树副本合并及完整范围导出仍未接入。ABR 纹理资源可以导入并保留来源，但 ABR 内部纹理参数到执行引擎的完整映射仍未支持；兼容报告或来源保留不等于已执行。跨平台验收按当前计划后置。
