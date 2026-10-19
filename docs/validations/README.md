# 验收文档索引

- [Warp强折叠预览与确认一致性](tools/transform-fold-consistency-validation.md)

- [自由变换与网格变形常驻预览](tools/transform-resident-preview-validation.md)

- [复杂图像套索、魔棒与油漆桶性能](tools/selection-complexity-performance-validation.md)

- [图层移动实时预览](tools/layer-move-preview-validation.md)

- [平滑画布导航与自适应笔尖轮廓](interface/canvas-navigation-adaptive-cursor-validation.md)

- [液化常驻纹理预览](tools/liquify-resident-preview-validation.md)

- [GPU 算子与弹性液化](tools/liquify-gpu-operators-validation.md)

- [液化有界并行](tools/liquify-bounded-parallel-validation.md)

- [大目录切换与起笔延迟](brush/performance/brush-library-switch-latency-validation.md)

- [参数与资源布局](brush/ui/brush-settings-resource-layout-validation.md)

- [整笔描边 Qt](brush/ui/brush-stroke-outline-ui-validation.md)

所有独立验收记录集中在 `docs/validations/`，按功能领域与设计目录对应分类，保留原文件名、日期、测试结果和未完成范围。设计及说明仍位于 `docs/brush/`、`docs/tools/` 等领域目录；总入口见 [文档索引](../README.md)。

记录描述当时实际执行的范围，不代表当前全量测试或所有平台均通过。当前结构整理及已知失败项见 [源码与契约目录调整验收](platform/source-layout-validation.md)。

本轮用户反馈的逐步测试与提交范围见[分步执行记录](planning/brush-feedback-execution-validation.md)，依赖顺序见[执行设计](../planning/brush-feedback-execution-design.md)。

| 分类 | 文档数 | 内容 |
| --- | ---: | --- |
| [笔刷核心](brush/core/) | 6 | 笔尖、遮罩、混合与参数模型 |
| [笔刷动力学](brush/dynamics/) | 22 | 压力、散布、形状、颜色、平滑与设备动态 |
| [双重笔刷](brush/dual/) | 11 | 双笔尖模型、尺寸、覆盖组合与桥接 |
| [湿混与涂抹](brush/wet/) | 24 | 载色、涂抹、湿混、喷枪与起收笔 |
| [ABR 导入](brush/abr/) | 18 | 描述符、采样、来源关联与参数转换 |
| [VBR 格式](brush/vbr/) | 5 | VBR 1.0、资源包、来源与兼容 |
| [画笔库与分组](brush/library/) | 21 | 目录、资源、组织树、持久化与导入导出 |
| [笔刷输入](brush/input/) | 7 | 输入分页、设备姿态与 Windows 路由 |
| [笔刷界面](brush/ui/) | 31 | 画笔设置、库面板、光标、预览和菜单 |
| [纹理](brush/texture/) | 5 | 覆盖模式、资源、深度与来源 |
| [笔刷性能](brush/performance/) | 2 | 大笔刷、密集笔触和图片笔尖 |
| [工具](tools/) | 11 | 选区、魔棒、油漆桶、变换、液化与吸色 |
| [图层](layers/) | 9 | 图层栈、组、蒙版、剪贴与批量操作 |
| [文档](document/) | 6 | 多文档、历史、暂存与文件 IO |
| [界面](interface/) | 13 | 工作区、停靠、面板、菜单与窗口 |
| [平台与工程](platform/) | 5 | 源码结构、FFI、任务渲染与性能 |
| [整体与阶段](planning/) | 3 | 首轮内核、笔刷系统与阶段修复 |

共 199 份验收记录。

## 笔刷核心

- [整条笔触描边分阶段验收](brush/core/brush-stroke-outline-validation.md)
- [整条笔触描边保存交换验收](brush/core/brush-stroke-outline-io-validation.md)
- [整条笔触描边原生入口验收](brush/core/brush-stroke-outline-abi-validation.md)

- [画笔混合原生入口验收](brush/core/brush-blend-abi-validation.md)
- [画笔颜色混合内核验收](brush/core/brush-blend-validation.md)
- [颜色动态IO验收](brush/core/brush-color-io-validation.md)
- [设备笔尖动态保存与交换验收](brush/core/brush-device-io-validation.md)
- [图片笔尖内核验收](brush/core/brush-mask-validation.md)
- [程序笔尖第一阶段验收](brush/core/brush-tip-validation.md)

## 笔刷动力学

