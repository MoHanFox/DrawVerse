# Qt 分组状态验收

日期：2026-10-17。本文先记录 ABI1.54 state20 在 Qt `BrushLibrary`、`PaintCoreClient`、设置持久化和画笔库树显示的镜像；后续补充同一入口的上下文菜单分组管理。分组包专用操作不在本文范围内。

## 已验证

- `BrushLibrary` 对 ordered-tree-v1 组织树进行严格校验：稳定 ID、DFS 顺序、嵌套深度、重复成员、环、孤立成员、容量和空组均受限；非法输入保持原状态。
- 非空组织使用 state20，并完整镜像到 `snapshot`、`catalogSnapshot`、`resetState` 和 `applyNativeCatalog`；显式平组织可规范化为空组织，未分组状态继续保持旧字段行为。
- 复制预设追加到源预设所在的直接组，删除预设只移除叶子并保留空组；重置只恢复参数基准，不移动组织位置。平包和 ABR 导入保留现有组织并将新预设追加到根。
- `PaintCoreClient` 的准备、后台 canonical state 回写和重启恢复均保留 organization 字段，并使用 `paint_brush_library_prepare_state_groups`。
- 画笔库面板消费 `organization` 的 ordered-tree-v1 投影：组行显示在子预设之前，嵌套预设按深度缩进；搜索只保留匹配预设及其必要的父组，平铺旧状态仍按原顺序显示。
- 组和预设的右键菜单支持新建根组/子组、改名、移动到其他组或根目录、解散组及级联删除；底栏按钮可以新建根组。
- Qt `BrushLibrary` 操作先构造候选树，再统一规范化和校验后采用；非法移动、内置画笔级联删除和空白/超长组名均保持快照不变。解散组会把子项插回原位置，级联删除会同步清除自定义预设及其重置基准。
- 空组在未搜索时保留在库树中；搜索时仍只展示匹配画笔及必要父组。移动和创建通过菜单完成，目前没有拖放排序交互。

## Windows 最小结果

Release 构建通过。QtTest：

- `brushGroupsState20MirrorBaselinesCopyDeleteAndFlatImports`
- `brushGroupsInvalidTreesAreAtomicAndExplicitFlatNormalizes`
- `brushGroupsClientPreparationPixelsHistoryAndRestart`
- `brushGroupsAppearInLibraryTreeAndSearchKeepsParent`
- `brushGroupsQtOperationsAreAtomicAndPreserveOrder`

结果：6 passed，0 failed，0 skipped（含 init/cleanup）。

## 未完成

拖放排序、Qt 分组包整树副本合并及完整范围导出仍未接入。ABR 纹理资源可以导入并保留来源，但 ABR 内部纹理参数到执行引擎的完整映射仍未支持；兼容报告或来源保留不等于已执行。跨平台验收按当前计划后置。
