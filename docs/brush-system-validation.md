# 统一笔刷系统验收计划

2026-10-16湿混双色保存交换：[验收](brush-wet-dual-io-validation.md)。显式独立载色策略、完整关闭值、state19/VBR1.23/behavior16和能力标记已实现；真实两笔/完整双载色、主/第二/纹理图片与原来源新目录往返，6项新IO及三旧入口保护/16项旧回归、Schema和Windows Release/ABI/Qt通过。原生新入口及Qt组合仍待接；前述Qt偶发参数输入问题没有据此宣称修复。

2026-10-16湿混双色 Engine：[验收](brush-wet-dual-engine-validation.md)。完整载色捕获和历史成功后整对采用、共享租约/取消/失败/stop及无像素拾色通过，6项新Actor与11项旧原生回归、Release/ABI通过。Qt参数输入首轮偶发失败、单项复核通过，保留日志且根因未定位；Qt涂抹通过。保存/原生准备和Qt组合入口仍待接，不能据此宣布公开组合或整体完成。

2026-10-16湿混双色实际绘画：[验收](brush-wet-dual-document-validation.md)。明确Rust Document入口与完整WetInk结束、四场隔离/共用27模式/饱和混色、分页起收笔双载色和实际网格预算/失败回滚通过；10项新函数（含216组常量对照及8组回放参考）/39项旧回归，Windows Release/ABI/Qt通过。Engine完整归还、IO/ABI准备及Qt组合仍待接，当前用户入口仍拒绝，不宣称整个系统完成。

2026-10-16湿混双载色状态：[验收](brush-wet-pair-validation.md)。主/第二二维载体按角色配置/清洗载色、工具独占整对租约与原子采用已实现；5项新数值和21项既有回归、Windows Release/ABI/Qt通过。Document双色贡献、Engine完整返回、起收笔检查点及IO/Qt组合尚待接，仍不开放湿混双色，不将本阶段当作用户功能完成。

2026-10-16起收笔 Qt：[验收](brush-taper-ui-validation.md)。完整 state18/关闭值/重置基准、ABI1.51入口和真实白色预览、在途捕获、像素历史/VBR1.22及重开均已接通，设置页提供距离和独立尺寸/opacity平滑曲线；Windows最小五项Qt通过。起收笔不再属于缺少用户入口的范围；其余湿混双色/橡皮、非分隔双笔刷、完整ABR/16位及VBR分组等不据此宣称完成。

2026-10-16工具栏参数追加：画笔/橡皮的流量、不透明度与各自压感复用当前预设，涂抹下拉改细分隔线；Windows Release 和最小三项 Qt 通过，见[验收](brush-toolbar-transfer-validation.md)。

2026-10-16用户范围修订：删除试画功能，保留自动预览；所有笔刷自动预览固定白色，不随双色、颜色动态、湿混、当前颜色或工具混合模式变色。下文历史试画/有色预览项以此为准，不再作为未完成门槛；实际绘画像素颜色与湿混载色仍独立验证。见[设计](brush-white-preview-design.md)。

2026-10-15追加：双色重算由整瓦片改为确切变化像素，工作预算同步实际区域，1%密集输入与私有/共享身份回归及648组真实瓦片历史参考通过，见[双色性能](brush-dual-color-performance-validation.md)。固定CPU基准4.48–35.58倍，不代表Qt/设备总体手感。双色原生Qt入口和自动描边/复合光标仍待实施。

2026-10-15追加：双色配置state16/VBR1.14/behavior13、关闭配置/27模式、相对尺寸/平滑/PCHIP两笔真实像素及共享图片/原始来源交换已验证，见[双色IO](brush-dual-color-io-validation.md)。ABI1.47/Qt色彩入口暂拒，下一步重算范围优化及完整入口；不代表双色用户功能或整个系统完成。

2026-10-15追加：双色双笔刷separated-stroke-v1核心已有真实像素、独立/配对路径、溢出/顺序/半透明/一次整笔opacity、分页及失败原子回滚验收，见[双色核心](brush-dual-color-core-validation.md)。31项对应和旧覆盖回归通过；旧IO明确拒绝新非默认配置，格式/原生Qt/自动描边与复合光标尚未接入，不代表整体笔刷系统完成。

2026-10-15追加：第二笔尖相对尺寸的内核、state15/VBR1.13及ABI1.47原生Qt入口已接通，见[尺寸内核](brush-dual-size-validation.md)、[尺寸IO](brush-dual-size-io-validation.md)、[尺寸入口](brush-dual-size-ui-validation.md)。DPR1/2真实控件/预览/在途像素、关闭重开/基准重置、VBR与ABI验证通过；仍只执行灰度覆盖，双色叠放/溢出/自动描边未完成。该阶段不能替代收尖、湿混、完整ABR与整体设备/跨平台验收。

状态：2026-10-13，分阶段实施。程序笔尖验收见 [阶段结果](brush-tip-validation.md)；VBR 程序子集结果见 [阶段结果](vbr-procedural-validation.md)。以下为完整系统的验收计划，不能以局部结果代替全部通过。

