# 用户反馈分步执行记录

2026-10-18，Windows。对应[执行设计](../../planning/brush-feedback-execution-design.md)。

## 步骤 0：分析与排序

- 读取用户反馈、仓库约束、当前契约和源码职责，工作区初始无修改。
- 已确认：基础参数更新无条件清除预览；画笔列表在预览更新时重新查询整个预设数组；设置面板一次实例化所有分类；相对双笔刷为像素偏移语义。
- 文档最小验证：相对链接存在、`git diff --check`；不代表功能修复已完成。

后续步骤在实际测试完成后追加结果，未执行项保留为待办。

## 步骤 1：标准预览缓存与逐行通知

- 标准预览固定半径24、流量1、不透明度1，真实绘画仍使用用户参数；调整这三个基础参数不清空缓存或丢弃在途预览，原生浮点往返保持同一预览身份。
- 预览完成使用单行通知，预设名称/预览查询改为目录缓存，避免每张图片完成时通知全库、每行重复从Qt构造全库数组。
- 构建与真实QML回归暴露既有空剪贴板解引用：`canPasteLayers()`未判断`mimeData()`为空。通过Release链接映射定位并补空值保护，真实窗口回归不再崩溃；不是笔刷引擎或GPU崩溃。
- Windows Release `qt_ui_tests` 构建及生成头检查通过；`standardBrushPreviewKeepsBaseEditsAndUpdatesOnlyItsRow`、`combinedBrushFeaturesPreviewSurvivesStateRoundtrip`、`allAutomaticBrushPreviewsStayWhiteWithExactAlphaAndColorIndependentCache`、`brushGroupsAppearInLibraryTreeAndSearchKeepsParent`四项通过（另含初始化/清理）。记录：`artifacts/feedback-step1-tests.txt`。
- 未完成：大ABR实际导入耗时和滑动帧耗时量化；本步消除已定位的目录刷新放大，不宣称所有导入卡顿已解决。未跑全量/硬件/跨平台/GPU。

## 步骤 2：设置分类按需创建与控件整理

- 12个分类改为仅实例化当前分类，切换时销毁隐藏分类的曲线/控件；编辑值仍保存在BrushLibrary，导入解释选项移到面板状态，文件对话框不依赖已销毁控件。
- 为原本仅靠提示说明的选择框增加短标题；来源选择采用上下布局；光标图片不透明度改用通用CompactSlider；相对双笔刷隐藏独立直径，标称直径显示保留一位小数。
- Windows Release构建通过；新增实际分类实例化/切换/值保留/控件显隐用例，以及双尺寸、起收笔、动态来源三项既有Qt真实预览/交换用例通过。记录：`artifacts/feedback-step2-tests.txt`。
- 本步保持旧相对像素偏移语义；百分比与新持久化语义仍待步骤7。未量化所有硬件上的设置打开时延，未跑全量/跨平台/设备/GPU。

## 步骤 3a：曲线拖动抓取

- 压力与设备曲线编辑器阻止父滚动视图抢走鼠标；内部节点快速越过邻点时钳制在合法区间，移出图表再移回仍连续更新，松开后停止更新。
- Windows Release构建通过；`curvePointKeepsGrabOutsideGraph`的pressure/speed两组真实鼠标按下、越界、返回、释放用例通过。记录：`artifacts/feedback-step3-tests.txt`。
- Ctrl点击单节点切换插值仍待3b：当前核心仅支持整条曲线线性/PCHIP，不能只改界面标记，也不能把整曲线开关冒充节点开关。

## 步骤 4：画布边缘与目录交互

- 鼠标/笔在画布控件内部、文档外侧可开始普通绘画，核心继续按真实坐标裁剪；选区、移动、填充、吸色和液化保留原工具起点约束。
- 画笔拖动期间禁用源行DropArea；图层拖动屏蔽全部拖动项，同时客户端拒绝包含目标自身的层拖放。
- 双击画笔弹出改名框，按捕获的预设ID应用，复用已有名称验证，不改变预设资源身份。
- Windows Release构建通过；`brushCanStartOutsideDocumentAndPaintEdge`鼠标/合成笔两组从文档外开始、进入文档、单历史与撤销，以及`libraryDoubleClickRenameAndSelfDropFiltering`、既有展开组落点用例通过。记录：`artifacts/feedback-step4-tests.txt`。
- 画布用例使用实际Main.qml实例及其生命周期；不代表实体数位板或所有鼠标跨窗口拖放已验收。未跑全量/跨平台/GPU。