- [颜色动态内核验收](brush/dynamics/brush-color-dynamics-validation.md)
- [设备与路径笔尖动态内核验收](brush/dynamics/brush-device-dynamics-validation.md)
- [笔触方向动态验收](brush/dynamics/brush-direction-validation.md)
- [笔触距离动态验收](brush/dynamics/brush-distance-validation.md)
- [动态来源矩阵验收](brush/dynamics/brush-dynamics-matrix-validation.md)
- [笔刷渐隐动态验收](brush/dynamics/brush-fade-validation.md)
- [独立不透明度压感验收](brush/dynamics/brush-opacity-pressure-validation.md)
- [压力动态IO验收](brush/dynamics/brush-pressure-io-validation.md)
- [自动平滑压力曲线验收](brush/dynamics/brush-pressure-smooth-validation.md)
- [压力动态内核验收](brush/dynamics/brush-pressure-validation.md)
- [笔旋转动态验收](brush/dynamics/brush-rotation-validation.md)
- [散布设备控制验收](brush/dynamics/brush-scatter-device-validation.md)
- [散布IO验收](brush/dynamics/brush-scatter-io-validation.md)
- [散布内核验收](brush/dynamics/brush-scatter-validation.md)
- [形状抖动IO验收](brush/dynamics/brush-shape-jitter-io-validation.md)
- [形状抖动内核验收](brush/dynamics/brush-shape-jitter-validation.md)
- [时间平滑IO验收](brush/dynamics/brush-smoothing-io-validation.md)
- [时间平滑内核验收](brush/dynamics/brush-smoothing-validation.md)
- [速度动态验收](brush/dynamics/brush-speed-validation.md)
- [独立笔刷尺寸 / 整笔不透明度验收](brush/dynamics/brush-stroke-opacity-validation.md)
- [切向压力动态验收](brush/dynamics/brush-tangent-validation.md)
- [倾斜幅度动态验收](brush/dynamics/brush-tilt-validation.md)

## 双重笔刷

- [原生第二笔尖资源生命周期验收](brush/dual/brush-dual-bridge-validation.md)
- [双色双笔刷整笔合成内核验收](brush/dual/brush-dual-color-core-validation.md)
- [双色双笔刷保存与交换验收](brush/dual/brush-dual-color-io-validation.md)
- [双色整笔重算性能验收](brush/dual/brush-dual-color-performance-validation.md)
- [双笔刷覆盖模式扩展验收](brush/dual/brush-dual-coverage-modes-validation.md)
- [双笔刷状态与 VBR 交换验收](brush/dual/brush-dual-io-validation.md)
- [第二笔尖独立路径验收](brush/dual/brush-dual-path-validation.md)
- [第二笔尖尺寸保存与交换验收](brush/dual/brush-dual-size-io-validation.md)
- [第二笔尖相对尺寸内核验收](brush/dual/brush-dual-size-validation.md)
- [逐印章双笔刷双色验收](brush/dual/brush-dual-stamp-validation.md)
- [双笔尖覆盖核心验收](brush/dual/brush-dual-validation.md)

## 湿混与涂抹

- [时间喷枪验收](brush/wet/brush-airbrush-validation.md)
- [混合与双笔刷性能验收](brush/wet/brush-mix-performance-validation.md)
- [起收笔原生入口验收](brush/wet/brush-taper-abi-validation.md)
- [起收笔实际绘画验收](brush/wet/brush-taper-document-validation.md)
- [起收笔预设与交换验收](brush/wet/brush-taper-io-validation.md)
- [起收笔曲线与笔尾调度验收](brush/wet/brush-taper-kernel-validation.md)
- [起收笔 Qt 验收](brush/wet/brush-taper-ui-validation.md)
- [湿混原生入口 ABI1.50 验收](brush/wet/brush-wet-abi-validation.md)
- [二维湿混载色内核验收](brush/wet/brush-wet-carrier-validation.md)
- [湿混与颜色动态验收](brush/wet/brush-wet-color-validation.md)
- [湿混文档笔触验收](brush/wet/brush-wet-document-validation.md)
- [湿混双色原生入口验收](brush/wet/brush-wet-dual-abi-validation.md)
- [湿混双色 Document 验收](brush/wet/brush-wet-dual-document-validation.md)
- [湿混双色 Engine 归还验收](brush/wet/brush-wet-dual-engine-validation.md)
- [湿混双色保存交换验收](brush/wet/brush-wet-dual-io-validation.md)
- [湿混双色 Qt 组合入口验收](brush/wet/brush-wet-dual-ui-validation.md)
- [湿混双色贡献场性能验收](brush/wet/brush-wet-field-performance-validation.md)
- [湿混状态与VBR交换验收](brush/wet/brush-wet-io-validation.md)
- [湿混共享工具载色与原生会话验收](brush/wet/brush-wet-owner-validation.md)
- [湿混双载色状态验收](brush/wet/brush-wet-pair-validation.md)
- [湿混 Qt 参数与真实绘画验收](brush/wet/brush-wet-qt-validation.md)
- [湿混统一设置与载色策略验收](brush/wet/brush-wet-settings-validation.md)
- [二维湿混涂抹验收](brush/wet/brush-wet-smudge-validation.md)
- [涂抹及参数提示验收（2026-10-12）](brush/wet/smudge-validation.md)