设计：[brush-system-design.md](brush-system-design.md)；格式：[vbr-format-design.md](vbr-format-design.md)。各实施阶段在本文件追加真实结果、命令、环境和限制，不把计划改写成“已通过”。

双笔刷准备桥见 [brush-dual-bridge-validation.md](brush-dual-bridge-validation.md)：第二资源准确采用、在途保持、闲置释放/重建已补。IO见 [brush-dual-io-validation.md](brush-dual-io-validation.md)，路径见 [brush-dual-path-validation.md](brush-dual-path-validation.md)，覆盖核心见 [brush-dual-validation.md](brush-dual-validation.md)。原生Qt和资源导出已接，见 [brush-dual-ui-validation.md](brush-dual-ui-validation.md)；组合轮廓/PS核验与完整系统范围仍待后续。

纹理原生Qt结果见 [brush-texture-ui-validation.md](brush-texture-ui-validation.md)：ABI1.42/纹理分类、捕获目标资源导入、真实像素历史、state11/VBR1.9与来源并集已完成。准备生命周期见 [brush-texture-bridge-validation.md](brush-texture-bridge-validation.md)。双笔刷、平滑、湿混、完整ABR映射、VBR扩展和总体性能验收仍待后续，不代表整体完成。

## 1. 定义、迁移与资源

- Rust：定义范围、非法浮点、资源缺失、曲线非法点、能力/profile/版本拒绝；快照在预设切换/资源删除后仍供已开始的笔触使用；预算与缓存失效。
- 旧预设迁移：活动预设、每支笔刷独立大小、间距与不透明度保持（最新用户要求覆盖旧共享尺寸断言）；坏数据不覆盖原库；旧圆笔覆盖与历史语义保持；普通绘画/擦除按最新整笔透明度上限验收。
- FFI：新导出的参数复制、句柄归属/生命周期、销毁时的在途任务、大小/reserved 校验、错误码、旧 DTO 及旧导出兼容；cbindgen 头严格字节检查。
- IO：VBR 往返定义、共享资源、顺序和版本；规范化保存幂等；失败/取消无半成品；未知字段有界保留。
- 非法包：路径穿越/别名/大小写冲突、重复条目、符号链接、错误哈希、解压/图片/JSON/计数上限、截断及缺失资源；拒绝前不进行超预算分配。

## 2. 真实笔触与手感

建立固定轨迹集：慢速圆弧、快速折线、压力渐变、压力尖峰、停顿、轻压起笔、抬笔收尖、倾斜侧锋、旋转跨角度边界。

- 关闭时间动态时，同一路径的不同事件分段应产生相同印章；启用时间模式后按时间积分验证，不以设备事件频率改变流量。
- 压力/位置平滑、收尖、曲线、散布及固定随机种子分别验证；鼠标缺失通道有明确回退。
- 绘画、擦除、涂抹同一笔尖覆盖；图片笔尖孔洞、纹理相位、双笔尖、流量与整笔透明度以真实像素断言验证。
- 选区 AA/羽化、偏移图层、蒙版目标、透明锁/完全锁/祖先锁、剪贴和组穿透不被新执行路径破坏。
- 每笔只有一条历史；取消、资源/磁盘失败、退出中止恢复真实像素；撤销/重做不依赖预设当前状态或重新随机采样。

原生硬件验收记录设备、驱动、报告能力、显示刷新率、DPR、平滑设置及引擎参数。模拟轨迹不能代替真实数位笔手感验收；没有相应设备的平台标为未验证。

## 3. 湿混与 ABR

- 湿混：载色消耗、清洗/重新载色、多色拾取、二维载色、透明底、边界、跨瓦片、合成取样只写目标层。验证每印章“先读后写”、后续印章可读取之前结果。
- 工具：涂抹保持独立 R 工具和既有参数；选预设不切换工具；同笔内参数修改不改变已捕获定义。
- ABR：按实际文件版本和能力建立合法样本库，涵盖程序/图片笔尖、动态、纹理、双笔刷及混合的支持边界；记录样本来源与可使用授权。
- 每个 ABR 样本检查真实笔尖资源、转换后的参数及引擎笔触；不以缩略图存在或解析成功作为绘画兼容通过。
- 不支持与近似项报告完整，保存 VBR 后资源独立；不支持的启用能力不能静默替换成普通圆笔。

## 4. 设置界面与预览

原生 Qt 覆盖停靠、普通浮窗、图标展开及至少 DPR 1/2：

- 左侧分类、独立功能开关、右侧参数与曲线、固定底部预览；关闭模块保留编辑值，不影响绘画。
- 笔刷库、设置页、工具详情栏及多个面板实例同步同一状态；切换笔刷不串参数或改变工具。
- 真实点击搜索/数值/曲线，工具/快捷键恢复；原生窗口切换后仍可输入，不能用强制焦点调用掩盖问题。
- 悬停提示延迟 1000ms；右侧滚动、窄宽度替代布局、分类/滚动/尺寸持久化。
- 后台真实预览覆盖线条、模拟压力、双色/颜色动态/湿混，所有输出固定白色并保留alpha；过期 token 不覆盖新结果；主文档 revision、dirty、历史不变。
- 产品界面仅出现已实施参数；没有“可点击但不执行”的功能入口。

