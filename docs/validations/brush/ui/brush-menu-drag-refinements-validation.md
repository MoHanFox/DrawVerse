# 画笔菜单与展开组落点验收

2026-10-18，Windows，对应[设计](../../../brush/ui/brush-menu-drag-refinements-design.md)。

- `brushLibraryShiftControlSelectionAndRootReorder` 通过：真实画笔 DropArea 行级回调和 footer 根级回调均无 `root is not defined` 或隐式参数警告；无组时根级重排正常。
- 同一用例验证独立 `brushPresetContextMenu` Window 弹层、紧凑菜单项、新导出文案，以及无组时“移动到组”禁用。
- `expandedGroupLowerDropInsertsFirstForBrushesAndLayers` 通过：画笔和图层都从展开组的标题下半部拖入，元素成为该组第一个子项，且没有 QML 警告。
- 既有画笔组树、搜索、滚动、滚轮拖动和多选测试继续通过。Windows Release 完整构建成功。

未运行全量测试、跨平台、实体设备或 GPU 验收。

2026-10-21追加：删除用户截图中尾部可见固定拖放条；同一`brushLibraryShiftControlSelectionAndRootReorder`断言旧条不存在，改在自然空白区真实拖放，软件与D3D11均通过，多选、行级重排、菜单和警告检查保持。Release构建成功；旧`drawverse.exe`被运行窗口锁定，主程序另输出同目录`drawverse-next.exe`，不关闭用户窗口。此次没有ABI或算法变化。
