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

## 步骤 5：光标细线与有界轮廓

- [设计](../../brush/ui/brush-cursor-bounded-outline-design.md)：默认线宽0，表示单条1界面像素细线；简化模式始终单线，使用外轮廓颜色的HSB亮度生成灰色，B滑条复用现有颜色偏好持久化。
- 显示轮廓最多2048段。复杂/大资源保守降采样并校正到原长宽比；核心原精确轮廓API、源覆盖、实际绘画像素和PNG图片光标保持。
- Rust `paint-core --test brush_outline`5项、`paint-io --test brush_resource`9项通过；新测试包含高频512方图、4096×1极薄图、闭合与源数据保持。两crate的lib严格Clippy、格式及Windows Release构建通过。
- Qt新增零宽/简化亮度/保存与非法值测试，既有全局偏好、图片偏好、真实图片光标轮廓与屏幕宽度三项通过。记录：`artifacts/feedback-step5-tests.txt`。
- 不保证保留低于简化采样分辨率的孔洞和纹理细节；这些细节仍保存在实际笔尖。未跑全量/实体设备/跨平台/GPU。

## 步骤 6a：2000px 主副笔尖范围

- [设计](../../brush/core/brush-size-limit-design.md)：主/第二笔尖、工具栏/设置/快捷键、ABR执行值、光标姿态与湿混载色入口统一直径1–2000px；共享Rust半径常量1000。载色网格仍最多64边，输入/印章预算不扩大。
- 核心真实边缘像素/单历史/撤销重做及光标边界4项、双尺寸4项、湿混载色/双载色/配置/工具22项通过。IO state6/VBR5/ABR35/双笔刷交换4项通过；历史schema保留原文件，测试明确旧范围仍接受并单独验证当前范围。
- 公开湿混API3项、三crate lib严格Clippy/格式、Windows Release/生成头检查通过。Qt最大尺寸编辑/工具栏范围/实际原生绘画/姿态/保存重开、既有双尺寸往返和快捷键三项通过。记录：`artifacts/feedback-step6a-tests.txt`。
- VBR仍为1.0，无ABI布局变化；新导入ABR允许1003px等原尺寸，超2000px才钳制报告，既有来源基线不重写。扁平图片间距仍待6b；本步不声明2000px所有组合的性能或硬件/跨平台/GPU验收。

## 步骤 6b：扁平图片笔尖间距

- [设计](../../brush/core/brush-image-spacing-design.md)：不可变资源构造时缓存非零覆盖包围盒，主/第二图片笔尖按有效短边与静态圆度修正排布步长；文档起笔/续段/收尾与分页统一使用同一编译结果。程序笔尖和覆盖占满正方形的默认图保持原步长。
- 核心图片6项（新增矩形/透明边方图×主副笔尖×0/90度×分段/分页像素与历史）、旧双笔刷11项、实际Document分页13项通过。IO图片VBR13/双笔刷IO3项及lib严格Clippy/格式通过。
- Windows Release/生成头检查通过；Qt图片导入预览捕获与新目录VBR原图移除后实际执行两项通过，记录：`artifacts/feedback-step6b-tests.txt`。
- 参数/资源哈希/state/VBR/ABI不变。薄图更密且可能耗时增加，仍受原分页和总预算限制；未宣称任意有孔图片完全连续、实体设备/跨平台/GPU通过。

## 步骤 7a：双笔刷比例模型与Qt入口

- [设计](../../brush/dual/brush-dual-ratio-design.md)：新增relative-ratio-v1/diameterRatio，state38/VBR1.0/behavior30/dual-size-ratio；旧relative-size-v1像素偏移保留，界面明确列为旧版模式。新启用无尺寸模型的双笔刷按当前主副名义尺寸初始化比例，比例/独立切换保留参数。
- 主副姿态、名义尺寸、压感耦合、预览临时状态和新目录交换接通。修复通用缺项补全向比例模型混入旧diameterOffset的问题，按模型补全；state仍严格拒绝降级、混合字段和非法值。
- 核心双尺寸5项（含边界比例/压感/配对与路径对照实际像素）、IO尺寸交换4/边界1项、原生准确准备/捕获/包写入口（旧偏移和新比例×开关）通过。三crate lib严格Clippy/格式及Windows Release/生成头检查通过。
- Qt新增比例控件/显隐/异步真实预览/实际绘画/新目录VBR像素一致/重开，以及旧偏移和分类按需创建两项通过，记录：`artifacts/feedback-step7-tests.txt`。
- ABR自动比例换算另列7b，尚未完成；不把新模型入口通过解释为所有ABR或实体设备验收。

## 步骤 7b：ABR双笔刷自动比例

- ABR转换器14/交接13按实际执行主副直径生成比例；完整原来源、导入基线与传递抖动保留，拒绝来源/交接降级。修正来源报告的第二笔尖直径和间距字段路径。
- Rust ABR转换35项、导入12项（含保存来源、拒绝降级、主尺寸变化带动第二尺寸）、IO lib严格Clippy和格式通过；Windows Release/生成头检查通过。
- Qt交接13/拒绝12/重置基线、旧计算双笔刷、实际许可ABR整包及新目录VBR源和像素交换、比例控件往返四项通过。记录：`artifacts/feedback-step7b-tests.txt`。实际包为仓库许可样本约901KB/148笔刷，不等同用户34MB/485笔刷大包；未跑全量/设备/跨平台/GPU。

## 步骤 8：起收笔相对长度与端部透明

- [设计](../../brush/wet/brush-taper-relative-length-design.md)：每端独立选择px或笔刷直径百分比，保留两组值并仅显示当前模式输入；相对长度在起笔捕获实际直径，单端上限仍4096px。端部透明复用不透明度minimum，关闭为1、开启为0，尺寸渐细独立保留。
- 新字段写state39/VBR1.0/behavior31/taper-relative-length，关闭值往返，旧像素对象不迁移。核心编译和尾部缓冲显式要求直径，避免遗漏相对换算。
- 核心起收笔11项、实际文档/湿混起收笔筛选13项、分页筛选18项通过（部分重叠）；IO起收笔8项含相对/独立×关闭/开启×三个临时直径的真实像素对照、新目录交换与非法/降级拒绝通过。三crate lib严格Clippy、格式、Windows Release/生成头检查通过。
- Qt起收笔真实控件/异步预览/绘画/单历史/VBR新目录/重开两组（独立与相对），旧模型关闭值/重置、双笔刷比例四项通过。记录：`artifacts/feedback-step8-tests.txt`。未跑全量/实体设备/跨平台/GPU。

## 步骤 9a：后台覆盖资源清单

- [设计](../../brush/library/brush-resource-picker-design.md)：覆盖库只读枚举普通PNG身份，包含未引用资源，排序并限制4096项；不解码/保留整库图片。ABI1.58/bit53后台资源JSON入口支持容量探测和错误清零，state/VBR不变。
- IO资源10项（新增排序/过滤/未引用与损坏文件/重开/零常驻/取消/上限），公开资源ABI4项（新增容量、哨兵、错误与修订保持）通过；修正既有图片光标测试将ABI次版本固定为53的过时断言，保持最低53能力要求。
- IO/FFI lib严格Clippy、格式、生成头及Windows Release四种C11/C++20静态/动态ABI消费者通过。Qt缩略图和选择器待9b，本步不宣称用户入口已完成；未跑全量/实体设备/跨平台/GPU。