## ABR 导入

- [ABR样本关联验收](brush/abr/abr-association-validation.md)
- [ABR颜色动态转换验收](brush/abr/abr-color-conversion-validation.md)
- [ABR基础转换验收](brush/abr/abr-conversion-validation.md)
- [ABR描述符读取验收](brush/abr/abr-descriptor-validation.md)
- [ABR设备形状转换验收](brush/abr/abr-device-conversion-validation.md)
- [ABR设备来源映射验收](brush/abr/abr-device-source-validation.md)
- [ABR计算双笔刷验收](brush/abr/abr-dual-brush-validation.md)
- [ABR用户入口验收](brush/abr/abr-entry-validation.md)
- [ABR整包IO导入验收](brush/abr/abr-import-validation.md)
- [ABR 整笔不透明度验收](brush/abr/abr-opacity-validation.md)
- [ABR 笔压动态阶段验收](brush/abr/abr-pressure-validation.md)
- [真实ABR对齐与语料验收](brush/abr/abr-real-corpus-validation.md)
- [ABR 采样读取验收](brush/abr/abr-sampled-validation.md)
- [ABR散布转换验收](brush/abr/abr-scatter-conversion-validation.md)
- [ABR6/10采样分节验收](brush/abr/abr-sections-validation.md)
- [ABR形状抖动转换验收](brush/abr/abr-shape-conversion-validation.md)
- [ABR 纹理导入验收](brush/abr/abr-texture-validation.md)
- [ABR 传递通道随机抖动验收](brush/abr/abr-transfer-jitter-validation.md)

## VBR 格式

- [VBR 1.0 兼容性验收](brush/vbr/vbr-format-1.0-compatibility-validation.md)
- [VBR 图片资源验收](brush/vbr/vbr-image-validation.md)
- [VBR 程序笔刷包验收](brush/vbr/vbr-procedural-validation.md)
- [VBR来源交换入口验收](brush/vbr/vbr-source-entry-validation.md)
- [VBR1.2来源IO验收](brush/vbr/vbr-source-validation.md)

## 画笔库与分组

- [原生笔刷库 ABI 验收](brush/library/brush-catalog-abi-validation.md)
- [Qt / 原生修订桥验收](brush/library/brush-catalog-bridge-validation.md)
- [Rust 笔刷库模型验收](brush/library/brush-catalog-validation.md)
- [分组状态原生准备验收](brush/library/brush-groups-abi-validation.md)
- [画笔组拖放排序验收](brush/library/brush-groups-drag-validation.md)
- [画笔库完整范围导出验收](brush/library/brush-groups-export-validation.md)
- [画笔分组包副本树导入验收](brush/library/brush-groups-import-validation.md)
- [笔刷分组Rust目录验收](brush/library/brush-groups-kernel-validation.md)
- [分组目录合并验收](brush/library/brush-groups-merge-validation.md)
- [分组包原生交接验收](brush/library/brush-groups-package-validation.md)
- [分组导入资源发布验收](brush/library/brush-groups-persist-validation.md)
- [Qt 分组状态验收](brush/library/brush-groups-qt-state-validation.md)
- [分组state20保存验收](brush/library/brush-groups-state-validation.md)
- [VBR分组清单与副本映射验收](brush/library/brush-groups-vbr-validation.md)
- [画笔导入导出保留分组验收](brush/library/brush-import-export-groups-validation.md)
- [原生共享图片资源 ABI 验收](brush/library/brush-resource-abi-validation.md)
- [图片笔尖身份/状态验收](brush/library/brush-resource-state-validation.md)
- [图片笔尖持久资源层验收](brush/library/brush-resource-validation.md)
- [原生来源准备桥验收](brush/library/brush-source-bridge-validation.md)
- [原生预设来源与v4状态验收](brush/library/brush-source-state-validation.md)
- [笔刷来源资源验收](brush/library/brush-source-validation.md)