## 5. 笔刷描边光标

- 圆、椭圆、旋转图片笔尖及带孔洞/多个连通区域的轮廓；鼠标标称尺寸、数位笔已知倾斜/旋转和落笔压力。
- 黑白双线/自定义线宽、中心十字、图片透明预览、仅悬停/绘画显示开关及全局设置恢复；VBR 不包含这些偏好。
- 缩放、平移、DPR、跨屏、浮动画布以及选预设后的轮廓同步；描边屏幕宽度不随文档缩放改变。
- 缓存命中时移动不重复提取轮廓；资源变更失效；缓存与简化误差有界。
- Alt、Ctrl、空格、Caps Lock、液化/自由变换接管；移出/失活隐藏；预览不进入任何导出/图层/缩略图或历史。

## 6. 性能与后端

普通圆笔、1% 间距线稿笔、巨大笔尖、复杂纹理、散布/双笔刷、湿混分别测量。记录输入到可见帧延迟、GUI tick p50/p95/p99、印章吞吐、瓦片驻留与历史暂存、轮廓/资源缓存峰值、长笔触稳定性。不能把 GUI tick 当成完整笔触延迟。

实施开始前保存同机同图同轨迹基线；不同配置结果不能直接比较。普通圆笔快路径要求无明显回退，具体量化门槛与高级笔刷预算在取得基线后固定并写入阶段记录。性能验证不能通过降低质量、静默增加间距、丢失输入或关闭选区通过。

后续 GPU 以相同 DabCommand、资源和取样顺序对比 CPU；记录容差，验证湿混顺序、取消、失败回退、读回提交及跨后端撤销的精确像素恢复。后端切换不得中途改变已开始笔触的语义。

## 7. 阶段结果记录模板

每阶段写明：完成能力/未完成能力、行为版本和 ABI 变化、相关 Rust/IO/FFI/Qt 测试命令与结果、原生截图/真实笔触与性能证据、跨平台/硬件限制、迁移及回退情况。沿用 `python tools/check.py` 作为仓库统一入口；用户要求最小测试时运行实际相关测试，并明确未运行全量，不能宣称完整通过。尚未存在的 Python 层不添加虚假 pytest。

本轮仅完成设计文档与上下文路由，功能实施尚未开始。

散布原生Qt阶段：ABI1.39/state8/VBR1.6/behavior5，见 [设计](brush-scatter-ui-design.md) / [验收](brush-scatter-ui-validation.md)。分类参数、准确捕获、图片/来源交换、DPR1/2与ABI已验证。实体数位板、跨平台、其他动态与湿混仍未验收；不是完整系统通过。

形状抖动内核阶段：见 [设计](brush-shape-jitter-design.md) / [验收](brush-shape-jitter-validation.md)。独立随机域/大小角度圆度/图片和四类写入/确定性真实像素已验证；保存/VBR/Qt入口尚未接，旧入口拒绝新配置，完整系统继续后续。

形状抖动IO阶段：state9/VBR1.7/behavior6，见 [验收](brush-shape-jitter-io-validation.md)。完整种子、关闭配置、组合两笔真实像素和来源保持已验证；原生Qt待接，旧包入口暂拒绝新对象，完整系统继续后续。

形状抖动原生Qt阶段：ABI1.40/state9/VBR1.7，见 [验收](brush-shape-jitter-ui-validation.md)。子页/实际输入与预览/捕获笔触、完整种子和最高目录版本、图片来源交换、DPR1/2与ABI已验证；其他动态/纹理/双笔刷/湿混及完整ABR仍待后续。

颜色动态内核阶段：见 [设计](brush-color-dynamics-design.md) / [验收](brush-color-dynamics-validation.md)。两色/HSV和真实多色覆盖/压力上限/分页/历史已验证，状态/VBR/用户背景原生传入/Qt尚未接，完整系统继续后续。

颜色动态IO阶段：state10/VBR1.8/behavior7，见 [验收](brush-color-io-validation.md)。模型/完整seed/关闭频率、真实两笔与图片来源还原已验证；原生背景与Qt待接，旧package暂拒绝新字段；完整系统继续后续。

颜色动态原生Qt阶段：ABI1.41、state10/VBR1.8，见 [验收](brush-color-ui-validation.md)。真实两色捕获/预览/绘画与关闭可编辑、保存交换/图片来源/DPR1/2/ABI已验证；模型语义明确，PS真实颜色兼容、纹理/双笔刷/湿混及其他全系统范围继续后续。

纹理内核阶段：见 [设计](brush-texture-design.md) / [验收](brush-texture-validation.md)。文档纸纹/局部纹理、真实四类覆盖/周期边界/资源预算/历史已验证；资源IO/VBR/原生Qt尚未接、其余纹理组合/双笔刷/湿混与完整系统继续后续。

