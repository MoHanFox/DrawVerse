# 面板标签双击收回与共享栏语义（2026-10-10）

本轮针对用户第 6 条的残留缺口：面板标签（非页头拖动栏）双击时，在“侧边展开窗 / 浮动列”里会把来源图标列重新展开。用户要求双击面板顶部拖动区域只收起面板，不得顺便展开所属面板列。

## 现状

`ui/qml/PanelGroup.qml` 有两处双击入口，共享同一 `PanelGroup` 渲染组件，因此停靠列、浮动列、侧边展开窗行为必须一致：

- 页头拖动栏 `groupGrip`（第 91 行）：`root.flyout ? dismissRequested() : setColumnCollapsed(id,true)`，展开窗只收回窗口，已经正确。
- 标签 `panelTabGrip`（第 160 行）：`dismissRequested(); setColumnCollapsed(id,!root.flyout)`。展开窗里 `!flyout === false`，于是把来源图标列重新展开，属于缺陷；停靠列里双击则正确收回列。

## 修改

标签双击统一为“只收起当前面板呈现，绝不展开来源列”：

- 展开窗（flyout）：`dismissRequested()` 关闭该展开窗，并保持来源图标列仍为折叠状态。
- 停靠/浮动列：`setColumnCollapsed(group,true)` 折叠为图标，与页头双击一致。
- 不改变单击选择标签、标签拖动换位/拆分、页头按住拖动整列的既有语义。

## 共享渲染约束

停靠、浮动、侧边展开三种呈现继续全部走同一个 `PanelContent`（正文）与 `PanelGroup`（页头/标签/正文容器），本轮只改标签双击分支，不新增第二套面板渲染路径，也不新增 QML 文件。

## 验收

在既有 `ui/tests/ui_tests.cpp`（Qt Test，普通 DPI 与两倍 DPI 两个 CTest 都跑同一二进制）中新增真实回归：

1. 侧边图标列处于折叠状态；点击图标打开类别展开窗；双击该窗内某个标签 → 窗口消失，且该标签所属整列仍为折叠（`groupDefinition(id).icons === true`、`columnGroups(id)` 全部为图标态），双击前已打开的其它类别展开窗不受影响。
2. 同一场景下双击页头拖动栏仍只收回窗口、不展开列（防回归）。
3. 停靠列内双击标签仍折叠该列为图标（既有语义不回归）。

同轮补齐三条本已实现但缺少断言的行为（用户第 11、25、27 条），断言只描述现状，不改实现：

4. 浮动展开窗内栏底色：顶层组栏为 `#05080A` 75% 不透明度，其下各组栏为不透明 `#05080A`。
5. 文本输入框获得焦点时按空格只输入空格，画布不平移、不产生笔触与历史。
6. 历史列表逐条回放定时器间隔仍为 20ms、每 tick 只前进一步。

## 兼容性

纯 Qt/QML 与 Qt Test 变更；Rust、C ABI 1.10、布局 v6 结构、设置文件格式、历史回放语义均不变。完整 `python tools/check.py`（普通与两倍 DPI）通过后提交并推送 `origin/master`，随后重新部署 `build/qt/bin/Release/drawverse.exe`。
