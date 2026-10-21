# 原始清单与 UI 回归统一修复验收

2026-10-21。[范围及当前状态](../../planning/2026-10-21-remaining-fixes-design.md)。

## 建立清单

逐项对照用户《新建 文本文档 (4).txt》、实现设计及验收，区分已完成、部分优化、只完成算子和未归因回归。U1–U4 的原失败见 welcome-workspace-validation，U2 独立复核仍失败，见 collapsed-panel-flyout-merge-validation。未把这些失败归类为既有问题。

本提交仅建立追踪入口，无运行时修改；未宣称上述待办已完成。后续在这里追加每项根因、改动、验证及限制。

## U3：快捷键列表旧断言

根因是测试仍要求早期26条，当前35条包含图层复制/粘贴、复制、合并、成组、删除、全选/取消/反选、移动工具九项。核对Main快捷键后，测试验证九项名称与键位、所有行三列非空及名称唯一、模型与实际行数一致，保留渲染像素和打开/关闭验证；没有删除产品快捷键迁就旧断言。

软件和D3D11下 `editMenuOpensKeyboardShortcutList` 各通过（含初始化清理3 passed），零QML警告。日志 artifacts/remaining-u3-{software,d3d11}.txt。Release/全部Qt及四ABI目标构建通过，生成头一致；无生产代码或ABI变化。其余待办保持。

## U2：异步图层面板创建

PanelContent异步创建LayersPanel期间，自定义属性尚未全部就绪；LayerLock内部Connections提前连接selected/kind时收到不存在信号警告，Instantiator的菜单项提前读取selected.mask。锁图标改为控件自身的属性变更处理，图标尚未创建时不请求绘画；菜单在selected尚未就绪时禁用。没有屏蔽Connections警告或改成同步加载。

`panelTabsRetractWithoutExpandingRailsAndKeepSharedPresentation` 在软件和D3D11通过，原7条QML警告消失，保留所有折叠、跨窗口、源栏状态断言；日志 artifacts/remaining-ui-probe.txt、remaining-u2-d3d11.txt。Release与Qt目标构建通过，ABI/state不变。退出仍有4个异步创建对象诊断，由U5继续跟踪；没有把该诊断称为已解决。

## U4 窄笔刷设置面板（2026-10-21）

方向盘固定横排的最小宽度使左列设置超宽，大小输入框点击落入画布。BrushTipShapeControl 在宽度小于220px时改为纵排；ScrollView 最小/隐式宽度为0，由外部面板分配，消除独立创建时的内容宽度循环。标准预览固定大小/不透明度，旧测试改验此契约；不同预设各自保留参数，切换默认压力圆笔验12px半径，再切回细线笔验20px半径。

原 `brushLibrarySettingsAndEnginePreviewsShareSelectionAcrossWindows` 在软件与D3D11各通过一次，保留真实40px输入、跨窗选中同步、标准预览/间距变化、实际透明像素、单历史及保存重开验证。日志：`artifacts/pixel-slider-ui-software.txt` / `artifacts/pixel-slider-ui-d3d11.txt`。方向盘两项软件通过；D3D11随S1最终共享回归记录。完整Release构建通过。

同批附加的旧液化事务用例软件通过、D3D11在3953行等待CPU frameRevision失败；未删除断言或标记通过，GPU常驻显示路径不保证逐笔CPU图像，该用例仍需专门分析。U1/U5及算法剩余项目保持待办。

## O1 新描边可操作链路

核心按实际主体印章复刻四通道max差集；Qt删除旧算法选择及独立建路径调度，接入允许溢出、实际笔触与预览、state45/VBR37及旧设置迁移。核心新5/旧11/内部3、IO7、原生1、Qt workflow软件/D3D11各8行、UiTests各3项、四ABI、严格Clippy/格式及Release通过。详见[描边验收](../brush/core/brush-stroke-replay-outline-validation.md)。原生旧Path/Sdf仍为兼容/参考执行保留，不将O1的“删除旧算法代码”标为完成；T1/T2/M1/H1/P1及U1/U5仍待。

## S1 大范围像素滑条

