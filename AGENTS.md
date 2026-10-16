# DrawVerse

- 最新空图层栈：`docs/empty-layer-stack-design.md` / `docs/empty-layer-stack-validation.md`；最后图层/整组/批量可删空，active=0与透明棋盘格、历史/新建/空栈粘贴、ORA零层往返和透明PNG；Windows最小Rust/Qt及ABI通过，无布局/格式版本变化。

- 最新起收笔 Qt：`docs/brush-taper-ui-design.md` / `docs/brush-taper-ui-validation.md`；state18完整镜像/关闭值/复制重置基准、ABI1.51准备与VBR1.22入口、距离和独立平滑尺寸/opacity曲线、真实白色预览/在途捕获/像素历史及重开，Windows最小Qt通过；完整ABR/16位、湿混双色等仍待后续。

- 最新工具栏传递参数：`docs/brush-toolbar-transfer-design.md` / `docs/brush-toolbar-transfer-validation.md`；画笔/橡皮当前预设的流量、不透明度及各自压感直接同步，涂抹下拉改细分隔线，Windows 最小三项 Qt 通过，无 ABI/格式变化。

- 最新起收笔原生入口：`docs/brush-taper-abi-design.md` / `docs/brush-taper-abi-validation.md`；ABI1.51/STROKE_TAPER、显式state18准备与VBR1.22包读写、准确捕获/旧图片修订重建和真实两笔/载色/历史已验证；旧入口保护保留，Qt镜像/基准/白色预览和设置页仍待接。

- 最新起收笔保存交换：`docs/brush-taper-io-design.md` / `docs/brush-taper-io-validation.md`；不可变预设/state18/VBR1.22/behavior15、完整关闭曲线和严格舍入前验证、真实两笔/载色/来源与全新目录交换通过；ABI1.50旧入口暂拒新对象防Qt丢值，完整原生/Qt镜像和重置基准仍待接。

- 最新起收笔实际绘画：`docs/brush-taper-document-design.md` / `docs/brush-taper-document-validation.md`；Rust明确入口/真实活动笔尾瓦片与贡献场恢复、单双流/双色/湿混/涂抹、单历史和预算/暂存失败整笔回滚，10项新像素用例/相关旧回归及Windows Release/ABI/Qt通过；预设IO/原生/Qt参数仍待接，无ABI或格式变化。

- 最新起收笔数值与调度：`docs/brush-taper-kernel-design.md` / `docs/brush-taper-kernel-validation.md`；独立尺寸/opacity曲线、全局弧长与双流身份、事务计划及有界活动笔尾，10项Rust与Clippy通过；尚未接Document像素/历史/IO/ABI/Qt，不宣称画布起收笔完成，无版本变化。

- 最新白色预览与范围修订：`docs/brush-white-preview-design.md` / `docs/brush-white-preview-validation.md`；用户取消试画/代码和入口已清理，所有自动预览固定白色及正常模式/保持真实alpha，换当前颜色不重算；真实画布颜色不变，无ABI或格式变化。

- 最新平滑/缩放/颜色保存：`docs/brush-follow-color-zoom-design.md` / `docs/brush-follow-color-zoom-validation.md`；用户确认保留拖绳、抬笔停在已绘末端不补线，Ctrl++/Ctrl+-主窗浮窗与文本保护，前景/背景及历史色偏好重启保持/多文档同步；无ABI或格式版本变化。

- 最新ABR设备形状导入：`docs/abr-device-conversion-design.md` / `docs/abr-device-conversion-validation.md`；converter7/record1/交接JSON6/VBR1.21，方向/旋转/倾斜角度与圆度/独立动态和旧六代保持；PS设备公式与下限换算标近似，关闭设备仅来源保留，16位/其他设备继续后续，无C ABI或state新版本。

- 最新Windows输入/菜单修复：`docs/brush-windows-ink-hover-design.md` / `docs/tablet-menu-routing-design.md` / `docs/windows-input-menu-validation.md`；笔位置所有权隔离Quick缓存Hover、原窗口光标先保存后切换、鼠标/笔菜单坐标路由与透明模态屏障；兼容窗口不激活防失焦关闭，实际Windows最小结果见验收，无ABI/格式变化。

- 最新ABR形状抖动导入：`docs/abr-shape-conversion-design.md` / `docs/abr-shape-conversion-validation.md`；converter6/record1/交接JSON5/VBR1.20，尺寸角度圆度随机与最低比例、独立压力散布颜色/完整关闭配置、旧五代来源保持；PS随机及下限叠加标近似，设备形状控制/16位等继续后续，无C ABI或state新版本。

- 最新ABR颜色动态导入：`docs/abr-color-conversion-design.md` / `docs/abr-color-conversion-validation.md`；converter5/record1/交接JSON4/VBR1.19，前景背景与HSV/逐笔逐印章和完整关闭配置、独立压力散布与旧四代来源保持；PS颜色空间/随机明确近似，纯度/设备颜色控制及16位等继续后续，无C ABI或state新版本。

- 最新ABR散布导入：`docs/abr-scatter-conversion-design.md` / `docs/abr-scatter-conversion-validation.md`；converter4/record1/交接JSON3/VBR1.18，真实数量Double与单/双轴/数量抖动、完整关闭/旧三代基准保持、四代交换及Windows Qt新库像素；PS随机分布标近似，设备散布/16位等继续后续，无C ABI或state新版本。

- 当前验收优先级（用户2026-10-15确认）：主要针对Windows平台；跨平台验收以后再做，不作为当前阶段提交/推送门槛。继续维护跨平台架构，实际验证范围如实记录。

- 最新真实ABR语料/对齐：`docs/abr-real-corpus-design.md` / `docs/abr-real-corpus-validation.md`；8BIM节间/尾部四字节填充、旧无填充兼容，CC0 Myer148与两个MIT Photoshop探针完整导入/来源/像素历史及Qt新库VBR；无版本变化，16位字节序分歧与后续证据见`docs/abr-16bit-investigation.md`，仍未实现16位。

