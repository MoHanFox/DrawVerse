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

## 普通 17：涂抹独立采样

- 主笔采用按笔尖尺寸/短轴决定的 0.25–2px 有界密集步长，第二笔尖配对采样，均忽略预设间距；仍使用 CPU 位移取样与原分页算子。普通画笔、橡皮间距不变。
- Rust 33 项通过：涂抹 4、步长边界 1、双笔刷 11、独立路径 7、湿混涂抹 6、双色工具策略 3、输入分页 1。新回归比较三种主间距与三种副间距，像素完全相同，程序/图片笔尖仍有不同形状，撤销重做、取样、锁和取消保持。
- paint-core / paint-ffi 严格 Clippy、格式及生成头检查通过；完整 drawverse Release 构建通过并输出到 build/qt/bin/Release。未做连续实体输入/大尺寸性能验收，不将更密集采样宣称为性能提升或 GPU 加速。

## 普通 12，第一步：建路径与独立纹理入口

- 既有算法显示为“建路径”。主纹理为空且整笔描边已开启时，仍可进入纹理选择、展开资源库并直接导入描边纹理；等待选择时只改变界面，不发布缺资源的启用配置。
- 后台复用 PNG 资源导入，role=3 应用到开始导入时的目标预设，保留主笔纹理和既有描边纹理参数。切换笔刷后导入完成不会污染新笔刷。
- Windows Release qt_workflow_tests 全部 13 项（含初始化/清理）通过，新用例 outlineTextureCanBeChosenWithoutMainTextureAndImportKeepsTarget 覆盖待选入口、直接导入、目标切换、主纹理不变、资源重新选择及状态重开。Release 已重建到 build/qt/bin/Release。
- SDF 算法与自身描边独立纹理扩展未在本项完成，未新增 GPU 算子。

## 性能 3，第二步：重复魔棒不增加步骤

- 同一连续 Add/max 或 Intersect/min 段的相同形状复用原选区身份；完整比较几何及灰度 alpha，保留跨操作次序和重复减法语义。
- 核心幂等单元 1 与 wand 集成 3 通过；Add/Intersect 各重复 100 次，步骤数、修订、历史及像素不变，灰度减法仍逐次累积。
- Windows Release Qt repeatedWandClicksDoNotGrowSelectionOrPublishNewGeometry 通过；真实客户端/原生队列重复点击 16 次后仍两步、零 selectionChanged，无多余历史与错误。
- paint-core / paint-ffi 严格 Clippy、格式与生成头检查通过，Release 已重建到 build/qt/bin/Release。独立复杂蚂蚁线绘制与 AA 专项仍待后续。

## 涂抹与描边共享 UI 补验

- 完整 Qt UI 目标重新编译、链接后，wetSmudgeUsesOwnToolStrengthAndFingerPolicyWithUndo、smudgeSharedBrushParametersInputAndDelayedHelp、strokeOutlineQtPreviewPixelsExchangeAndReopen、strokeOutlineResourcesExchangeToFreshStore 均通过；覆盖真实涂抹/撤销和描边预览、像素、资源交换及重开。

## 性能 4–5：GPU 常驻蚂蚁线

- 4096×2160 交织选区在真实 D3D11 的 12 次已呈现导航中，静态遮罩只栅格化一次；动画不再周期性重描整个路径。软件/超大文档回退与几何回归通过，详情见[专项验收](../tools/selection-resident-ants-validation.md)。
- Release 已重建到 build/qt/bin/Release。此项解决有界 GPU 画布的复杂选区显示，不宣称初始几何、全部 AA 场景或软件性能已经解决。

## 普通 15：完整图像翻转