共用有符号对数坐标与指数反函数，跨度≥512px时启用，保留1px加减与精确输入。软件/D3D11各7项通过，含真实输入/鼠标/键盘及笔刷、液化模型双向同步；Release构建通过。详见[设计](../../interface/pixel-slider-exponential-design.md)与[验收](../interface/pixel-slider-exponential-validation.md)。U4共享方向盘两项D3D11也通过。U5透明交换链提示在3行真实拖放仍存在，默认alpha缓冲配置并未消除提示，因此撤回该无效候选，不称其修复。

## O1 旧原生执行清理完成

删除Path/SDF实际执行、缓存和核心选择接口；旧格式只在IO严格解析，绑定时产出Replay，规范目录写回state45。核心19、纯算子7、IO新旧迁移7/共享24、原生三模型1、软件/D3D11工作流各6行及UI各3、四ABI、严格Clippy/格式/生成头与Release均通过。旧主程序正在运行而链接失败，保留用户进程，新产物为 `build/qt/bin/Release/drawverse-next.exe`。详见描边验收末节；O1标完成，其余T1/T2/M1/H1/P1、U1/U5仍待。

## T1 普通根层局部选区常驻预览

一次捕获原始选中像素/余像素，原位保持精确原图；持续变换只改CPU几何并由Qt GPU显示，确认按原始完整源生成结果。支持像素半透明、羽化及图层偏移；图层属性opacity/fill非1、自身蒙版、组内/多目标仍归T2。核心12/公开7/原生2、软件/D3D11各10行及硬件Warp独立复跑3次、四ABI/Clippy/格式/生成头/Release通过。

4096×2160约256px套索六次Warp核心中位334.561→1.556ms，最终摘要相同；确认从0.319→61.887ms，因为取样转移到确认阶段。该计时不含真实显示/首次资产，不能写成交互时延。截图夹具的ready生命周期和历史深度作用域两次失败均已定位改正，未删产品行为断言。详细证据和限制见[变换验收末节](../tools/transform-resident-preview-validation.md)。新Release程序仍为drawverse-next.exe。T2、M1/H1/P1、U1/U5与实体持续性能继续待办。

## U6 文件成功文字残留

移除通用“文件操作完成”，文件成功清空旧通知，失败与保存关闭保护保持。白底/透明真实新建直接关闭、实际保存ORA再关闭、打开不存在文件报错，软件/D3D11各2行通过；Release构建通过。D3D11退出仍有1个异步对象诊断，继续归U5。见[欢迎页验收末节](../interface/welcome-workspace-validation.md)。

## T2 第一项：组内完整普通目标

移动和变换共用可消去祖先分割，变换覆盖不可变原层时保留显示parent；两级Normal/PassThrough祖先、链接蒙版、半透明目标及静态兄弟可常驻，真实结构不改。核心变换13/公开7/共享移动3/液化3、原生3、Qt复合变换软件/D3D11各7行及共享移动7/4K选区2通过；硬件焦点混合批次未能激活窗口，独立同例通过，原失败保留。四ABI、严格Clippy/格式/生成头/Release通过。

2000px带灰度链接蒙版、两级Normal组的六次Warp核心中位18335.863→5.087ms，确认355.272→3231.376ms，最终摘要相同；不含首次资产或UI显示。标准 `build/qt/bin/Release/drawverse.exe` 已可写且更新。详见[变换验收末节](../tools/transform-resident-preview-validation.md)。固定/祖先蒙版、非连续等T2剩余项及M1/H1/P1、U1/U5保持待办。


## U5：隐藏窗口孵化和浮窗透明 surface

PanelContent 检查真实窗口可见与客户端关闭状态；卸载仍在 Loading 的内容时先完成已启动的 deferred 控件，再卸载。LayersPanel 自身完成后才填充子委托，浮动列展开/折叠保持透明 surface。软件及D3D11的欢迎生命周期、白底/透明保存关闭、原生拖动合并三来源共各6行通过；显式检查关闭后的Loader不激活，并将残留创建/cancel/alpha诊断作为测试失败，零上述诊断。完整Release通过，标准drawverse.exe已更新。见欢迎页验收及 artifacts/batch-workflow-{software,d3d11}.txt；U1与原始性能项目继续推进。