- 最新ABR笔压动态：`docs/abr-pressure-design.md` / `docs/abr-pressure-validation.md`；转换器v3/来源record1/交接JSON2/VBR1.17，尺寸流量opacity独立压感与最低比例、旧v1/v2字节/基准保持、三代混合/降级拒绝及真实Qt导入重置/像素/重开；无C ABI或state新版本，完整ABR动态/16位、湿混双色、收尖等继续后续。

- 最新数位笔悬停修复：`docs/brush-tablet-hover-fix-design.md` / `docs/brush-tablet-hover-fix-validation.md`；双色页移除两个预设按钮/既有预设保留，直接入点与笔触后Stylus Hover跟随、设备几何保持/原生窗口光标有界接管与恢复、环境缩放1/2相关Qt回归；无ABI或格式变化。

- 最新ABR整笔不透明度：`docs/abr-opacity-design.md` / `docs/abr-opacity-validation.md`；转换器v2/来源record1/VBR1.16，非100%真实整笔上限/密集程序与图片、v1原基准/报告及记录字节不重算、新旧混合最高minor/降级拒绝、原生prepared与Qt导入/重置/新库交换；无ABI/state变化，完整ABR动态/16位、双色湿混及收尖等继续后续。

- 最新二维湿混涂抹：`docs/brush-wet-smudge-design.md` / `docs/brush-wet-smudge-validation.md`；独立R共用载体/强度压感与工具取样、手指自动载色/透明搬运/蒙版线性灰度、真实像素/分页组合/原生两笔及Qt历史，追加原生高DPI悬停计时输入隔离与重复回归（环境倍率非硬件DPR）；无ABI/格式变化，湿混双色/橡皮、收尖/非分隔与高级ABR等继续后续。

- 最新湿混Qt入口：`docs/brush-wet-qt-design.md` / `docs/brush-wet-qt-validation.md`；ABI1.50/state17/VBR1.15完整镜像/独立基准、湿混分类与通用控件、手动载色/清洗/主文档共享工具、真实独立有色预览及DPI1/2交互/像素/持久化；新版准备/库包接通，旧接口保护保留；双色湿混/Erase/Smudge、收尖/非分隔及高级ABR继续后续。

- 最新湿混原生入口：`docs/brush-wet-abi-design.md` / `docs/brush-wet-abi-validation.md`；ABI1.50/共享工具句柄与64所有者上限、准确准备手动载色/清洗/状态/跨会话绑定、state17/库包显式wet入口与真实两笔/资源交接；旧入口继续拒绝防旧Qt丢值，完整Qt镜像/基准/有色预览与双色湿混继续后续。

- 最新湿混工具所有权：`docs/brush-wet-owner-design.md` / `docs/brush-wet-owner-validation.md`；共享WetTool/独占租约、原生Engine配置起笔/历史成功采用、取消/预算失败/New/stop释放与跨文档Busy、真实两笔参考像素/载色；无ABI变化，外部工具句柄/准备桥/Qt及双色湿混继续后续。

- 最新湿混保存交换：`docs/brush-wet-io-design.md` / `docs/brush-wet-io-validation.md`；state17/VBR1.15/behavior14、完整关闭策略与六比例、真实两笔载色/像素/历史及图片来源保持、Schema与合法非法示例；ABI1.49原生准备/包入口暂拒防旧Qt丢值，完整桥/Qt与双色湿混继续后续。

- 最新湿混统一设置：`docs/brush-wet-settings-design.md` / `docs/brush-wet-settings-validation.md`；Rust不可变预设/编译快照、配置起笔自动清洗装载与跨笔保色/路径重置/尺寸重采样、真实像素与历史；无ABI/格式变化，旧IO非默认拒绝，完整湿混交换/桥/Qt与双色组合继续后续。

- 最新湿混颜色动态：`docs/brush-wet-color-design.md` / `docs/brush-wet-color-validation.md`；逐印章准备/沉积后线性HSV、alpha与原始载色保持、真实分页组合及故障回滚、旧算法20736组位级核对；无ABI/格式变化，湿混IO/桥/Qt与双色组合继续后续。

- 最新湿混文档内核：`docs/brush-wet-document-design.md` / `docs/brush-wet-document-validation.md`；独立Rust起笔/结束返回载色、真实逐印章取样/分页RGBA贡献与opacity上限、选区/蒙版/锁及单历史、失败/滤波预算回滚；无ABI/格式变化，参数IO/桥/Qt与颜色动态/双色组合仍待后续。

- 最新二维湿混载色内核：`docs/brush-wet-carrier-design.md` / `docs/brush-wet-carrier-validation.md`；真实二维预乘RGBA拾色/混合/沉积/距离消耗、清洗重载与准备/提交快照、64阶有界网格及数值像素回归；无ABI/格式变化，尚未接Document/历史/预设/Qt，不宣称湿混工具已完成。

- 最新可选复合光标：`docs/brush-composite-cursor-ui-design.md` / `docs/brush-composite-cursor-ui-validation.md`；ABI1.49消费/默认关闭的全局开关、主第二动态与图片轮廓/在途身份/最多4个后台请求、关闭不生成第二轮廓、DPR1/2和数位笔伴随Hover/鼠标接管；无格式变化，随机印章预演/非分隔/收尖/湿混等继续后续。

- 最新复合笔尖原生几何：`docs/brush-composite-pose-design.md` / `docs/brush-composite-pose-validation.md`；ABI1.49/64字节双姿态DTO、同prepared捕获/独立压力设备/相对及固定边宽/图片与有效溢出标志，实际印章尺寸及C/C++布局回归；无state/VBR变化，Qt双轮廓尚待接入，不宣称完整复合光标。

- 最新自动描边预设：`docs/brush-auto-outline-design.md` / `docs/brush-auto-outline-validation.md`；双色页创建跟随压感/固定边宽独立用户预设，配对硬圆/+8px/背景后方/完整基准与库上限，真实压力边宽/交叉整笔opacity/捕获及VBR历史；无ABI/格式变化，复合光标、非分隔、收尖与湿混等继续后续。

- 最新双色原生Qt入口：`docs/brush-dual-color-ui-design.md` / `docs/brush-dual-color-ui-validation.md`；ABI1.48/state16/VBR1.14、完整关闭配置/27贡献模式/实际两色预览、起笔捕获/历史及保存交换，重置预设仅来源与兼容页；解除原生准备/库包临时拒绝，自动描边与复合光标等继续后续。

