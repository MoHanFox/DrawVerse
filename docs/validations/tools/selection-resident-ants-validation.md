# 常驻选区蚂蚁线验收

日期：2026-10-20；master；Windows 11 / Qt 6.8.3 MSVC Release。设计见[常驻遮罩](../../tools/selection-resident-ants-design.md)。

- 完整 drawverse Release 与 qt_transform_tests、qt_selection_tests、qt_workflow_tests 构建通过，输出 build/qt/bin/Release；无 ABI/格式变化，生成头检查通过。
- 真实 D3D11：residentSelectionAntsKeepTextureDuringNavigation 通过，无跳过。4096×2160、48 条横纵交织矩形，2836 个路径元素；实际呈现 12 次平移/缩放，maskRasterizations 为 1→1，路径身份不变，相位继续推进。实际图像的黑/白边缘与透明内部断言通过，截图人工检查通过；不把测试耗时当帧延迟。
- 软件：selectionMaskAnimationDoesNotRasterizeAgain、selectionPresentationRetainsSoftwareAndLargeDocumentFallback 通过。相位推进不重画静态 mask，修改选择会重画；软件与宽度 8192 的文档走向量回退。
- qt_selection_tests 4 项（含初始化/清理共 6）通过：复杂几何的 100 次导航、灰度/对角/布尔组合、复杂 mask 边界、密集套索和裁剪。固定样本导航 0.010ms，复杂 mask 构建 0.762ms，均不含 UI 帧呈现。

CPU 仍计算魔棒/套索与选区覆盖率，GPU 只显示既有半覆盖边界；AA 并不删除细小选区。初次路径布尔/栅格化、超过 4096 的文档、软件高复杂度显示尚可能耗时。低倍 mipmap 为视觉近似；未跑全量、跨平台或实体连续输入。画笔、变换物化没有因此获得 GPU 计算加速。