纹理IO阶段：state11/VBR1.9，见 [验收](brush-texture-io-validation.md)。资源共享/周期mip预算、真实恢复/完整来源及6145条目边界已验证；原生Qt待接，完整系统继续后续。

双笔刷原生Qt阶段：ABI1.43/state12/VBR1.10，见 [设计](brush-dual-ui-design.md) / [验收](brush-dual-ui-validation.md)。四子页/完整第二配置与捕获导入、实际尺寸比例预览、真实像素/历史和新目录资源交换已验证，库包临时拒绝解除；组合光标、平滑、湿混与ABR完整映射继续后续。

双色复合/自动描边扩展已获用户确认，设计见 [brush-dual-color-extension-design.md](brush-dual-color-extension-design.md)。第二颜色、相对尺寸/溢出/正常合成/前后顺序、整笔贡献与两种压感边宽均纳入后续验收；交叉和回笔不能出现后印章描边盖进主线，整笔透明度只应用一次。现未实现、未验证，等待用户再次发令。

时间平滑内核阶段：见 [验收](brush-smoothing-validation.md)，固定网格和实际共用写入/预算/取消/分页历史已验证，状态与原生Qt入口及起收笔待接；非默认旧保存明确拒绝。完整系统范围继续后续。

重置预设阶段：设置按钮及完整独立基准/真实持久化重开/资源恢复与捕获笔触已验证，见 [验收](brush-reset-validation.md)。双色新配置仍待实现，完整系统继续后续。

画笔颜色混合内核阶段：27种目标颜色写入/整笔限制/蒙版与透明锁/复杂组合及分页历史已验证，见 [验收](brush-blend-validation.md)。原生Qt工具状态/菜单/预览仍待接入，不代表普通画笔混合完整用户功能或双色系统已完成。

画笔混合原生入口阶段：ABI1.44、新模式捕获DTO/准确准备/两色复制与真实历史及旧入口兼容已验证，见 [验收](brush-blend-abi-validation.md)。Qt工具栏/持久状态和有色底预览待接，完整系统继续后续。

当前追加阶段：画笔混合Qt工具状态/菜单/真实有色底预览已接入，见[验收](brush-blend-ui-validation.md)。自动平滑压感曲线ABI1.45/state13/VBR1.11/behavior10及统一原生采样已接入，见[验收](brush-pressure-smooth-validation.md)；编辑后自动平滑，旧未编辑模型保留。重置预设移至右侧参数末尾/固定预览上方。历史阶段的“待接”描述保留当时状态，以此最新条目为准。双色扩展、湿混和完整笔刷系统继续后续。

最新用户修正及混合性能阶段：工具下拉跨原生面板置顶；笔刷预览固定正常，不随工具混合切换。主/第二饱和覆盖跳过与圆尖平方距离快路径已验证，见[验收](brush-mix-performance-validation.md)。之前“有色底跟随模式”的预览设计被本轮用户要求取代。时间平滑IO/Qt、双色扩展、湿混和全系统范围继续后续。

时间平滑IO最新阶段：state14/VBR1.12/behavior11保存完整开关/参数，双流压感及来源图片真实笔触往返已验证，见[验收](brush-smoothing-io-validation.md)。旧ABI1.45/Qt暂拒新配置，下一阶段接完整原生桥和设置；收尖、双色与湿混等完整目标保持。

时间平滑原生Qt最新阶段：ABI1.46完整设置、关闭配置/基准、真实时间预览/在途捕获及VBR/保存重开已接，见[验收](brush-smoothing-ui-validation.md)。前阶段准备/库包临时拒绝已解除；收尖、双色复合、湿混与高级ABR等全系统范围继续后续。

双色原生Qt入口最新阶段：ABI1.48、完整state16/VBR1.14、五个双笔刷子页、主/第二27贡献模式、实际两色预览与捕获/独立客户端像素/历史/关闭重开，见[验收](brush-dual-color-ui-validation.md)。重置仅保留来源与兼容分类末尾。自动描边、复合光标、收尖、非分隔绘制、湿混及高级ABR等目标继续后续。

自动描边预设阶段：双色页创建跟随压感/固定边宽独立用户预设，复用已完成模型，不新增ABI或VBR字段；完整基准与1024上限、真实两种压力边宽/交叉/opacity/捕获/独立VBR像素/历史，见[验收](brush-auto-outline-validation.md)。复合光标、非分隔绘制、收尖、湿混与高级ABR继续后续。

复合笔尖原生几何阶段：ABI1.49新增非阻塞双姿态读取，主/第二压力设备、图片与有效溢出标志和准确修订寿命保持，见[验收](brush-composite-pose-validation.md)。Qt当前仍走旧主姿态读取，双轮廓及第二图片轮廓待下一阶段；不视为完整复合光标完成。

可选基础复合光标Qt已接通，默认关闭、全局保存且不进VBR，第二动态/图片与在途捕获、DPR1/2及原单笔尖回归见[验收](brush-composite-cursor-ui-validation.md)。不含随机印章或整笔面积预演；非分隔、收尖、湿混和高级ABR等继续后续。