- 最新双色重算性能：`docs/brush-dual-color-performance-design.md` / `docs/brush-dual-color-performance-validation.md`；变化行位图OR/精确像素重算、before与私有写入/快照、实际区域预算及1%密集段、648组真实瓦片历史一致与Release基准；无ABI变化，双色原生Qt/描边及复合光标继续后续。

- 最新双色双笔刷IO：`docs/brush-dual-color-io-design.md` / `docs/brush-dual-color-io-validation.md`；state16/VBR1.14/behavior13、稳定27模式名称与关闭配置、相对/平滑/PCHIP真实两笔及图片来源保持；ABI1.47原生/Qt暂拒新色彩配置，重算性能/入口/描边与复合光标继续后续。

- 最新双色双笔刷内核：`docs/brush-dual-color-core-design.md` / `docs/brush-dual-color-core-validation.md`；真实两场分页/起笔before隔离合成、前后/背景/溢出与min软裁切、27贡献/工具混合/一次整笔opacity、选区/蒙版/锁及存储原子回滚，旧IO非默认明确拒绝；无ABI变化，格式/原生Qt/描边与复合光标继续后续。

- 最新双笔刷相对尺寸原生Qt：`docs/brush-dual-size-ui-design.md` / `docs/brush-dual-size-ui-validation.md`；ABI1.47/state15/VBR1.13、居中偏移/跟随与固定宽度/标称尺寸说明、关闭/重置与最高版本、真实预览和在途像素/VBR重开；解除临时入口拒绝，双色合成/溢出/描边与复合光标继续后续。

- 最新双笔刷尺寸IO：`docs/brush-dual-size-io-design.md` / `docs/brush-dual-size-io-validation.md`；state15/VBR1.13/behavior12、严格嵌套sizing/关闭与独立保留、组合时间平滑真实两笔与最高版本、ABI1.46原生准备/库包入口暂拒防旧Qt丢值；原生Qt/双色合成继续后续。

- 最新双笔刷相对尺寸内核：`docs/brush-dual-size-design.md` / `docs/brush-dual-size-validation.md`；独立/相对直径偏移、跟随/固定宽度、实际主动态与独立流明确定义映射、保守工作预算和旧IO非默认拒绝；无ABI变化，格式/原生Qt/双色合成与描边继续后续。

- 最新低流量混合性能：`docs/brush-mix-performance-design.md` / `docs/brush-mix-performance-validation.md`；f32覆盖精确停滞时跳过重复颜色混合、保留纸纹与工具翻转，27模式/颜色动态/双路径真实瓦片与历史对照，软边反复覆盖Release基准；无ABI或文件格式变化。

- 最新时间平滑原生Qt：`docs/brush-smoothing-ui-design.md` / `docs/brush-smoothing-ui-validation.md`；ABI1.46/state14/VBR1.12、完整位置/压力/速度/稳定距离分类、真实时间预览/在途捕获/保存重开与重置、解除准备/库包临时拒绝；收尖/双色/湿混/高级ABR继续后续。

- 最新时间平滑IO：`docs/brush-smoothing-io-design.md` / `docs/brush-smoothing-io-validation.md`；state14/VBR1.12/behavior11、f64完整关闭配置/默认旧字节、压感与双路径/图片来源真实时间笔触往返；ABI1.45准备/库包暂拒新配置防旧Qt丢字段，原生桥/Qt与收尖继续后续。

- 最新混合性能/菜单与预览：`docs/brush-mix-performance-design.md` / `docs/brush-mix-performance-validation.md`；圆尖平方距离/饱和主与第二覆盖跳过、整笔像素/历史保持、工具菜单跨原生窗置顶与Qt6.5回退、预览固定正常及跨文档缓存保持，无ABI变化。

- 最新自动平滑压感：`docs/brush-pressure-smooth-design.md` / `docs/brush-pressure-smooth-validation.md`；ABI1.45/state13/VBR1.11/behavior10、统一PCHIP/纯原生图形采样、旧曲线保留/编辑自动平滑、关闭配置与重置基准，重置按钮在参数末尾。
- 最新画笔混合Qt：`docs/brush-blend-ui-design.md` / `docs/brush-blend-ui-validation.md`；27模式独立工具状态/在途捕获、共享持久化及真实有色底预览，双色扩展继续后续。

- 最新画笔混合原生捕获：`docs/brush-blend-abi-design.md` / `docs/brush-blend-abi-validation.md`；ABI1.44/16字节独立模式DTO/新prepared导出、两色与模式复制/准确票据及历史、旧入口正常兼容；Qt模式下拉/偏好/有色底预览待接，不改state/VBR。

- 最新画笔颜色混合内核：`docs/brush-blend-design.md` / `docs/brush-blend-validation.md`；27模式/整笔贡献与before、蒙版可见灰度/透明锁精确alpha、文档坐标溶解/预算与历史；无ABI变化，工具状态/原生Qt下拉与真实预览待接，双色及自动描边随后。

- 最新重置预设：`docs/brush-reset-design.md` / `docs/brush-reset-validation.md`；设置按钮/完整工厂与副本导入基准、独立有界resetState持久化/重开及原生规范化、资源恢复与在途捕获；无ABI变化，双色新字段随后接入。

- 最新时间平滑内核：`docs/brush-smoothing-design.md` / `docs/brush-smoothing-validation.md`；4ms网格/位置速度响应/独立压力/拖绳与有界终点、双路径共用/累计预算/分页历史；无ABI变化，非默认旧IO拒绝，状态/VBR/Qt与收尖待接，普通画笔混合模式按用户优先级先推进。

- 已确认双笔刷扩展（待实施）：`docs/brush-dual-color-extension-design.md`；第二颜色、相对像素尺寸/溢出/正常双色/前后顺序，起笔两色捕获、整笔分开累积合成和两种压感边宽；含自动描边条件，已再次发令继续；补充笔刷设置“重置预设”按钮及完整持久基准。

