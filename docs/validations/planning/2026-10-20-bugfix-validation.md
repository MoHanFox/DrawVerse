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

## 普通 9：桶与涂抹同排滑条

- 两工具使用“标题、常驻整数滑条、百分比输入”的顺序，保留已有数值编辑。
- Windows Release qt_workflow_tests::toolPercentSlidersAndInputsStaySynchronized 通过；滑条 75% 更新工具参数及文本、输入 40% 更新滑条，并确认画笔不透明度未被修改。

## 普通 4：图层组行

- 组行改为 24px，取消组总预览请求和显示；像素层仍为 34px 并保留缩略图。
- Windows Release qt_workflow_tests::groupRowsAreCompactAndHaveNoAggregateThumbnail 通过，实际 QML 展开组与子层的行高和缩略图可见性符合要求。

## 普通 5：图层和组原位改名

- ABI 1.65，现有 group request 增补改名操作，结构历史只保存元数据并复用像素。首次单击的异步选层会临时置忙，双击改名等待其完成后再开启输入框，避免双击无效。
- Rust 核心 layer_and_group_rename_are_atomic_undoable_and_do_not_copy_pixels 通过：中文、像素层/组、undo/redo、同名无修订、非法名称拒绝、4K 零瓦片白底保持。
- 原生 layer_rename_abi_copies_utf8_and_preserves_undo 通过：返回前复制 UTF-8，调用者随后覆写源字节不影响结果；撤销重做保持。
- Windows Release Qt layerAndGroupNamesEditInlineAndEscapePreservesHistory 通过：真实双击、Enter 保存、Esc 放弃和图层/组历史恢复。
- 四个 C/C++ 静态/动态 ABI 消费者通过；生成头检查通过，DTO 布局保持。

## 普通 8：整数参数编辑

- 笔刷设置统一整数显示精度、整数输入校验与滑动提交；半径/归一化内部值仍按显示单位换算，曲线不变。
- Windows Release qt_workflow_tests::brushScalarControlsCommitIntegerDisplayUnits 通过：37.6 滑动提交 38，小数文本拒绝、整数接受，13 个分类内超过 40 个标量控件均为整数精度。
- 原双笔刷与起收笔 UI 用例的显示/赋值预期同步为整数；完整测试源重新编译后的相关用例继续补验。

## 普通 3：笔刷组闪烁

- 显示列表按稳定身份增删/移动行，展开与折叠不再替换整个模型和全部预览。
- Windows Release qt_workflow_tests::brushGroupCollapseKeepsUnchangedDelegates 通过；三轮展开/折叠的行数正确，未变化的笔刷 delegate 指针始终保持，没有 QML 警告。

## 普通 2：资源缩略图闪烁

- 可见项保留内容身份对应的不可变缩略图；全局缓存淘汰不再清空图片或重复排队，切换资源身份才请求新图。
- Windows Release qt_workflow_tests::visibleResourceThumbnailSurvivesSharedCacheEviction 通过；70 次全局完成信号及缓存淘汰后原图片保持、零重复请求，更换身份后只请求一次并正确采用新图。
- Release 程序已重建到 build/qt/bin/Release。测试覆盖缓存生命周期，实体滚动视觉仍需用户复核。

## 普通 6，第一步：笔刷组创建与原位改名

- 创建组直接分组所选笔刷，自动使用未占用的“笔刷组N”；根目录投放提示已移除。组名单击只选择，三角按钮折叠，双击名称原位编辑。
- Windows Release Qt brushGroupsCreateDirectlyAndRenameInline 与 brushGroupCollapseKeepsUnchangedDelegates 通过；验证两个自动名称、单笔刷归组、仅三角折叠、真实双击/Enter 保存/Esc 放弃，以及三轮折叠保持其他预览。折叠后输入测试等待真实 frameSwapped，避免在布局尚未呈现时使用旧位置。
- Release 已重建到 build/qt/bin/Release。多选拖动另列下一项。

## 普通 6，第二步：多选拖放

- 多选拖动复用现有 MIME，一次组织事务移动全部最高层选择，按树顺序保留排列；选中组与其后代不重复移动，自身/子组环与不存在目标均原子拒绝。
- Windows Release Qt multiBrushMoveIsAtomicOrderedAndRejectsCycles、multiBrushDropMovesEntireSelectionIntoGroup、brushGroupsCreateDirectlyAndRenameInline 通过。覆盖一次变更通知、同父重排、循环拒绝后字节保持、保存重开及窗口实际 MIME drop 将两个笔刷一起放入组。
- Release 已重建到 build/qt/bin/Release；真实跨窗口鼠标拖动未自动化。

## 整数参数与分组共享 UI 补验

- 完整 qt_ui_tests 源重新编译后，existingSecondaryPickerPreviewPixelsFreshStoreAndReopen、dualRatioControlsPreviewExchangeAndReopen、taperControlsCaptureWhitePreviewPixelsVbrAndReopen 两模式均通过，包含实际像素、预览、VBR 与重开。
- 三项旧分组用例改为等待实际 delegate 创建完成，brushGroupsAppearInLibraryTreeAndSearchKeepsParent、brushLibraryShiftControlSelectionAndRootReorder、expandedGroupLowerDropInsertsFirstForBrushesAndLayers 全部通过；覆盖搜索保留父组、Ctrl/Shift 选择、根目录重排及笔刷/图层展开组下半部投放。
