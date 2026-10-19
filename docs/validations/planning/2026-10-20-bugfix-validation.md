# 2026-10-20 分项修复验收

对应[问题清单与设计](../../../planning/2026-10-20-bugfix-design.md)。在 master 按项提交并推送；以下只记录实际完成的项。

## 严重 1：自由变换退出

- 修复 Qt 漏发 active 关闭与 busy 结束通知；空图层开始失败后也恢复界面。
- 回归：qt_transform_tests::emptyTransformAndCancelPublishClosedState，覆盖空图层、取消、实际 Escape、QML 覆盖层和历史不增长。
- Windows Release 输出：build/qt/bin/Release。
- 不代表全量、跨平台或实体连续拖动验收。

## 普通 1：初始背景乱码

- 白色新文档的背景名称从错误编码字面量恢复为“背景”。不改用户已有文档名称。
- paint-core masks 的 initial_white_background_has_readable_name_without_allocating_pixels 通过；验证 4096×2160 名称、白底、零瓦片和零修订。
- Release 随本批验收构建更新到指定目录。

## 普通 18：变换交叉/退化拖动

- Qt 仅静默处理原生校验返回的交叉/退化 UPDATE，不采用无效控制点，其他错误保留。
- Windows Release Qt：invalidIntermediateTransformKeepsLastGeometryAndRecovers 通过；退化、交叉两种输入后控制点不变且无错误通知，后续有效移动和取消正常。
- 同跑 emptyTransformAndCancelPublishClosedState 通过，保留会话期间修订通知。

## 普通 20：微小套索

- 有限的重复/共线路径在 Qt 提交前成为无操作，保留既有选区、修订与历史，不弹错误。
- Windows Release Qt：degenerateLassoIsANoopAndSelfIntersectionStillWorks 通过，覆盖自由/多边形套索各三种退化路径，并验证零有向面积的自交套索仍可正常提交。

## 普通 14：变换紧凑菜单

- 右键菜单切换为项目共用 GlassMenu / GlassMenuItem。
- Windows Release Qt：emptyTransformAndCancelPublishClosedState 通过；实际取消菜单行高度不超过 32、padding 为 6，触发菜单取消及 Escape 均关闭覆盖层。

## 性能 3–5，第一步：导航不重建复杂选区

- 画布向 SelectionOverlay 提供明确文档尺寸；缩放与平移只更新显示，不重建布尔路径。
- Windows Release qt_selection_tests 六项（含初始化/清理）通过。4096×2160、48 条纵横交织矩形选区，100 次 zoom/rect 更新约 0.013ms，pathChanged 为零、路径保持，文档尺寸改变后才重新裁剪。此数值只含属性和几何更新，不含窗口渲染延迟。
- 原灰度边缘/对角岛/四组合、复杂蒙版边界、4096 点套索和裁剪回归通过。
- 复杂蚂蚁线逐帧绘制、AA 判断和实体导航体验仍待后续专项，不将本项扩大为所有选区性能已解决。

## 普通 7：大目录多选保持

- 只在原生目录同步路径保留仍存在的笔刷/组选择；常规文件 restore 仍恢复活动预设。
- Windows Release qt_workflow_tests::nativeCatalogRefreshPreservesLargeLibraryMultiSelection 通过；400 个合法预设加组，参数同步后两笔刷和一组仍被选中、活动 ID 正确、保持多选禁画；普通 restore 回到单选。
- 使用小型 workflow Qt 测试目标验证工作流，避免每次编译整个大型 UI 测试源文件。