- 最新双笔刷原生Qt：`docs/brush-dual-ui-design.md` / `docs/brush-dual-ui-validation.md`；ABI1.43/state12/VBR1.10、四子页/第二图片捕获/独立曲线动态、真实比例预览与像素历史、关闭保存及资源并集交换；库package解除临时拒绝，组合光标/平滑/湿混/ABR高级映射继续后续。

- 最新双笔刷准备桥：`docs/brush-dual-bridge-design.md` / `docs/brush-dual-bridge-validation.md`；第二独占/三角色共享和关闭配置交接、发布/mutate采用、在途保留与闲置源释放、state12准确重建/真实像素历史；无ABI变化，完整Qt和资源导出继续后续。

- 最新双笔刷IO：`docs/brush-dual-io-design.md` / `docs/brush-dual-io-validation.md`；state12/VBR1.10/behavior9、两模型/完整第二动态与资源/真实两笔还原、来源并集/7169条目和状态字节预算；ABI1.42包暂拒绝新对象，原生资源生命周期和Qt继续后续。

- 最新双笔尖独立路径：`docs/brush-dual-path-design.md` / `docs/brush-dual-path-validation.md`；第二直径1%间距/双流调度、独立随机字段/主频率无关、稀疏分页与终点/真实四类写入历史、预算及存储失败原子回滚；无ABI变化，旧配对保持，保存交换和原生Qt继续后续。

- 最新双笔尖覆盖核心：`docs/brush-dual-design.md` / `docs/brush-dual-validation.md`；paired-dab-v1六模式/独立第二压力设备与形状散布、共享图片预算/预检与真实四类写入历史；旧state/来源拒绝非默认对象，无ABI变化，独立路径间距/IO与原生Qt继续后续。

- 最新纹理原生Qt：`docs/brush-texture-ui-design.md` / `docs/brush-texture-ui-validation.md`；ABI1.42/state11/VBR1.9、捕获目标PNG导入/纹理分类/关闭编辑、统一真实预览与像素历史、同源图片与原始来源交换/窄列可见；解除库package临时拒绝，双笔刷/平滑/湿混和ABR动态继续后续。

- 最新纹理准备桥：`docs/brush-texture-bridge-design.md` / `docs/brush-texture-bridge-validation.md`；纹理独占/共用与关闭交接、在途保持/闲置源和mip释放、state11准确重建及真实像素历史；无ABI变化，Qt和公开包入口继续后续。

- 最新纹理IO：`docs/brush-texture-io-design.md` / `docs/brush-texture-io-validation.md`；state11/VBR1.9/behavior8、预设资源绑定/共享周期mip/预算预检、同源PNG与真实两笔/来源保持、6145条目边界；ABI1.41包暂拒绝新对象，原生Qt待接。

- 最新纹理内核：`docs/brush-texture-design.md` / `docs/brush-texture-validation.md`；文档纸纹/笔尖局部、周期边界与面积mip/专用覆盖、资源共享预算/真实四类写入及历史；旧IO拒绝新配置，资源IO/VBR/原生Qt待接，无ABI变化。

- 最新颜色动态原生Qt：`docs/brush-color-ui-design.md` / `docs/brush-color-ui-validation.md`；ABI1.41/背景DTO与旧入口兼容、两色捕获/state10/VBR1.8/模型、颜色分类/关闭可编辑/真实预览与像素/历史；解除package临时拒绝，纹理/双笔刷/湿混与ABR动态继续后续。

- 最新颜色动态IO：`docs/brush-color-io-design.md` / `docs/brush-color-io-validation.md`；state10/VBR1.8/behavior7、严格线性HSV模型/完整种子/关闭频率、真实组合两笔及来源保持；ABI1.40包入口暂拒绝新对象，原生背景与Qt待接。

- 最新颜色动态内核：`docs/brush-color-dynamics-design.md` / `docs/brush-color-dynamics-validation.md`；两色/线性HSV/整笔与逐印章、分页颜色贡献与透明度上限、真实蒙版/手指载色/历史；旧IO拒绝新配置，保存/VBR/原生背景与Qt待接，无ABI变化。

- 最新形状抖动原生Qt：`docs/brush-shape-jitter-ui-design.md` / `docs/brush-shape-jitter-ui-validation.md`；ABI1.40/state9/VBR1.7、形状动态子页/真实预览与捕获/完整种子关闭值、最高目录版本/图片来源往返；解除package临时拒绝，其他动态与湿混继续后续。

- 最新形状抖动IO：`docs/brush-shape-jitter-io-design.md` / `docs/brush-shape-jitter-io-validation.md`；state9/VBR1.7/behavior6、独立对象/完整种子/舍入前范围、真实组合两笔还原/图片来源保持；ABI1.39包入口暂拒绝保护旧Qt，原生Qt继续后续。

- 最新形状抖动内核：`docs/brush-shape-jitter-design.md` / `docs/brush-shape-jitter-validation.md`；不可变大小/角度/圆度与下限、独立随机域/实际散布子印章、程序图片和四类写入/名义预算与历史；旧IO明确拒绝非默认新配置，状态/VBR/原生Qt继续后续，无ABI变化。

- 最新散布原生Qt：`docs/brush-scatter-ui-design.md` / `docs/brush-scatter-ui-validation.md`；ABI1.39/state8/VBR1.6、PS分类/真实统一预览与捕获笔触、完整种子/关闭值/目录最大版本、图片来源往返；解除package临时拒绝，其他动态与湿混继续后续。

- 最新散布IO：`docs/brush-scatter-io-design.md` / `docs/brush-scatter-io-validation.md`；state8/VBR1.6/behavior5、独立散布对象/完整u64种子、真实两笔还原与原始来源保持；ABI1.38包入口暂拒绝新对象保护旧Qt，原生Qt继续后续。

- 最新散布内核：`docs/brush-scatter-design.md` / `docs/brush-scatter-validation.md`；确定性计数器/单轴双轴/数量抖动、输入拆分一致、共用四类写入/实际涂抹载色、展开预算预检与原子历史；旧IO明确拒绝非默认散布，保存/原生Qt继续后续，无ABI变化。

- 最新实时动态光标：`docs/brush-live-pose-design.md` / `docs/brush-live-pose-validation.md`；ABI1.38/可复用捕获Pose、非阻塞原生几何/未知悬停压力/压力大小、Qt旋转倾斜方向/在途图片身份与DPI；其余动态/湿混继续后续。