## 笔刷输入

- [复合笔尖原生几何验收](brush/input/brush-composite-pose-validation.md)
- [后台Actor输入分页验收](brush/input/brush-input-paging-actor-validation.md)
- [Document真实输入分页验收](brush/input/brush-input-paging-document-validation.md)
- [输入分页数值与预算验收](brush/input/brush-input-paging-kernel-validation.md)
- [实时动态光标验收](brush/input/brush-live-pose-validation.md)
- [数位笔悬停修复验收](brush/input/brush-tablet-hover-fix-validation.md)
- [Windows悬停与模式菜单验收](brush/input/windows-input-menu-validation.md)

## 笔刷界面

- [自动描边预设验收](brush/ui/brush-auto-outline-validation.md)
- [画笔混合Qt入口验收](brush/ui/brush-blend-ui-validation.md)
- [颜色动态原生Qt验收](brush/ui/brush-color-ui-validation.md)
- [可选复合光标Qt验收](brush/ui/brush-composite-cursor-ui-validation.md)
- [半透明图片光标Qt验收](brush/ui/brush-cursor-image-ui-validation.md)
- [半透明图片光标：数值与原生验收](brush/ui/brush-cursor-image-validation.md)
- [笔尖光标偏好验收](brush/ui/brush-cursor-settings-validation.md)
- [设备动态Qt/原生入口验收](brush/ui/brush-device-ui-validation.md)
- [双色双笔刷原生与Qt入口验收](brush/ui/brush-dual-color-ui-validation.md)
- [第二笔尖相对尺寸原生与Qt验收](brush/ui/brush-dual-size-ui-validation.md)
- [双笔刷原生与 Qt 入口验收](brush/ui/brush-dual-ui-validation.md)
- [平滑收尾、缩放快捷键与色轮保存验收](brush/ui/brush-follow-color-zoom-validation.md)
- [图片笔尖 Qt 接入验收](brush/ui/brush-image-ui-validation.md)
- [ABR 大包兼容、画笔多选与选中导出验收](brush/ui/brush-library-abr-multiselect-export-validation.md)
- [画笔库交互修复验收](brush/ui/brush-library-interaction-fixes-validation.md)
- [画笔库 / 当前画笔设置验收（2026-10-09）](brush/ui/brush-library-validation.md)
- [画笔菜单与展开组落点验收](brush/ui/brush-menu-drag-refinements-validation.md)
- [双色绘制删除与模式收窄验收](brush/ui/brush-mode-cleanup-validation.md)
- [图片笔尖轮廓验收](brush/ui/brush-outline-validation.md)
- [压力曲线入口验收](brush/ui/brush-pressure-ui-validation.md)
- [笔刷预览轮转与 ABR 间距验收](brush/ui/brush-preview-roundtrip-validation.md)
- [重置预设验收](brush/ui/brush-reset-validation.md)
- [散布原生Qt验收](brush/ui/brush-scatter-ui-validation.md)
- [形状抖动原生Qt验收](brush/ui/brush-shape-jitter-ui-validation.md)
- [时间平滑原生桥与Qt验收](brush/ui/brush-smoothing-ui-validation.md)
- [笔刷来源界面验收](brush/ui/brush-source-ui-validation.md)
- [纹理原生与 Qt 入口验收](brush/ui/brush-texture-ui-validation.md)
- [工具详细栏传递参数验收](brush/ui/brush-toolbar-transfer-validation.md)
- [固定白色预览验收](brush/ui/brush-white-preview-validation.md)
- [Qt Quick / QML UI 验证](brush/ui/qml-ui-validation.md)
- [参考图 QML 与停靠验收（2026-10-09）](brush/ui/reference-ui-validation.md)

## 纹理

- [原生纹理资源生命周期验收](brush/texture/brush-texture-bridge-validation.md)
- [纹理溶解模式验收](brush/texture/brush-texture-dissolve-validation.md)
- [纹理IO验收](brush/texture/brush-texture-io-validation.md)
- [纹理混合模式验收](brush/texture/brush-texture-mode-validation.md)
- [纹理内核验收](brush/texture/brush-texture-validation.md)

## 笔刷性能