二维湿混数值载色组件已实现，7项Rust像素/数值回归及clippy通过，见[验收](brush-wet-carrier-validation.md)。支持真实二维拾色/混合/沉积、透明底、距离消耗和清洗/重载，尚未接文档事务/历史、预算/图层取样、预设交换或Qt；不是可用湿混工具或完整系统通过。

湿混真实Document阶段已接通Rust专用起笔/结束和载色返回，12项真实文档用例、3项故障/预算用例及相关旧画笔/ABI/Qt回归通过，见[验收](brush-wet-document-validation.md)。取样源、选区/蒙版/锁、分页账本和历史已执行；尚无湿混参数IO/ABI/Qt入口，颜色动态/双色组合仍暂拒，完整目标继续后续。

湿混颜色动态阶段：真实沉积后复用linear-hsv-v1，保持alpha与工具载体隔离；35项相关Rust用例、Debug/Release旧算法位级参考及Release/ABI/Qt回归通过，见[验收](brush-wet-color-validation.md)。解除上一阶段颜色动态临时拒绝，双色湿混及完整IO/ABI/Qt和自动清洗重载仍待完成。

湿混统一设置与载色策略阶段：Rust预设/定义/编译快照和配置起笔已接autoClean/autoLoad/allLayers及WetMix，真实两笔保色/重采样/捕获与回滚，47项相关Rust及Release/ABI/Qt回归通过，见[验收](brush-wet-settings-validation.md)。旧IO明确拒绝非默认湿混防丢值；完整交换、桥/Qt、工具持有与手动载色操作仍需继续。

湿混保存交换阶段：state17/VBR1.15/behavior14完整wetMix、关闭值与策略、真实跨笔载色/像素/历史和图片/ABR来源保持，29项相关Rust、Schema解释器及Release/ABI/Qt回归通过，见[验收](brush-wet-io-validation.md)。原生准备与包入口暂拒新配置防旧Qt丢值，完整执行桥/设置/预览/基准与工具所有权继续下一阶段。

湿混共享工具与原生会话阶段：WetTool/WetLease隔离预设、文档历史和工具载色，原生Engine配置起笔并仅成功结束后采用；23项相关Rust、Release/ABI/Qt普通绘画回归通过，见[验收](brush-wet-owner-validation.md)。当前无新C ABI，准备/包临时拒绝仍有效，外部共享工具句柄、资源桥与完整Qt湿混入口继续后续。

湿混原生ABI阶段：ABI1.50/WET_BRUSH共享工具create/destroy/状态/清洗/准备装载与会话绑定、新湿混state/库包入口，10项相关Rust、生成头/Release/C/C++及Qt普通笔刷回归通过，见[验收](brush-wet-abi-validation.md)。旧入口保留Unsupported保护；下一阶段完整Qt参数、基准/关闭值保存、跨文档工具寿命及实际有色预览，用户湿混界面尚未完成。

湿混Qt阶段：ABI1.50完整state17镜像/关闭值/基准/VBR，湿混分类/六比例/策略与手动动作、跨文档共享和独立真实有色预览已接；6项相关Qt功能回归、DPI2重复交互及Release/ABI通过，见[验收](brush-wet-qt-validation.md)。新版准备/包入口已使用，旧接口保护保留；湿混双色/Erase/Smudge、收尖、非分隔及高级ABR与整体验收继续后续，完整目标未完成。

二维湿混涂抹阶段：独立R工具复用二维载体、笔尖动态/纹理/灰度双路径与原生租约，工具强度压感/取样及手指策略、透明搬运/选区/蒙版/锁/历史已执行，见[验收](brush-wet-smudge-validation.md)。关闭湿混保留旧涂抹，无ABI/格式变化；湿混双色/橡皮、收尖、非分隔、高级ABR和整体验收仍待后续。