- 图像菜单已接水平/垂直翻转，复用 transform 源取样与 LayerEdit 历史；整数镜像不做插值。所有图层/蒙版及选择一起翻转，隐式白底保留、画布外像素可恢复、置入图片源不变；含隐藏/锁定层，单次撤销。
- Rust 核心新 3 项通过：129×77 奇数尺寸、实心/半透明与透明白底覆盖、实际图层/组蒙版、负坐标隐藏层、灰度/羽化选择、双翻转与 undo/redo；源 Arc 保持、资源失败后修订/像素/选择/历史不变；与自由变换 nearest 镜像逐像素一致。
- 原生 full_image_flip_abi_pixels_single_history_and_descriptor_validation 通过：33×19 半透明实际像素、两轴、单历史/撤销重做、空指针描述符契约和未知 kind 拒绝。
- Windows Release Qt fullImageFlipClientRefreshesPixelsAndUndo 通过，实际异步客户端翻转/刷新与撤销；变换会话中拒绝翻转。共享 qt_transform_tests 全部 7 项通过（含初始化/清理），包括实际窗口强折叠不透明/半透明预览与完成零差异。
- ABI 1.66，DTO 布局不变；四个 C/C++ 静态/动态消费者、严格 Clippy、格式与生成头检查通过。Release 输出 build/qt/bin/Release。
- 仍受坐标、逻辑瓦片与暂存预算限制，失败保留文档；没有新增 GPU 算子，也尚未接长任务分步进度。未跑全量/跨平台/实体设备。

## 普通 16，第一步：多选图层移动

- 图层工具普通拖动提交当前多选，组/子节点重叠归并，绝对位移预览与取消覆盖全部目标；松手最终坐标一次生成历史。Ctrl 拾取仍为单目标。
- 核心 layer_move 5 项通过，覆盖两个独立根、组与蒙版重叠、锁拒绝原子性、取消、最终坐标、单历史与瓦片 Arc 保持。原生数组复制/参数校验 1 项与既有常驻移动屏障回归 1 项通过。
- Windows Release Qt multipleLayersMoveTogetherAndCancelRestoresAll 通过：两实际绘画图层的多选、实时位移、最终坐标、undo、取消与选择保持。四 ABI 消费者、严格 Clippy、格式与生成头检查通过，ABI1.67，Release 输出 build/qt/bin/Release。
- 多目标目前使用 CPU 像素回退，单目标常驻 GPU 保持；本项不宣称多目标或大画布首次移动性能达标，未跑全量/跨平台/实体连续输入。

## 普通 19：蒙版链接按钮

- 链条现在读取、修改核心真实链接状态；解除链接后所有者/组的移动与变换跳过蒙版，显式选中蒙版仍可单独移动。组蒙版同样显示可操作链条，支持撤销/重做及保存重开。修正蒙版变换中重复叠加所有者绝对位移的问题。
- 核心 layer_move 7 项、磁盘/内存链接历史 1 项、共享自由变换 7 项与组/图层/蒙版 21 项通过；包含实际半透明蒙版与非零独立坐标、单历史、锁拒绝、取消、复制粘贴、原始瓦片身份及逐像素变换对照。
- ORA 蒙版 4 项通过，覆盖原版本4/5、新版本6/7、普通/置入源、链接开关、像素保持与非法标志拒绝。链接默认开启，旧链接文档仍用原版本；解除链接才升级扩展，避免旧读入静默丢失状态。
- 原生新链接 1 项与旧蒙版 1 项通过；查询准确 publication、失败输出、非法描述符、同值无历史、实际移动、undo/redo 与 modified 发布均验证。
- Windows Release Qt maskLinkButtonMovesIndependentlyAndPersistsWithUndo 通过：真实鼠标点击、独立移动、撤销重做、保存/重开、重新链接和组蒙版；重开按蒙版/所有者关系重新定位（ORA 导入会重分配节点 ID）。多选移动共享 Qt 回归通过。
- ABI1.68，已有 DTO 布局不变；四 C/C++ 消费者、严格 Clippy、格式、生成头与 Release 通过，产物 build/qt/bin/Release。未跑全量/跨平台/实体连续输入。
