# DrawVerse 文档索引

- [复杂图像套索、魔棒与油漆桶性能](tools/selection-complexity-performance-design.md)

- [图层移动实时预览](tools/layer-move-preview-design.md)

- [GPU 加速规则、使用方法与代码入口](platform/gpu-acceleration-design.md)
- [平滑画布导航与自适应笔尖轮廓](interface/canvas-navigation-adaptive-cursor-design.md)

- [液化常驻纹理预览](tools/liquify-resident-preview-design.md)

- [GPU 算子与可恢复弹性液化](tools/liquify-gpu-operators-design.md)

- [液化有界并行](tools/liquify-bounded-parallel-design.md)

- [大目录切换与起笔延迟](brush/performance/brush-library-switch-latency-design.md)

- [参数与资源布局](brush/ui/brush-settings-resource-layout-design.md)

- [整笔描边 Qt](brush/ui/brush-stroke-outline-ui-design.md)

设计与说明按功能领域组织；所有独立验收记录统一位于 [validations](validations/README.md)，按相同领域分类。新增设计使用 `*-design.md`，新增验收使用 `*-validation.md`，并维护设计与验收之间的相对链接。

| 领域 | 设计与说明 | 验收记录 | 内容 |
| --- | --- | --- | --- |
| 笔刷核心 | [6 份](brush/core/) | [6 份](validations/brush/core/) | 笔尖、遮罩、混合与参数模型 |
| 笔刷动力学 | [22 份](brush/dynamics/) | [22 份](validations/brush/dynamics/) | 压力、散布、形状、颜色、平滑与设备动态 |
| 双重笔刷 | [12 份](brush/dual/) | [11 份](validations/brush/dual/) | 双笔尖模型、尺寸、覆盖组合与桥接 |
| 湿混与涂抹 | [24 份](brush/wet/) | [24 份](validations/brush/wet/) | 载色、涂抹、湿混、喷枪与起收笔 |
| ABR 导入 | [19 份](brush/abr/) | [18 份](validations/brush/abr/) | 描述符、采样、来源关联与参数转换 |
| VBR 格式 | [7 份](brush/vbr/) | [5 份](validations/brush/vbr/) | VBR 1.0、资源包、来源与兼容 |
| 画笔库与分组 | [21 份](brush/library/) | [21 份](validations/brush/library/) | 目录、资源、组织树、持久化与导入导出 |
| 笔刷输入 | [8 份](brush/input/) | [7 份](validations/brush/input/) | 输入分页、设备姿态与 Windows 路由 |
| 笔刷界面 | [31 份](brush/ui/) | [31 份](validations/brush/ui/) | 画笔设置、库面板、光标、预览和菜单 |
| 纹理 | [5 份](brush/texture/) | [5 份](validations/brush/texture/) | 覆盖模式、资源、深度与来源 |
| 笔刷性能 | [2 份](brush/performance/) | [2 份](validations/brush/performance/) | 大笔刷、密集笔触和图片笔尖 |
| 工具 | [16 份](tools/) | [11 份](validations/tools/) | 选区、魔棒、油漆桶、变换、液化与吸色 |
| 图层 | [10 份](layers/) | [9 份](validations/layers/) | 图层栈、组、蒙版、剪贴与批量操作 |
| 文档 | [8 份](document/) | [6 份](validations/document/) | 多文档、历史、暂存与文件 IO |
| 界面 | [15 份](interface/) | [13 份](validations/interface/) | 工作区、停靠、面板、菜单与窗口 |
| 平台与工程 | [6 份](platform/) | [5 份](validations/platform/) | 源码结构、FFI、任务渲染与性能 |
| 整体与阶段 | [5 份](planning/) | [3 份](validations/planning/) | 首轮内核、笔刷系统与阶段修复 |

## 常用入口

- 整笔描边：[核心设计](brush/core/brush-stroke-outline-design.md) / [保存交换](brush/core/brush-stroke-outline-io-design.md) / [原生入口](brush/core/brush-stroke-outline-abi-design.md) / [分阶段验收](validations/brush/core/brush-stroke-outline-validation.md) / [保存交换验收](validations/brush/core/brush-stroke-outline-io-validation.md) / [原生入口验收](validations/brush/core/brush-stroke-outline-abi-validation.md)。

- 启动与诊断：[开发者控制台](platform/developer-console-design.md)。

- 本轮反馈：[依赖与执行顺序](planning/brush-feedback-execution-design.md) / [逐步验收](validations/planning/brush-feedback-execution-validation.md)。
- 项目结构：[源码目录与依赖](platform/source-layout-design.md) / [构建运行验收](validations/platform/source-layout-validation.md)。
- 构建环境：[检出与构建复现](platform/build-reproducibility.md) / [Release 目录权限验收](validations/platform/release-directory-permissions-validation.md)。
- 笔刷：[系统设计](planning/brush-system-design.md) / [剩余功能与边界](planning/brush-system-remaining.md)。
- 文件格式：[当前 VBR 1.0 契约](../contracts/brush/vbr/format.md) / [兼容设计](brush/vbr/vbr-format-1.0-compatibility-design.md) / [验收](validations/brush/vbr/vbr-format-1.0-compatibility-validation.md)。
- 接口边界：[契约索引](../contracts/README.md) / [C ABI](../contracts/ffi/abi.md)。
- 全部验收：[分类索引](validations/README.md)。