- 最新设备动态Qt入口：`docs/brush-device-ui-design.md` / `docs/brush-device-ui-validation.md`；ABI1.37/state7/VBR1.5、形状动态参数/真实预览和旋转笔触/复制持久化、解除package临时拒绝；实时动态光标和其余动态湿混继续后续。

- 最新设备笔尖动态IO：`docs/brush-device-io-design.md` / `docs/brush-device-io-validation.md`；state7/VBR1.5/behavior4、独立tipDynamics/严格枚举范围/逐像素恢复、压力与图片来源共存；现有package入口暂拒绝新数据防Qt丢失，原生Qt与光标继续后续，无ABI变化。

- 最新设备笔尖动态内核：`docs/brush-device-dynamics-design.md` / `docs/brush-device-dynamics-validation.md`；不可变角度来源/倾斜圆度、每印章共用几何/图片完整边界、短路径旋转与能力回退；IO非默认明确拒绝，状态/VBR/原生Qt/动态光标继续后续，无ABI变化。

- 最新不透明度压感：`docs/brush-opacity-pressure-design.md` / `docs/brush-opacity-pressure-validation.md`；ABI1.36/v6/VBR1.4、独立压力局部整笔上限/曲线最低比例、关闭数据保留、Qt真实像素与交换；其余动态/湿混/ABR动态继续后续。

- 最新独立尺寸与整笔不透明度：`docs/brush-stroke-opacity-design.md` / `docs/brush-stroke-opacity-validation.md`；活动预设拥有实际大小、旧共享断言替换、低不透明度整笔上限与分页覆盖/撤销、数位笔绘画位置优先；独立不透明度压感仍待后续。

- 最新压力曲线入口：`docs/brush-pressure-ui-design.md` / `docs/brush-pressure-ui-validation.md`；ABI1.35、Qt v5/形状动态与传递曲线、原生真实预览与VBR1.3往返；解除package临时拒绝，其余动态/湿混继续后续。

- 最新压力动态IO：`docs/brush-pressure-io-design.md` / `docs/brush-pressure-io-validation.md`；v5/有界验证DTO、VBR1.3/行为2/pressure-curves、旧版本不升级/节点保留/源图共存与真实像素、准确原生准备；旧package暂拒绝压力防丢数据，ABI/Qt曲线继续后续，无ABI变化。

- 最新压力动态内核：`docs/brush-pressure-design.md` / `docs/brush-pressure-validation.md`；有界不可变曲线/最低比例/精确分段编译、尺寸流量与涂抹强度共用/旧像素与在途快照；旧IO明确拒绝非默认数据，状态ABI/VBR与编辑界面继续后续，无ABI变化。

- 最新VBR来源入口：`docs/vbr-source-entry-design.md` / `docs/vbr-source-entry-validation.md`；ABI1.34、原生完整来源交换/当前与基线PNG、待采用资源/基线mip释放、Qt v4/UUID副本/原ABR范围显式选项与切换保护、跨新仓库重开像素报告；分组扩展/完整动态湿混继续后续。

- 最新VBR来源IO：`docs/vbr-source-design.md` / `docs/vbr-source-validation.md`；VBR1.2三角色/原记录SHA/共享原包与当前基线PNG、批次导出/5121条目边界与Schema；旧原生接口明确拒绝防丢来源，原生Qt范围选项/分组动态继续后续，无ABI变化。

- 最新ABR用户入口：`docs/abr-entry-design.md` / `docs/abr-entry-validation.md`；ABI1.33、后台整包交接/准确资源采用、VBR与ABR统一按钮/UUID整包副本、部分数量与来源报告/删除输入后绘画恢复；完整ABR动态/旧参数16位、VBR来源与湿混继续后续。

- 最新ABR整包IO导入：`docs/abr-import-design.md` / `docs/abr-import-validation.md`；6/10全链编译/共享PNG与完整来源批次、单原包保存/总报告边界/顺序与像素验证；原生Qt入口、旧参数/16位与VBR来源继续后续，无ABI变化。

- 最新笔刷来源界面：`docs/brush-source-ui-design.md` / `docs/brush-source-ui-validation.md`；Qt v4保存/原生目录/副本保留来源、后台导入快照与逐项兼容报告/旧结果拒绝；来源VBR导出明确未支持，ABR用户入口与批量来源继续后续，无ABI变化。

- 最新原生来源准备桥：`docs/brush-source-bridge-design.md` / `docs/brush-source-bridge-validation.md`；ABI1.32、共享来源上下文/v4准确准备、Weak身份凭据/当前与在途独立持有、后台来源报告新校验；Qt/VBR来源/ABR入口继续后续。

- 最新原生来源引用/状态：`docs/brush-source-state-design.md` / `docs/brush-source-state-validation.md`；来源与执行分离/不可变修订/编辑副本保留、v4显式上下文与批次原始验证；准备桥/Qt/VBR与高频来源复用继续后续，无ABI变化。

- 最新笔刷来源资源：`docs/brush-source-design.md` / `docs/brush-source-validation.md`；完整原始来源/小型导入快照与报告、SHA身份/不覆盖保存/原字节交换；原生预设与状态/Qt/VBR接入、批量来源IO继续后续，无ABI变化。

- 最新ABR基础转换：`docs/abr-conversion-design.md` / `docs/abr-conversion-validation.md`；严格单位/范围、真实编译/共享覆盖、逐叶支持与未执行报告、外部无动态alpha核对；来源保存/原生Qt入口/动态与PS核验继续后续，无ABI变化。

- 最新ABR样本关联：`docs/abr-association-design.md` / `docs/abr-association-validation.md`；Pascal身份/可变前缀/sampledData严格关联、共享资源/完整来源字段；外部上游诊断像素匹配但资产不分发，参数报告/用户入口继续后续，无ABI变化。

- 最新ABR描述符：`docs/abr-descriptor-design.md` / `docs/abr-descriptor-validation.md`；有界typed tree/单位与引用/显式desc接入、长度与递归拒绝；具体PS参数/样本关联/用户入口/真实授权语料继续后续，无ABI变化。

