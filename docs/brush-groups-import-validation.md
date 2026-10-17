# 画笔分组包副本树导入验收

2026-10-18，Windows，对应[设计](brush-groups-import-design.md)。本阶段完成 Qt 分组包副本树合并，不改变 state/VBR/ABI。

- `brushAllPackageExportPreservesTreeAndStableIds` 通过：完整导出后导入到已有同名组的另一库，预设数、组数和导入的 `Export copy` 保持；导入组有内容，同名旧组不被覆盖；连续导入两次仍生成两组副本树。
- `vbrImagePackageImportsIntoFreshStoreAndExecutesAfterOriginalRemoval` 与 `wetDualImageVbrFreshStoreAndReopenMatchCapturedPixels` 通过：平包经分组读入口返回显式平根后仍能导入图片/湿混双色资源并执行真实像素。
- `brushGroupsQtOperationsAreAtomicAndPreserveOrder` 和画笔库面板加载测试通过；完整导出/导入测试连续运行六次稳定。Windows Release 构建与 C/C++ ABI 烟测通过。

边界：未自动化真实鼠标点击导入按钮或跨进程文件监视；文件选择/错误提示沿用既有导入路径。没有运行全量 `tools/check.py` 或跨平台验收。
