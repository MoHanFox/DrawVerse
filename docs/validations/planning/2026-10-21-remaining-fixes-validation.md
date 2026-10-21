# 原始清单与 UI 回归统一修复验收

2026-10-21。[范围及当前状态](../../planning/2026-10-21-remaining-fixes-design.md)。

## 建立清单

逐项对照用户《新建 文本文档 (4).txt》、实现设计及验收，区分已完成、部分优化、只完成算子和未归因回归。U1–U4 的原失败见 welcome-workspace-validation，U2 独立复核仍失败，见 collapsed-panel-flyout-merge-validation。未把这些失败归类为既有问题。

本提交仅建立追踪入口，无运行时修改；未宣称上述待办已完成。后续在这里追加每项根因、改动、验证及限制。

## U3：快捷键列表旧断言

根因是测试仍要求早期26条，当前35条包含图层复制/粘贴、复制、合并、成组、删除、全选/取消/反选、移动工具九项。核对Main快捷键后，测试验证九项名称与键位、所有行三列非空及名称唯一、模型与实际行数一致，保留渲染像素和打开/关闭验证；没有删除产品快捷键迁就旧断言。

软件和D3D11下 `editMenuOpensKeyboardShortcutList` 各通过（含初始化清理3 passed），零QML警告。日志 artifacts/remaining-u3-{software,d3d11}.txt。Release/全部Qt及四ABI目标构建通过，生成头一致；无生产代码或ABI变化。其余待办保持。