- 最新ABR分节采样：`docs/abr-sections-design.md` / `docs/abr-sections-validation.md`；6/10×1/2有界分节/记录对齐/共用PackBits、原始来源元数据保留；desc执行/用户入口/16位/真实授权语料继续后续，无ABI变化。

- 最新ABR采样读取：`docs/abr-sampled-design.md` / `docs/abr-sampled-validation.md`；Rust1/2采样原始/PackBits/UTF16与来源字段、整包拒绝与真实孔洞测试；尚未接用户入口，6/10/程序/16位/动态报告和授权真实语料继续后续，无ABI变化。

- 最新VBR图片资源：`docs/vbr-image-design.md` / `docs/vbr-image-validation.md`；ABI1.31/VBR1.1、完整PNG身份/资源清单/整包编译/多资源交接、Qt跨资源目录导入导出/UUID副本；分组/扩展/ABR/动态湿混继续后续。

- 最新图片笔尖轮廓：`docs/brush-outline-design.md` / `docs/brush-outline-validation.md`；ABI 1.30、0.5阈值/孔洞与多区域/有界几何缓存、Qt当前身份与显示变换、DPI固定描边；超复杂纹理明确简化，图片悬停/动态设备/VBR图片/ABR继续后续。

- 最新图片笔尖Qt接入：`docs/brush-image-ui-design.md` / `docs/brush-image-ui-validation.md`；后台PNG三种解释/捕获目标/持久v3/独立真实预览、图片硬度禁用和程序恢复；图片精确轮廓/VBR图片/ABR/高级动态仍待完成。

- 最新图片笔尖原生资源接口：`docs/brush-resource-abi-design.md` / `docs/brush-resource-abi-validation.md`；ABI 1.29、后台PNG导入/共享资源目录、v3准确准备、闲置修订释放/重建及在途资源保留；Qt图片控件/VBR图片/ABR仍待接入。

- 最新图片笔尖身份/状态：`docs/brush-resource-state-design.md` / `docs/brush-resource-state-validation.md`；覆盖不可变PNG身份/v3哈希引用/后台资源恢复与整包编译，旧状态保留；原生资源目录/Qt/VBR图片/ABR仍待接入。

- 最新图片笔尖资源IO：`docs/brush-resource-design.md` / `docs/brush-resource-validation.md`；SHA-256内容身份/不覆盖原子PNG、8/16位覆盖转换与量化标记、Weak缓存/活跃mip预算，资源ID原生库/Qt接入与VBR图片/ABR仍待后续。

- 最新 Qt / 原生笔刷修订桥：`docs/brush-catalog-bridge-design.md` / `docs/brush-catalog-bridge-validation.md`；ABI 1.28、后台状态迁移/独立编译池/准确准备记录、Rust起笔等待与Qt共享库、隔离原生预览/有效状态保存；图片资源/ABR/动态/湿混继续后续。

- 最新原生笔刷库 ABI：`docs/brush-catalog-abi-design.md` / `docs/brush-catalog-abi-validation.md`；ABI 1.27、独立句柄/准确修订/原子目录操作、捕获预设的真实异步笔触及生命周期；Qt 镜像迁移与图片资源接口仍待接入。

- 最新 Rust 笔刷库模型：`docs/brush-catalog-design.md` / `docs/brush-catalog-validation.md`；验证编译/不可变修订、原子目录操作、旧预设兼容、1024预设/共享资源预算；原生库 ABI 与 Qt 完整迁移仍待接入。

- 最新图片笔尖内核：`docs/brush-mask-design.md` / `docs/brush-mask-validation.md`；不可变覆盖资源/mip、每印章采样计划、孔洞/非方形/旋转完整边界、资源快照与四类写入共用，图片 UI/VBR 资源/ABR 尚待接入，无 ABI 变化。

- 最新笔尖光标偏好：`docs/brush-cursor-settings-design.md` / `docs/brush-cursor-settings-validation.md`；实际/简化轮廓、中心/悬停/线宽/颜色、全局保存不进入 VBR、多文档笔刷状态同步与单一写入者移交，无 ABI 变化。

- 最新 VBR 程序子集：`docs/vbr-procedural-design.md` / `docs/vbr-procedural-validation.md`；ABI 1.26、Rust 有界 ZIP/JSON 与原子 IO、程序预设 UUID/能力/单位、后台导入导出、冲突副本与整包发布；资源/ABR 暂未实现。

- 笔刷系统：`docs/brush-system-design.md` / `docs/brush-system-validation.md` / `docs/vbr-format-design.md`；总体分阶段实施，ABR/VBR 等后续范围见验收计划。
- 最新程序笔尖：`docs/brush-tip-design.md` / `docs/brush-tip-validation.md`；ABI 1.25、共用编译覆盖、硬度/圆度/角度/流量与压感、旧圆笔兼容、预设 v2 原子恢复、PS 分类面板/真实预览/旋转椭圆光标。

- 最新 Ctrl 选层与临时移动：`docs/ctrl-layer-pick-design.md` / `docs/ctrl-layer-pick-validation.md`；ABI 1.24、异步像素命中/原子移动/锁与蒙版、图层搜索首次点击焦点、自由变换共用紧凑输入框。

- 最新图片与变换：`docs/transform-design.md` / `docs/transform-validation.md`；ABI 1.23、嵌入原始图片模型、Ctrl+T/八点控制/透视与网格、组/多选/选区像素原子变换、会话撤销重做、栅格化与 ORA v5 原图恢复。

- 最新液化历史：`docs/liquify-history-design.md` / `docs/liquify-history-validation.md`；ABI 1.22、整笔触会话撤销/重做、设置步数与校验暂存、历史域和滚动恢复、Ctrl+Z/Ctrl+Shift+Z、[ ] 液化尺寸。

- 最新液化性能：`docs/liquify-performance-design.md` / `docs/liquify-performance-validation.md`；连续预览非屏障、瓦片批处理与相邻取样缓存、共用选区覆盖缓存保留坐标、紧凑复选框与标题分隔，无 ABI 变更。

- 最新搜索与快捷键：`docs/layer-search-tools-design.md` / `docs/layer-search-tools-validation.md`；组层级搜索、文本编辑与画布焦点区分、快捷键恢复、液化仅滤镜入口及 Enter/Esc、图层拷贝粘贴仅快捷键。