湿混涂抹高DPI复查：已记录原生鼠标事件与QtTest输入交错的证据，仅对既有悬停计时测试做局部输入隔离，产品代码不变；环境缩放倍率2的原生窗口2项功能用例连续3次通过，倍率1的3项相关用例通过。此前待查结果及追加证据见[湿混涂抹验收](brush-wet-smudge-validation.md#原生高-dpi-悬停复查)，不把配置倍率混称硬件DPR；完整笔刷系统剩余范围不变。

ABR整笔不透明度阶段：基础转换器v2执行非100% Opct并标明来源版本，保持v1不可变记录/基准/报告；VBR1.16新旧来源混合/最高版本与降级拒绝、真实密集往返像素/历史及Qt导入/重置/全新库交换已验证，见[验收](abr-opacity-validation.md)。未使用新行为保持原转换器/包需求，无ABI/state变化；完整ABR动态/16位、湿混双色/橡皮、收尖/非分隔等仍未完成。

ABR笔压动态阶段：转换器v3执行尺寸/流量/opacity笔压与独立最低比例，来源record1/交接JSON2/VBR1.17及旧来源字节保持、三代混合最高版本/降级拒绝、Qt真实低压力像素/历史/重置与新库交换和重开已验证，见[验收](abr-pressure-validation.md)。35项相关IO、12项FFI、4项Qt功能、Schema及Release/C/C++通过；无新C ABI/state，完整ABR动态/16位、湿混双色/橡皮、收尖/非分隔和整体验收仍待后续，目标未完成。

真实ABR语料/分节对齐阶段：修复真实Photoshop文件节间/末尾填充，加入CC0 Myer148与两个MIT自作者PS探针，全部150项原覆盖/来源身份保持、代表笔尖真实绘画/历史，公开FFI148整包与Qt用户入口/新库VBR像素一致通过，见[验收](abr-real-corpus-validation.md)。原外部诊断更新并运行新压感路径。许可真实8位语料已有常规回归；16位调查发现上游字节序分歧，仍明确未支持，不能推定16位或PS完整笔触等效已完成。其他完整目标范围继续后续。

## 当前平台验收顺序（2026-10-15用户确认）

当前主要针对Windows平台完成实现与阶段验收，相关最小测试通过后提交/推送并更新Windows开发版。macOS/Linux及跨平台一致性验收后置，不作为当前阶段提交门槛；其结果仍明确标为待验收，不凭Windows通过推定其他平台已通过。

ABR散布导入阶段：converter4/record1/交接JSON3/VBR1.18，单/双轴、数量及抖动、完整关闭配置、压力与图片组合真实两笔及历史，旧三代基准/报告和四代交换字节保持，Qt导入/重置/保存与新库逐像素回归通过，见[验收](abr-scatter-conversion-validation.md)。Windows相关IO42项、FFI5项、外部诊断/Schema、Qt3功能与Release/ABI通过；PS随机分布标近似，设备散布控制与完整高级ABR/16位、湿混双色/橡皮、收尖/非分隔等继续后续。跨平台验收按用户要求后置，不作为当前阶段门槛。

ABR颜色动态阶段：converter5/record1/交接JSON4/VBR1.19，前景背景/HSV与逐笔逐印章、完整关闭配置、五代来源字节/最高版本与降级拒绝，Qt真实两色导入/编辑重置/新库像素及重开已验证，见[验收](abr-color-conversion-validation.md)。Windows相关IO46项、FFI6项、Qt3功能与Release/ABI/Schema通过；纯度/设备颜色控制和完整PS视觉核对仍待后续。

ABR形状抖动阶段：converter6/record1/交接JSON5/VBR1.20，尺寸/角度/圆度随机及最低比例、关闭和缺省配置、压力散布颜色组合真实两笔、六代来源字节/最高版本/降级和Qt新库像素已验证，见[验收](abr-shape-conversion-validation.md)。Windows相关IO51项、FFI7项、Qt3功能、Release/ABI/Schema/外部基础诊断通过；随机/下限叠加标近似，设备形状控制和完整ABR/16位、湿混双色/橡皮、收尖/非分隔等继续后续。跨平台验收后置，完整目标未完成。

2026-10-16 Windows输入修复：针对用户Windows Ink入点后悬停轮廓回跳/双光标，以及鼠标和笔点击模式菜单却落笔，补充笔输入所有权、Quick帧同步Hover隔离、原生光标先捕获后切换、菜单窗口坐标路由/透明模态屏障及不激活的兼容宿主。Release/生成头与环境倍率1/2相关Qt验收通过（13/9 passed），见[验收](windows-input-menu-validation.md)。实体驱动与实际Qt6.5未认证，跨平台后置，无ABI或格式变化；完整笔刷系统其余范围继续后续。

ABR设备形状阶段：converter7/record1/交接JSON6/VBR1.21，方向/笔旋转/倾斜角度与圆度映射既有TipDynamics，独立压力/随机/散布/颜色组合、旧六代来源字节和七代最高版本/降级、Qt编辑重置/新库真实设备像素及重开已验证，见[验收](abr-device-conversion-validation.md)。Windows相关IO56项、FFI8项、Qt3功能与Release/ABI/Schema通过；PS设备公式与下限换算标近似，关闭设备只保留原来源，完整ABR/16位和其他高级动态、湿混双色/橡皮、收尖/非分隔、VBR分组及整体验收继续后续。完整目标未完成，跨平台后置。

2026-10-16用户平滑收尾修订：保留拖绳距离，抬笔停在当前笔画末端，不补线；删除旧512步追赶/原始终点强连，不自动补压力。同步Ctrl++/Ctrl+-主窗/浮窗/文本保护、前景/背景及历史色正常关闭重启保存、多文档单写入者同步。Rust8、IO4、Qt6功能及Release/Clippy/ABI通过，见[验收](brush-follow-color-zoom-validation.md)。无ABI或格式版本变化，其他完整目标范围继续后续。

2026-10-16自动预览修订：程序/图片/颜色动态/双色/湿混均固定白色并保留真实alpha，当前两色不再使缓存失效；五类配置实际工作区逐像素、五项相关既有功能、Release/生成头/ABI通过，见[验收](brush-white-preview-validation.md)。用户取消试画，未接入代码和入口已删除并从目标移除；真实绘画颜色、载色、历史及全局颜色保存保持。其他完整系统范围继续后续。

2026-10-16起收笔数值/调度阶段：独立尺寸与opacity曲线、最低值/重叠、双流全局弧长、只保留活动笔尾的事务准备/采用已实现，10项相关Rust测试与Clippy通过，见[验收](brush-taper-kernel-validation.md)。这是待接入的执行内核，不是用户可用的起收笔功能；Document实际像素回算/检查点/历史、持久化/原生/Qt仍待完成，无ABI或格式变化。其余完整系统范围保持，跨平台后置。

2026-10-16起收笔实际绘画阶段：Rust明确入口、有界活动笔尾图层/两份贡献场及执行检查点、随机/单双流/双色/湿混涂抹、原子单历史和失败整笔回滚已接，见[验收](brush-taper-document-validation.md)。10项新用例含40组一次性参考像素与载色，8项平滑、52项相关集成及Windows Release/Clippy/ABI/Qt通过。起收笔预设/基准/state/VBR、原生及Qt参数仍待接，不宣称界面功能已完成，无ABI或格式变化；完整ABR/16位、湿混双色/橡皮、非分隔和VBR分组等完整范围保持。

2026-10-16起收笔保存阶段：不可变预设/state18/VBR1.22/behavior15、启用/关闭完整曲线与舍入前严格验证、默认旧版本和混包最高需求、真实两笔/载色/来源与新资源目录交换已接，见[验收](brush-taper-io-validation.md)。新增6项IO、相关旧回归12项、新增原生保护1项、Schema及Windows Release/Clippy/ABI/Qt通过。ABI1.50和旧Qt入口暂拒新配置；明确支持的新原生准备/包接口、Qt镜像/重置基准/设置与白色预览继续后续，尚无起收笔界面入口。完整目标其余范围保持。

2026-10-16起收笔原生阶段：ABI1.51/STROKE_TAPER、显式state18准备和VBR1.22包读写、准确票据/旧图片修订重建与真实异步两笔/湿混载色/历史已接，见[验收](brush-taper-abi-validation.md)。6项公开API（新3/旧湿混3）、2项旧入口保护及Windows Release/Clippy/C/C++/Qt相关回归通过。旧入口保护继续保留；Qt镜像/关闭值/重置基准、设置与白色预览参数尚待接，仍无起收笔界面入口；完整系统其余范围不变。

2026-10-16湿混双色原生阶段：ABI1.52/WET_DUAL_COLOR，完整state19准备/VBR1.23包、双角色原子状态及手动装载、准确图片修订重建与真实两笔像素/载色/历史已接，见[设计](brush-wet-dual-abi-design.md)与[验收](brush-wet-dual-abi-validation.md)。新3旧6公开API、10核心/7原生及Windows Release/Clippy/4ABI/Qt通过；旧入口保护保留，Qt镜像/关闭值/基准/白色预览及组合入口仍待接，不宣称用户界面已支持。其余完整范围保持，跨平台后置。

2026-10-16湿混双色Qt阶段：完整state19镜像/关闭配置/复制和resetState、新ABI1.52准备及VBR1.23包、两色手动装载和隔离白色预览已接通，真实绘画/历史/图片三资源用途与新目录导入重开像素验证见[验收](brush-wet-dual-ui-validation.md)。Windows新3旧6Qt、环境倍率2/Release/ABI通过；旧入口拒绝保护保留。大跨度密集图片输入仍会触发预算回滚，分块调度未实现；原参数输入偶发问题未定位。完整剩余范围见[功能清单](brush-system-remaining.md)，用户要求本阶段完成后等待指示。

2026-10-16输入分页数值/预算阶段：原始段的主副有序印章、弧长/设备插值与间距余量、只读准备/成功采用的有界事件页已实现；Document预算提取共用权重而保持旧限制。9项新测试（含14400组旧预算分类）及相关真实湿混/起收笔旧回归、Windows Release/Clippy/ABI通过，见[设计](brush-input-paging-design.md)及[验收](brush-input-paging-kernel-validation.md)。尚未接Document像素/笔尾和Actor调度，不宣称画布64px密集输入已修复；用户已恢复推进，整体Windows设备手感由用户自行验收。

2026-10-17输入分页Document阶段：明确Rust输入/继续/结束入口、真实主副写入与活动尾有界重放、一次滤波、完整载色及单历史/失败回滚已实现，13项新Document、10调度及相关旧核心/原生、Windows Release/Clippy/ABI通过，见[设计](brush-input-paging-document-design.md)和[验收](brush-input-paging-document-validation.md)。原64px在新Rust入口完成并对照一次性参考；Actor与公开Qt尚未接，不宣称用户画布问题已修复。无ABI/格式变化；整体设备手感由用户验收。

2026-10-17输入分页Actor阶段：实际Move/End接通Document有界页，完成水位/笔尾发布/单历史与双载色采用保持，邮箱及暂存共同限额、页间取消/重建/stop和屏障顺序已接。原64px单输入经公开API和Qt新目录交换重开对照通过；任务8项、新Actor6项及相关原生/公开回归、Windows Release/Clippy/4 ABI/Qt通过，见[设计](brush-input-paging-actor-design.md)及[验收](brush-input-paging-actor-validation.md)。无ABI/格式变化；大笔刷性能并非全部完成，单印章及总输入预算保持，整体设备手感由用户验收。

2026-10-17湿混双色贡献场性能：主/第二几何与RGBA场合并覆盖遍历、零覆盖跳过无效颜色采样，保留独立日记/COW及饱和后继续混色。162组原两遍路径逐场/像素/载色对照、起收笔分页及相关回归通过；8组固定Release基准摘要相同，耗时减少4.1%至29.5%，不能外推所有场景。见[设计](brush-wet-field-performance-design.md)及[验收](brush-wet-field-performance-validation.md)。无ABI/格式变化，单印章和总工作预算保持。

2026-10-17图片光标原生阶段：实际覆盖的白色/alpha图、双线性面积积分及8项有界PNG资源缓存、ABI1.53查询与容量/生命周期已实现。新内核5项、资源9项/公开3项及旧轮廓4项、相关Clippy通过，见[设计](brush-cursor-image-design.md)及[验收](brush-cursor-image-validation.md)。无state/VBR变化；Qt显示开关、透明度全局保存及在途身份消费尚未接，不宣称用户入口已完成。

2026-10-17图片光标Qt阶段：ABI1.53白色PNG后台消费、默认关闭/25%可调全局偏好、当前与在途主副最多4身份、工具/简化/仅悬停及独立轮廓显示已接；主副捕获/重开恢复、失败合法状态回退、多文档同步和重启保存、实际窗口像素叠加通过。见[设计](brush-cursor-image-ui-design.md)及[验收](brush-cursor-image-ui-validation.md)。Windows新2旧4 Qt、环境倍率2新2、Release/4 ABI通过，无ABI/格式变化；随机/纸纹/最终混色预演不属于基础图片模式，其他完整目标继续。

2026-10-17笔刷分组Rust目录阶段：不可变稳定ID组织/有序根及子项/DFS投影、原子建组改名移动/解组及受保护级联删除、同组复制导入与捕获保持已接；新6集成+1修订边界/旧5目录、IO5/原生2及Windows Release/Clippy/4 ABI/旧Qt2通过，见[设计](brush-groups-design.md)及[验收](brush-groups-kernel-validation.md)。旧状态编码有组时明确拒绝，未接分组保存交换/原生及Qt，不宣称用户入口完成；无ABI/state/VBR版本变化。

2026-10-17分组状态阶段：有界ordered-tree-v1 DTO/state20已实现，空组/显式顺序/关闭参数及真实三图片用途/ABR来源跨新目录两笔像素载色历史往返；最终IO20项、新旧原生四入口保护1项及Schema/Windows Release/Clippy/4 ABI/旧Qt2通过，见[设计](brush-groups-state-design.md)及[验收](brush-groups-state-validation.md)。平目录旧字节保持；旧原生有组明确拒绝。无ABI或VBR版本变化，VBR分组清单/整树映射/原生和Qt仍待接，不宣称用户分组完成。

2026-10-17分组VBR阶段：VBR1.24组织清单/空组与顺序、原子整树副本ID映射已接，三图片用途/ABR来源及真实两笔像素载色历史保持；IO33项、新四类旧读写保护1项、Schema/Windows Release/Clippy/4 ABI/旧Qt2通过，见[设计](brush-groups-vbr-design.md)及[验收](brush-groups-vbr-validation.md)。平包旧字节保持，无ABI/state变化；目标库原子合并/完整原生交接/Qt仍待接，不宣称分组用户入口完成。

2026-10-17分组目标目录合并阶段：Core支持完整有序子树在根/指定组位置原子合并、跨角色身份/合并深度容量与资源预算预检、单修订及失败保持；VBR真实资源和来源内存桥已接，两笔像素/双载色/历史保持。新核心4/旧11/修订1、IO21及Windows Release/Clippy/4 ABI/旧Qt2通过，见[设计](brush-groups-merge-design.md)及[验收](brush-groups-merge-validation.md)。无ABI/state/VBR变化；资源文件耐久事务、完整原生交接/Qt仍待接，不宣称分组用户入口完成。

2026-10-17分组资源发布阶段：候选目录先完整合并预检、包内PNG与来源先落盘后采用；故障保留原目录/共享文件，真实新目录state/组织与两笔像素载色历史恢复通过。IO40、Clippy/Windows Release/4 ABI/旧Qt2通过，见[设计](brush-groups-persist-design.md)及[验收](brush-groups-persist-validation.md)。无ABI/state/VBR变化；失败可能保留合法未引用公共缓存，不声称多文件整体回滚/GC。完整原生交接/Qt及重置基准全链仍待接。