- [非完全覆盖图片笔尖性能验收](brush/performance/brush-image-tip-performance-validation.md)
- [大笔刷密集笔触性能验收](brush/performance/brush-large-dense-performance-validation.md)

## 工具

- [油漆桶验收（2026-10-11）](tools/bucket-validation.md)
- [Ctrl 选层、搜索焦点与紧凑变换输入验收](tools/ctrl-layer-pick-validation.md)
- [吸色取色环验收（2026-10-12）](tools/eyedropper-ring-validation.md)
- [共用选区与套索验收（2026-10-11）](tools/lasso-selection-validation.md)
- [液化会话历史最小验收](tools/liquify-history-validation.md)
- [液化性能最小验收](tools/liquify-performance-validation.md)
- [大尺寸液化采样与基准验收](tools/liquify-large-brush-performance-validation.md)
- [液化验收](tools/liquify-validation.md)
- [框选与套索绘画/擦除性能验收（2026-10-11）](tools/selection-performance-validation.md)
- [几何选区验收（2026-10-09）](tools/selection-validation.md)
- [图片模型与自由变换验收](tools/transform-validation.md)
- [魔棒验收（2026-10-12）](tools/wand-validation.md)

## 图层

- [并排蒙版、剪贴与预览修复验收（2026-10-09）](layers/clipping-validation.md)
- [空图层栈验收](layers/empty-layer-stack-validation.md)
- [组穿透验收（2026-10-12）](layers/group-pass-through-validation.md)
- [图层组与透明背景验收](layers/groups-validation.md)
- [图层操作验收](layers/layer-actions-validation.md)
- [搜索与快捷键最小验证](layers/layer-search-tools-validation.md)
- [图层控制模块验收](layers/layers-validation.md)
- [蒙版、拖放和默认背景验收（2026-10-08）](layers/masks-validation.md)
- [菜单分类与图层批量修复验收（2026-10-11）](layers/menu-layer-batch-validation.md)

## 文档

- [文档分页验收（2026-10-08，Windows）](document/document-paging-validation.md)
- [多文档工作区与面板列验收（2026-10-09）](document/document-workspace-validation.md)
- [真实历史标签验收（2026-10-10）](document/history-labels-validation.md)
- [大笔触历史存储验收](document/history-storage-validation.md)
- [文件 IO / 工作区 v2 验收](document/io-validation.md)
- [暂存盘设置与残留清扫验收](document/storage-settings-validation.md)

## 界面

- [自由停靠验收（2026-10-09）](interface/flexible-docking-validation.md)
- [图标列与画布标题修复验收（2026-10-09）](interface/icon-rail-validation.md)
- [菜单底层圆角模糊与最大化拖动验收（2026-10-10）](interface/menu-blur-layer-validation.md)
- [菜单悬停与多选建组验收](interface/menu-hover-grouping-validation.md)
- [面板分组与统一内容验收（2026-10-10）](interface/panel-categories-validation.md)
- [面板折叠尺寸验收（2026-10-11）](interface/panel-fold-sizing-validation.md)
- [面板状态与紧凑信息栏验收（2026-10-10）](interface/panel-presentation-validation.md)
- [面板细化与悬停预览验收（2026-10-10）](interface/panel-refinement-validation.md)
- [面板标签双击收回验收（2026-10-10）](interface/panel-tab-collapse-validation.md)
- [常驻图标面板列验收（2026-10-10）](interface/persistent-panel-rails-validation.md)
- [任务栏与原生窗口恢复验收（2026-10-10）](interface/taskbar-window-validation.md)
- [窗口恢复 / 边缘吸附 / 标签排序验收（2026-10-09）](interface/window-recovery-validation.md)
- [工具列、稳定视口与交互验收（2026-10-10）](interface/workspace-tools-validation.md)

## 平台与工程

- [Release 目录权限验收](platform/release-directory-permissions-validation.md)

- [第 3 阶段验证记录：paint-ffi + cbindgen](platform/ffi-validation.md)
- [绘画 CPU 与延迟回退验收](platform/painting-performance-validation.md)
- [源码与契约目录调整验收](platform/source-layout-validation.md)
- [任务与 CPU 视口渲染验证](platform/task-render-validation.md)

## 整体与阶段

- [统一笔刷系统验收计划](planning/brush-system-validation.md)
- [遗留功能修复验收（2026-10-11）](planning/deepseek-recovery-validation.md)
- [第一轮验证记录](planning/validation.md)