- 最新液化：`docs/liquify-design.md` / `docs/liquify-validation.md`；ABI 1.21、滤镜入口、工具详情栏与图标、稀疏位移场与原始取样、事务历史、保存参数、文档锁定与浮窗隐藏恢复。

- 最新图层操作：`docs/layer-actions-design.md` / `docs/layer-actions-validation.md`；ABI 1.20、右键/主菜单、原子复制粘贴/子树合并、缩略图编辑焦点、Delete 与文本焦点保护、下边缘拖放修复。

- 最新组穿透：`docs/group-pass-through-design.md` / `docs/group-pass-through-validation.md`；ABI 1.19、组专用混合模式 27，子层对组外下层合成，opacity/fill/蒙版与剪贴、预览/撤销/ORA；新建组默认穿透、菜单置于正常上方。

- 最新吸色取色环：`docs/eyedropper-ring-design.md` / `docs/eyedropper-ring-validation.md`；按下拖动显示新旧颜色半环与灰色外圈、I/Alt/数位笔共用、显示开关、无 ABI 变更。

- 最新涂抹：`docs/smudge-design.md` / `docs/smudge-validation.md`；ABI 1.18、独立 R 工具复用笔刷、强度/压感/合成取样/手指绘画、最低 1% 间隔；参数说明统一悬停 1 秒。

跨平台绘画应用：Rust 内核 → 稳定 C ABI → C++20 / Qt 6.5+ UI；Python 插件最后实施。

- 最新魔棒：`docs/wand-design.md` / `docs/wand-validation.md`；ABI 1.17、四种组合/七种取样尺寸/默认容差 10/AA/连续/所有图层、灰度掩码与真实轮廓、历史和持久化。

- 最新油漆桶：`docs/bucket-design.md` / `docs/bucket-validation.md`；ABI 1.16、不透明度/容差（默认 0）/AA/连续/所有图层取样、选区与锁、单条原子历史、鼠标/数位板和浮动画布。

- 最新选区绘画性能：`docs/selection-performance-design.md` / `docs/selection-performance-validation.md`；框选/套索按行覆盖与有界缓存、画笔和橡皮共用、保留 AA/羽化/组合像素结果，无 ABI 变更。

- 最新共用选区与套索：`docs/lasso-selection-design.md` / `docs/lasso-selection-validation.md`；ABI 1.15、自由/多边形套索、四种组合与羽化/AA 共用、路径索引与历史/ORA、鼠标和数位板采集。

- 最新菜单悬停与多选建组：`docs/menu-hover-grouping-design.md` / `docs/menu-hover-grouping-validation.md`；ABI 1.14、统一指针预览、原生菜单完整坐标转发、首行跨边回归、多选一次建组与撤销。

- 最新面板折叠尺寸保持：`docs/panel-fold-sizing-design.md` / `docs/panel-fold-sizing-validation.md`；布局 v6 可选 retainedWidths，折叠仅压缩自身、工具栏宽度优先、浮窗收窄、相邻宽高与展开尺寸保持、显式调整解除保持。

