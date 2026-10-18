# 画笔组拖放排序验收

2026-10-18，Windows，对应[设计](../../../brush/library/brush-groups-drag-design.md)。本阶段只验证 Qt 组织排序和界面加载，不改 state/VBR/ABI。

2026-10-18 拖动修复：画笔库按图层列表的模式处理位置调整——拖到组中部显示整行边框并进入组，拖到条目上半/下半显示上方/下方插入线；列表 delegate 的鼠标区域启用 `preventStealing`，拖动开始后暂停 ListView 滚动，避免竖直手势被列表滚动吞掉；拖动靠近列表上下边缘自动滚动，空区/底部区域可移回根目录，释放/取消都会复位拖动状态。QML 实际加载与组树搜索回归通过；真实鼠标跨行拖放仍未自动化。

- `qt_ui_tests.exe brushGroupsQtOperationsAreAtomicAndPreserveOrder` 通过：指定位置移动预设与组、同父索引修正、组移动回原组、环移动拒绝以及原有删除/重命名原子行为。
- `qt_ui_tests.exe brushGroupsAppearInLibraryTreeAndSearchKeepsParent` 通过：真实加载 `Main.qml`/画笔库面板无 QML 警告，搜索仍保留父组。
- Windows Release 构建和既有 C/C++ ABI 烟测通过；组织文件仍为 state20/VBR1.24，未新增字段。

边界：Qt 测试覆盖拖放调用的 C++ 排序 API 和 QML 加载，未用真实鼠标自动化模拟整段跨行拖放；实际拖放命中区域与窄列视觉由用户验收。没有运行全量 `tools/check.py` 或跨平台验收。