- 最新菜单分类与图层批量修复：`docs/menu-layer-batch-design.md` / `docs/menu-layer-batch-validation.md`；ABI 1.13、共享悬停侧向子菜单、组删除双选择、原子多选删除/拖放、拖入剪贴链自动继承、图层面板首开异步创建。
- 最新遗留修复与吸色：`docs/deepseek-recovery-design.md` / `docs/deepseek-recovery-validation.md`；ABI 1.12、I 独立吸色与 Alt 临时吸色、DPI/视口坐标修复、剪贴链插层/基底变化、缩略图独立通知、剩余面板高度和恢复拖动。GPU/液化/.vbr/图层复制合并本轮暂缓。
- 当前阶段与验证命令：`README.md`、`docs/roadmap.md`。
- 最新真实历史标签：`docs/history-labels-design.md` / `docs/history-labels-validation.md`；ABI 1.10、真实工具名称/图标、裁剪后不显示伪初始状态；20ms 逐条历史回放保留。
- 最新面板细化与混合悬停预览：`docs/panel-refinement-design.md` / `docs/panel-refinement-validation.md`；ABI 1.9、浮窗拆分保留尺寸、颜色即时交换、共享紧凑控件、向外半透明描边、数位板悬停轮廓。替代 HEX 双击输入与延迟颜色交换。
- 检出/构建规则：`.gitattributes` / `docs/build-reproducibility.md`；文本 LF，生成头严格字节检查，每个工作树独立 Cargo target，tools/check.py 包含真实 Git 检出测试。
- 架构、UI 停靠与兼容性：`docs/architecture.md`。
- 正式 C ABI：`core/crates/paint-ffi/src/api.rs` / `ui/include/paint_api.h`；契约在 `contracts/abi.md`。
- `contracts/paint-api-v1.draft.h` 仅为历史设计；FFI 方案/验收在 `docs/ffi-design.md`。
- 当前文件 IO 与布局 v2 清理：`docs/io-design.md` / `docs/io-validation.md`；ABI 1.2 文件任务、Qt 文件菜单，笔记已退役。
- 当前实现：`core/crates/paint-core/src`、`core/crates/paint-ffi/src`、`ui/src`、`ui/qml` 与对应测试。
- 最新 UI 设计/验收：`docs/qml-ui-design.md` / `docs/qml-ui-validation.md`；用户指定 QML，旧 Widgets 设计已被取代。
- 最新参考图 UI / 停靠：`docs/reference-ui-design.md` / `docs/reference-ui-validation.md`；无边框紧凑工作区、原生浮窗、工具条拖放、布局 v4。用户要求每个模块验证后 commit 并推送 origin/master，持续推进；Python 插件最后。
- 最新自由停靠：`docs/flexible-docking-design.md` / `docs/flexible-docking-validation.md`；画布、面板组和工具条共用主窗/浮窗分割树，空分支收缩，布局 v5，替代固定左右列；只有一个可移换父项的主画布实例。
- 最新文档工作区 / 面板列：`docs/document-workspace-design.md` / `docs/document-workspace-validation.md`；多个独立 PaintCoreClient 文档、画布浮窗标签、工具列统一图标折叠、实际原生窗口拖动、布局 v6。画布与工具面板互不接收，主窗保留 #17191C 文档留空区。
- 最新常驻图标面板列：`docs/persistent-panel-rails-design.md` / `docs/persistent-panel-rails-validation.md`；每个图标独立打开、允许多开、空白与失焦不收回、最近点击置顶、侧边导航锚定与独立缩放、浮动列背景/高度及归位状态保留。替代此前唯一临时窗及空白关闭。
- 最新工具列与画布交互：`docs/workspace-tools-design.md` / `docs/workspace-tools-validation.md`；工具整列停靠、36–72px 宽度与拆分缩回、仅鼠标磁吸/外侧提示、折叠按钮拖动、布局像素位置保持、菜单置顶、画笔/橡皮独立大小与固定预览、颜色单双击、轮廓与跨面板空格平移；顶部 #05080A 75%，外描边 #63666B。
- 最新面板分组与统一渲染：`docs/panel-categories-design.md` / `docs/panel-categories-validation.md`；外侧整列停靠、内侧同组并列，布局 v6 可选 row 角色；图标按组整窗展开与居中分隔，双击收回不展开列，浮动列仅顶层栏透明，导航红框与共享内容/调色板，历史面板无撤销/重做按钮（逐条跳转保留）。
- 最新面板状态与信息栏：`docs/panel-presentation-design.md` / `docs/panel-presentation-validation.md`；同图标再次点击收回，独立宽高/侧边位置随布局 v6 保存；描边 #1B1C1F、小标签圆角、顶部 #05080A（浮动 75% 不透明度），紧凑底栏与中心缩放/适配取代工具条适合窗口按钮。
- 最新图标列 / 临时面板：`docs/icon-rail-design.md` / `docs/icon-rail-validation.md`；主工作区边缘优先归位、唯一图标折叠模式与状态继承、图标直接拖出、侧边临时窗切换/滑动/空白关闭、单画布浮窗一行标题。
- 最新窗口恢复 / 边缘吸附：`docs/window-recovery-design.md` / `docs/window-recovery-validation.md`；最小化恢复无边框与最大化状态、唯一临时面板外部输入/失焦关闭、统一 24px 最近边吸附和窄边发光、默认双列布局，参考图预设已删除。
- 最新任务栏 / 原生标题残留：`docs/taskbar-window-design.md` / `docs/taskbar-window-validation.md`；显式系统最小化能力、主窗非客户区绘制过滤、恢复后真实合成像素回归、灰黑细分隔线。
- 最新菜单底层圆角模糊：`docs/menu-blur-layer-design.md` / `docs/menu-blur-layer-validation.md`；独立非交互原生窗口、HostBackdropBrush 与合成视觉圆角裁剪、最小化前隐藏、最大化拖动恢复普通窗口并跟随鼠标。
- 最新几何选区：`docs/selection-design.md` / `docs/selection-validation.md`；ABI 1.8、有界几何、笔刷/擦除/蒙版限制、历史与 ORA v4、QML 静态轮廓。
- 最新画笔库 / 设置关联：`docs/brush-library-design.md` / `docs/brush-library-validation.md`；共享活动预设、真实异步引擎预览、直径/间距单位转换、后台保存；高级笔尖未实现。
- 当前任务/CPU 视口与异步 ABI：`docs/task-render-design.md` / `docs/task-render-validation.md`，Qt 不再使用 C++ 绘画队列或整张文档缓存。
- 最新绘画性能修复：`docs/painting-performance-design.md` / `docs/painting-performance-validation.md`；有界 Weak 瓦片缓存、稀疏输出、QML 图层通知与原生性能基准。
- 最新剪贴/并排蒙版/预览修复：`docs/clipping-design.md` / `docs/clipping-validation.md`；ABI 1.7，Alt 层间点击、动态连续同级剪贴、ORA v3、边缘 LOD 正确采样。
- 最新蒙版/拖放/默认背景：`docs/masks-design.md` / `docs/masks-validation.md`；ABI 1.6，图层/组蒙版、拖放排序/归组、深度、隐式白色“背景”、中文命名、ORA 原始树与兼容预览。
- 最新图层组/透明背景：`docs/groups-design.md` / `docs/groups-validation.md` / `docs/transparency-design.md`；ABI 1.5、隔离嵌套、锁继承、结构历史/子树删除、嵌套 ORA、QML 折叠/浮窗与棋盘格。蒙版已在 ABI 1.6 完成。
- 图层控制：`docs/layers-design.md` / `docs/layers-validation.md`；ABI 1.4、27 种混合、三种锁、位置/填充、可见行后台预览、ORA 扩展，fx 未实施。
- 最新暂存盘配置：`docs/storage-settings-design.md` / `docs/storage-settings-validation.md`；paint-storage、ABI 1.3、后台检查与租约清扫，设置重启生效。
- 最新文档分页：`docs/document-paging-design.md` / `docs/document-paging-validation.md`；256MiB 为像素驻留缓存，冷页磁盘后备，快照/历史保持不可变身份。
- 最新历史存储：`docs/history-storage-design.md` / `docs/history-storage-validation.md`；大笔触无损编码与磁盘暂存，不再按内存历史预算拒绝笔触。

约束：Rust 不依赖 Qt；UI 只经 PaintCoreClient 使用 C ABI；Python 不访问 Qt/UI 线程。
任何新模块先写设计与验收条件，再实现。禁止用空实现或假成功代替未支持的功能。
所有行为修改补对应层回归测试（Rust / Qt Test / pytest）；尚未存在的层不创建虚假测试。
仓库统一验证入口：`python tools/check.py`（默认含 Qt，--qt-prefix 可指定 SDK）；--core-only 仅表示核心部分验证，不允许跳过失败项后报告完整通过。
ABI 破坏性修改升主版本；生成头由 cbindgen 管理，不手工修改生成物。
只有 paint-ffi 允许经过审查的 unsafe；caller-memory 读写集中在 pointers.rs，回调注明安全约束。
不要把开发机绝对路径写入构建配置；不要提交构建输出或工具链。
