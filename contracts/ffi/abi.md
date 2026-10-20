# C ABI 1.70.0 契约

## ABI 1.70 — 自身描边独立纹理

完整描边及Groups准备/包入口支持state42：`strokeOutline.source`在`model:"self-brush"`时可含独立`texture`对象，参数、资源与关闭值复用现有纹理规则。只改变描边覆盖，不改主纹理；不含该字段时旧state41编码保持。state41携带此扩展拒绝。VBR1.0按字段补齐并重算behavior34及`outline-self-texture`能力，旧模式仍behavior33。既有bit55/入口及DTO布局保持，无新增GPU计算。

## ABI 1.69 — 成功笔触提交回执

`paint_session_committed_strokes(core,session,publication,after_sequence,sequences,capacity,required)` 只查询准确publication；旧票据返回BUSY，required初始清零。返回该publication中所有大于after_sequence的成功且产生像素变更的End命令序列，按提交顺序排列；取消、失败、无变更、Undo/Redo不新增回执。记录跨文档New/Open保留，最多4096项（32KiB序列数据，共享不可变队列）；调用方须及时读取，早于保留窗口的回执不保证存在。

容量以u64元素计，required为u32元素数；null/0查询非空列表返回BUFFER_TOO_SMALL，空列表可返回OK。容量不足不部分写入，足够时输出必须为可写、对齐、互不重叠的存储。返回前完成复制，不持有调用方指针，无DTO布局变化。Qt按End序列匹配起笔快照，记录最多八个最近笔刷参数；该偏好不进入VBR或核心state。

## ABI 1.67 — 多图层移动会话

`paint_session_begin_layer_moves(core,session,ids,count,deferred,out_sequence)` 在返回前复制 1–4096 个不重复、非零 ID，deferred 为0/1；其余操作复用 `paint_session_layer_move` 的 UPDATE/COMMIT/CANCEL。重叠子树归并、所有锁/坐标先验证，一次最终历史，取消恢复全部目标；多目标当前保留 CPU 显示回退，deferred 只对兼容单目标常驻生效。DTO 不变，旧入口保留。

## ABI 1.66 — 完整图像翻转

现有 `paint_session_transform` 增补 `FLIP_IMAGE_HORIZONTAL=9`、`FLIP_IMAGE_VERTICAL=10`；geometry 必须为 null，异步 sequence/barrier 与已有变换一致。只在文档空闲且无文件任务时执行，不进入编辑会话；全部图层（含隐藏/锁定/组蒙版与置入图片）、画布外像素及选择同步镜像，画布尺寸不变。单条 LayerEdit 历史、失败原子保持；CPU 整数取样不插值，无新增 GPU 算子。DTO 布局不变。

## ABI 1.65 — 图层/组改名

`paint_session_group` 接受 `PAINT_GROUP_RENAME_LAYER=4`，layer_id 必须为现有节点，parent_id=0，name/name_length 为借入 UTF-8；返回前完整复制。OK 表示入队，空白名称、超过 1024 字节或 NUL 异步拒绝，原名称和历史不变。支持像素层、组和蒙版，同名不产生历史；名称变化以图层属性历史保存，可撤销重做。DTO 布局和旧 kind 语义不变。

## ABI 1.64 — 自由变换常驻网格预览

`PAINT_FEATURE_TRANSFORM_RESIDENT=576460752303423488`；新增 `paint_session_begin_transform_layers_deferred`，参数/所有权与普通 begin layers 相同。兼容单层普通Normal栅格与置入图片请求常驻预览，复杂组合或选区保留像素路径；旧入口行为保持。OK表示入队，异步错误通过会话发布。

已经开始的变换（包括文件置入自动创建的会话）可用 `PAINT_TRANSFORM_ENABLE_RESIDENT=8` 请求接入，geometry须null；没有活动会话异步报错，已接入时幂等。旧begin与UPDATE/关闭语义不变。

共用 `PaintResidentPreviewInfo` 的 `focus` 定义为位标记：bit0仍为隔离原图，bit1为正向变换网格，只有新入口会发布bit1。正向网格顶点位置是曲面映射结果，UV来自不可变源；消费者先将原图网格渲染至视口纹理，再在完整画布上与静态上下层合成，不能沿用逆向场的单遍局部合成。DTO布局与LE顶点包不变，旧液化/移动标记不变。网格至多145×145，静态资产最长边2048。

更新只改变几何，保持相邻绝对输入合并与checkpoint关闭顺序；确认从原始源完整重采样并生成单历史（Warp与预览共用正向三角面覆盖和行序后面覆盖前面规则，非Warp保留逆矩阵），取消恢复基线，资源失败保留会话可修正。详见[设计](../../docs/tools/transform-resident-preview-design.md)。

## ABI 1.63 — 图层移动会话与共用常驻预览

`PAINT_FEATURE_LAYER_MOVE_PREVIEW=288230376151711744`，`PAINT_SESSION_LAYER_MOVE=32`。`paint_session_layer_move(core,session,kind,layer_id,x,y,out_sequence)`：0/1 为普通/请求常驻预览的 BEGIN（id 指定图层，x/y=0）；2/3 为 PICK（x/y 为非负文档像素坐标，命中后捕获目标）；4 为相对会话起点的绝对整数位移 UPDATE；5 为携带最终位移的 COMMIT；6 为 CANCEL（x/y=0）。x/y 绝对值不得超过 MAX_DIMENSION，非法参数清零 sequence。除 BEGIN 外 layer_id 忽略。OK 表示排队；BEGIN/PICK 完成后检查会话标记，空白 PICK 不创建会话，锁定目标异步报错。

UPDATE 不写历史，相邻未执行更新可合并；COMMIT/CANCEL 使用关闭屏障，不能丢失最终位置。提交复用原有移动历史命令，取消恢复基线；其他编辑与文件操作在会话内拒绝。组/蒙版保持共同移动，置入图片保留原始模型。CPU 回退照常生成像素帧；兼容普通栅格栈使用四顶点和静态源纹理，视口0/1暂停重采样。

`PaintResidentPreviewInfo` 是 `PaintLiquifyPreviewInfo` 的布局兼容别名。`paint_session_resident_preview_info` / `paint_session_read_resident_preview` 为液化和移动共用的查询，票据、容量、像素格式和预算沿用下述 ABI 1.62 协议；旧液化命名入口保留为兼容别名。会话之间的 source_id 不复用；旧 revision 网格读返回 BUSY。详见[移动设计](../../docs/tools/layer-move-preview-design.md)。

## ABI 1.62 — 液化网格预览

`PAINT_FEATURE_LIQUIFY_PREVIEW`（bit57）提供 `PAINT_LIQUIFY_BEGIN_DEFERRED`（11）。兼容的无选区、顶层正常混合栅格栈使用网格预览；不支持的组合继续普通像素预览。旧 BEGIN 和动作语义保持。液化半径范围扩展至 7500px（直径 15000px），非法输入依旧拒绝。

`paint_session_liquify_preview_info` 返回带 generation/revision/source_id 的不可变网格信息；普通像素路径或会话结束返回 NOT_FOUND。`paint_session_read_liquify_preview` 的 kind0 返回 columns×rows 个小端 f32 `(x,y,u,v)`（目标文档坐标、源纹理 UV），必须匹配全部票据。kind1 返回六个小端 u32（原图、下层、上层各自 width/height），随后为三张紧密 RGBA8 sRGB 预乘图片；静态图片只匹配 generation/source_id，不因同源网格新修订而失效。纹理最长边 2048px，各自最多 4Mi 像素；网格最多 257×257 节点。容量探测写 required，不足时不部分写；kind1 首次读取可执行静态代理渲染，必须在后台线程调用，后续同源复用缓存。

原图包含保留的画布外像素。GPU 显示资源由 UI 渲染线程持有，不通过 ABI 传递图形指针。直接预览时视口0/1不生成 CPU 帧，调用方使用网格显示；完成/取消后恢复普通帧。CPU 数值执行器也能独立运行 deferred 核心，但软件 UI 应使用普通 BEGIN。见[设计](../../docs/tools/liquify-resident-preview-design.md)。

## ABI 1.61 — 可选 GPU 计算算子

`PAINT_FEATURE_COMPUTE_OPERATORS`（bit56）表示共享数值算子接口，不表示 GPU 画布合成或设备必定可用。新增 `paint_compute_hardware` 返回无 NUL 的 UTF-8 JSON 数组（`id/name/backend`）；始终包含 `cpu`，GPU 构建另有 `auto` 和真实设备。按已有容量探测契约返回 required，不足时不写部分内容。枚举应在调用方后台线程执行。

`paint_session_set_compute` 复制 1–1024 字节 UTF-8 设备 ID 并排队，返回 OK 仅代表入队。活动笔触、液化、变换或文件作业拒绝切换；新建和载入文档保持会话后端。未知、失效或执行失败的硬件以同一输入回退 CPU，`paint_session_compute_status` 返回实际后端和回退原因。CPU-only 构建只枚举 CPU；现有 DTO、VBR/state 与图层像素格式不变。见[设计](../../docs/tools/liquify-gpu-operators-design.md)。

## ABI 1.60 — 整条笔触描边入口

`PAINT_FEATURE_STROKE_OUTLINE`（bit55）新增`paint_brush_library_prepare_state_outline`与`paint_brush_library_package_read_outline` / `paint_brush_library_package_write_outline`。准备入口接受完整state41及所有既有完整参数；包入口读写VBR 1.0/behavior33及`stroke-outline`能力，包含自身、已有笔刷和纹理来源，并按内容身份捕获描边已有笔刷/纹理资源。关闭但非默认的对象仍完整往返。

普通、wet、taper和wet-dual准备/包入口继续对描边返回Unsupported；Groups完整入口同样支持描边。容量探测不安装资源，读取先安装完整束再返回JSON。无DTO布局变化；Qt镜像/控件和真实预览仍待后续。见`docs/brush/core/brush-stroke-outline-abi-design.md`。

## ABI 1.59 — 曲线节点采样

新增 `PAINT_FEATURE_CURVE_NODE_MODES`（bit54），已有 `paint_pressure_curve_samples` 的model2接受严格对象 `{curve:[[x,y],...],nodeModes:[bool,...]}`，使用与实际执行相同的节点混合插值，固定输出257个采样。JSON上限4096字节、2–17个节点、布尔模式与节点等长；错误清零有效输出。model0/1的数组输入和计算保持不变。无结构布局变化；格式说明见[当前VBR契约](../brush/vbr/format.md)。

## ABI 1.58 — 覆盖资源清单

`PAINT_FEATURE_BRUSH_RESOURCE_LIST`（bit53）新增后台入口`paint_brush_library_resources_json`，返回UTF-8无NUL的`{"model":"coverage-resources-v1","resources":["64位小写十六进制身份",...]}`。按身份升序，最多4096项，包含库中未引用PNG；只枚举普通文件，不解码、不改变预设或修订。实际使用仍通过原资源哈希/PNG校验。调用方后台查询并按需生成缩略图，不能把枚举成功当作内容有效。

容量不足返回BUFFER_TOO_SMALL及required且不复制；其他错误清零required。资源未绑定返回NOT_FOUND。目录可在两次查询间变化，调用方应重试容量不足；不提供跨查询快照票据。现有DTO布局和state/VBR不变。

当前版本与能力以 `core/crates/paint-ffi/src/abi/api.rs` 和 `ui/include/paint_api.h` 为准。本文所有笔刷文件均指 VBR 1.0，读取按实际字段补全、校验和执行，不以 minor 判断功能。下文按 ABI 发布阶段说明接口边界；ABI、state、behavior 和来源版本独立于文件版本，见 [当前格式契约](../brush/vbr/format.md)。

## ABI 1.57 — 时间喷枪

`PAINT_FEATURE_AIRBRUSH`（bit52）公布预设级 `time-interval-v1` 时间喷枪，state23/VBR 1.0/behavior18。新增 `PAINT_COMMAND_AIRBRUSH_TICK`（kind14），复用既有 `PaintCommand.point` 与 `paint_session_submit`，不改变 DTO 或导出签名。调用方必须已在同一活动笔触上启用 airbrush，并提供与上次点相同的位置、单调递增时间戳和当前压力；Rust 只在该时间已跨过 `intervalMs` 时沉积一次，未到间隔为无操作。

喷枪 tick 与普通输入共用 Document 事务、随机序号、动态、选区、图层锁、蒙版和单笔历史；低不透明度设置按当前文档像素逐 tick 累积，结束仍只有一条历史。只支持干画笔和橡皮；普通涂抹、湿混及其他不支持的组合返回明确错误。Qt 在真实绘画期间按预设间隔提交 tick，结束/取消/丢笔/关闭立即停止。自动预览仍固定白色及正常工具模式。

## ABI 1.56 — 逐印章双色模型

`PAINT_FEATURE_DUAL_COLOR_INTERLEAVED`（bit51）公布 `interleaved-stamp-v1` 双色模型。它复用既有 `PaintBrushDualDesc` 与 `dualBrush.color` JSON 对象，不改变 DTO 或导出签名；新增能力仅表示 state22/VBR 1.0/behavior17 的逐印章渲染可由当前核心执行。

逐印章模型把每个主印章与对应第二印章先按配置的前后顺序合成，再按印章顺序叠加到隔离笔触场，最后统一应用整笔不透明度。因此后一个印章的第二贡献可以覆盖先前印章的主贡献，区别于 `separated-stroke-v1` 的整笔两份场最终合成。模型要求配对第二笔尖（`paired-dab-v1`）；独立第二路径明确拒绝。橡皮和未开启手指绘画的涂抹忽略颜色模块并按原工具执行，手指绘画涂抹在第一笔沉积阶段使用双色；湿混下的双色涂抹/橡皮不处理。选区、图层蒙版、透明锁、偏移、文档纸纹、颜色动态、相对尺寸、图像笔尖及撤销重做仍走既有 Document 事务。

颜色模型为 `interleaved-stamp-v1` 时，持久状态要求 state22；旧 state21 不得携带该模型。VBR 1.0 根据实际字段执行该模型，不设 minor 门槛。VBR 能力名称为 `dual-color-stamp`，behavior17。旧 `PAINT_FEATURE_DUAL_COLOR_STROKE` 继续覆盖 `separated-stroke-v1` 和既有双色入口；Qt 必须在同一预设中同时保留旧能力与新模式能力。自动预览继续固定白色/正常工具模式，模型切换只改变真实笔触的贡献顺序。

## ABI 1.48 双色双笔刷入口

PAINT_FEATURE_DUAL_COLOR_STROKE=8796093022208，既有准确准备与库包接通state16/VBR 1.0/behavior13，解除色彩对象临时拒绝。无DTO/导出签名改变；Qt最低ABI1.48，完整color镜像/基准/关闭值和背景来源、27模式保持，预览捕获实际工具两色但不保存到预设。工具预览仍正常，真实起笔两色/模式与定义不可变捕获。旧程序包不支持扩展，双色橡皮和涂抹仍明确拒绝。见docs/brush/ui/brush-dual-color-ui-design.md。


## ABI 1.47 第二笔尖相对尺寸入口

PAINT_FEATURE_DUAL_RELATIVE_SIZE=4398046511104，既有准确准备与库包接通state15/VBR 1.0/behavior12，解除新尺寸对象临时拒绝。无DTO/导出签名改变，Qt最低ABI1.47；双笔刷镜像、关闭值、重置基准、预览与VBR完整携带sizing。真正尺寸与压力几何由Rust执行，Qt只显示标称夹限；旧无库程序包继续拒绝扩展。见docs/brush/ui/brush-dual-size-ui-design.md。


## ABI 1.46 时间平滑设置与完整桥

PAINT_FEATURE_PATH_SMOOTHING=2199023255552，既有准确准备和库包支持state14/VBR 1.0/behavior11完整time-grid-v1。无DTO/导出签名变化。Qt至少要求ABI1.46，完整镜像/基准/关闭配置及后台规范化持久化；主/第二复用同一路径，预览使用真实时间输入，真实笔触仍按捕获修订执行。上阶段新配置暂拒已解除；无库程序子集入口仍明确Unsupported。PAINT_FEATURE_PRESSURE_SMOOTH继续仅为PCHIP插值能力，两项不混淆。见docs/validations/brush/ui/brush-smoothing-ui-validation.md。

## ABI 1.45 自动平滑压力曲线

PAINT_FEATURE_PRESSURE_SMOOTH=1099511627776。paint_pressure_curve_samples(json,length,model,out)为允许UI线程调用的无句柄纯查询，不等待actor/资源；JSON为2–17个[x,y]点，1..4096字节，model=0旧折线/1 smooth-pchip-v1，out为对齐可写257个f32。所有缓冲区不重叠，不保留caller内存；有效输出在失败时清零，NULL/错位/非法数量/模型/点拒绝。实际执行精确求值，UI只画采样。state13/VBR 1.0/behavior10/pressure-smooth能力保存插值模型，缺省旧折线保留，编辑显式转平滑；主/第二关闭数据及完整基准均往返。见docs/brush/dynamics/brush-pressure-smooth-design.md。

## ABI 1.44 画笔颜色混合捕获

PAINT_FEATURE_BRUSH_WRITE_BLEND=549755813888（u64）。PaintBrushBlendDesc为16字节，size/u32模式位于0/4，reserved/u64位于8；模式0..26，reserved=0。新paint_session_begin_prepared_preset_with_blend读取现有tool/background/point及此描述，经pointers.rs复制并验证，失败sequence=0；非正常擦除/涂抹和组穿透拒绝。actor捕获模式与准确准备票据，不借用caller内存；旧入口保持正常，无旧DTO或签名变化，state12/VBR 1.0不保存临时工具模式。Qt工具栏/持久化已接。最新用户要求预览固定正常，菜单跨原生面板置顶，见docs/validations/brush/wet/brush-mix-performance-validation.md；早期预览证据保留在docs/validations/brush/ui/brush-blend-ui-validation.md。

## ABI 1.43 双笔刷设置与资源入口

`PAINT_FEATURE_DUAL_BRUSH=274877906944`（u64），无DTO或导出签名变化。准确准备/state12/VBR 1.0/behavior9接Qt完整双对象，PNG导入捕获预设与第二角色；新关闭默认独立路径24px/25%，显式配对保留null间距。库package解除临时拒绝，当前主/纹理/第二与来源基线PNG去重并集导出；缺失资源不改已有目标。旧无库上下文程序导出仍明确拒绝双对象。Qt至少要求ABI1.43，真实预览保持两笔尖尺寸比例，光标仍显示主笔尖。见 `docs/validations/brush/ui/brush-dual-ui-validation.md`。

历史双笔刷IO/准备桥阶段ABI1.42曾保留库包临时拒绝，现已被上述入口取代，原阶段证据保留。

## ABI 1.42 纹理资源与设置入口

`PAINT_FEATURE_BRUSH_TEXTURE=137438953472`（u64）。现有准确准备与库包读写支持 state11/VBR 1.0/behavior8 的预设级 texture 对象，PNG 导入沿既有覆盖资源接口。无 DTO 布局变化。库包解除临时拒绝，导出覆盖当前笔尖、纹理与来源基线 PNG 并集；无库上下文的旧程序包接口继续明确拒绝纹理。Qt 严格保存完整模型/资源/关闭值，真实预览和绘画仅由 Rust 执行，导入捕获预设与目标角色。见 `docs/brush/ui/brush-texture-ui-design.md`。

历史纹理IO阶段：Rust state11/VBR 1.0/behavior8支持预设级texture资源绑定，当时ABI1.41、Qt未接。临时包拒绝已由ABI1.42入口取代，阶段证据见 `docs/validations/brush/texture/brush-texture-io-validation.md`。

## ABI 1.41 颜色动态与背景捕获

`PAINT_FEATURE_COLOR_DYNAMICS=68719476736`（u64）。新增 `PaintBrushBackgroundDesc`（24字节：size/reserved/4线性RGBA，RGBA偏移8）及 `paint_session_begin_prepared_preset_with_background`。旧ToolDesc/导出布局不变，旧入口白色背景。新入口与前景tool同时复制背景为拥有的Pixel，size/reserved/RGBA/point校验、失败sequence=0，actor等准确准备后捕获完整预设，两色不借用caller内存；所有caller读取仍在pointers.rs。state10/VBR 1.0/behavior7/model接Qt镜像，库package解除临时拒绝；旧程序子集接口保持原范围。见 `docs/validations/brush/ui/brush-color-ui-validation.md`。

颜色动态IO阶段：Rust state10/VBR 1.0/behavior7保存独立colorDynamics及完整hex种子，模型严格linear-hsv-v1；ABI仍1.40，用户背景工具传入与Qt待接。库package_read/write暂对新对象返回Unsupported，读required=0且不改caller/资源，写不改目标；旧包保持原路径。见 `docs/validations/brush/core/brush-color-io-validation.md`。

## ABI 1.40 形状抖动入口

`PAINT_FEATURE_SHAPE_JITTER=34359738368`，features为u64。state9/VBR 1.0/behavior6/shape-jitter接入准确准备与Qt镜像，没有DTO变化。库package_read/write解除形状对象临时Unsupported边界；旧程序子集入口仍只支持原子集。UI要求至少ABI1.40。每预设保留独立对象和完整seed字符串，关闭不清配置，真实预览/绘画仅Rust执行；光标显示输入笔尖而非随机样本。见 `docs/validations/brush/ui/brush-shape-jitter-ui-validation.md`。

形状抖动IO阶段：Rust state9/VBR 1.0/behavior6保存独立shapeJitter对象及16位十六进制u64种子；ABI仍1.39，Qt尚未接入。package_read/write暂对新对象返回Unsupported，读required=0且不改caller字节或资源目录，写不改目标；默认和既有包保留原路径。见 `docs/validations/brush/dynamics/brush-shape-jitter-io-validation.md`。

## ABI 1.39 散布入口

`PAINT_FEATURE_BRUSH_SCATTER=17179869184`（features为u64）。state8/VBR 1.0/behavior5/scatter-stamps使用既有准确准备与捕获笔触，没有DTO布局变化。package_read/write解除ABI1.38对散布对象的临时Unsupported边界；Qt独立保存对象及完整种子字符串。散布运算仅在Rust，预览/实际笔触共用；关闭保留配置，旧目录默认不升级。见 `docs/validations/brush/ui/brush-scatter-ui-validation.md`。

散布IO阶段：Rust state8/VBR 1.0/behavior5保存独立scatterDynamics及16位十六进制u64种子；ABI1.38和Qt尚未接入。package_read/write暂对scatterDynamics对象返回Unsupported，读required=0且不改caller字节或资源目录，写不改现有目标；默认和既有包保留原路径。见 `docs/validations/brush/dynamics/brush-scatter-io-validation.md`。

## ABI1.38：实时笔尖姿态

`PAINT_FEATURE_BRUSH_POSE=8589934592`。新增独立PaintBrushPose句柄create/bind_prepared/read/clear/destroy，绑定准确准备记录、ID（1–64字节UTF8）和名义半径0.5–1000。句柄反复绑定/clear不增加token；解析就绪后只保留不可变所选预设，库销毁不使其失效。clear释放捕获，destroy清空caller句柄，陈旧句柄拒绝。

read接受既有sized PaintPoint、严格0/1的pressure_known/direction_known和有限方向向量，未知方向必须零向量，已知方向非零；输出24字节PaintTipPose（struct_size/reserved/radius/angle/roundness/reserved_tail，radius offset8/angle12/roundness16）。输出保留字段为零，Busy/错误清输出，成功半径可因零压力为零。使用原生尺寸/几何算法；未知压力返回名义半径，缺设备通道回退基础几何。读取只try_result，不等待，不做caller线程文件IO，冷准备在独立池排任务；生命周期与查询均禁止在回调内调用。caller内存通过pointers.rs，安全前提见生成头。

Qt绘画姿态绑定与BeginPrepared相同的记录/ID/半径，悬停独立，图像轮廓身份在本笔保留；此接口不修改文档/历史/预设或保存格式。见 `docs/validations/brush/input/brush-live-pose-validation.md`。

## ABI1.37：设备笔尖动态入口

`PAINT_FEATURE_DEVICE_TIP_DYNAMICS=4294967296`，features为u64。state7/VBR 1.0/behavior4/tip-dynamics接入准确原生准备和Qt镜像；library_package_read/write解除设备对象临时Unsupported边界。曲线和设备控制分别保存，旧C参数编辑保留完整动态；无DTO布局变化。Qt仅校验/展示，不执行几何。实际输入与预览沿同一内核，实时光标pose接口仍在后续范围，见 `docs/validations/brush/ui/brush-device-ui-validation.md`。

设备动态IO阶段：Rust state7/VBR 1.0已有独立tipDynamics保存，现有Qt仍只接受state6/behavior3。ABI36 package_read/write对tipDynamics对象暂返回Unsupported（读required=0且不改caller字节，写不改已有目标），不宣称旧Qt已支持新数据；新原生能力和Qt接入后解除边界。无DTO变化，来源/压力包保持原路径。见 `docs/validations/brush/core/brush-device-io-validation.md`。

## ABI 1.36：独立不透明度压感

`PAINT_FEATURE_OPACITY_PRESSURE=2147483648`（features为u64）。现有目录状态接受v6，package接口支持VBR 1.0/behavior3/opacity-pressure。pressureDynamics增加可选opacity对象，curve/minimum沿既有有界控制、enabled为严格布尔；v5不能携带此对象。普通绘画/擦除/蒙版起笔捕获局部压力上限，流量独立，较低压力不反向清除原颜色；涂抹继续强度语义。无DTO布局改变，旧C编辑保留动态，caller-memory边界不变。设计与验收见 `docs/brush/dynamics/brush-opacity-pressure-design.md` / `docs/validations/brush/dynamics/brush-opacity-pressure-validation.md`。

## ABI 1.35：压力动态入口与整笔不透明度修复

`PAINT_FEATURE_PRESSURE_DYNAMICS=1073741824`。现有 library package 读写支持 VBR 1.0 / behavior2 / pressure-curves，Qt 保存并编辑 v5 尺寸/流量曲线。没有 DTO 布局变化，生成头由 cbindgen 管理。

用户要求修正普通绘画/擦除/蒙版的 opacity 为整笔上限，tip.flow 与流量压力仍控制印章累积；旧字段位置不变，旧低不透明度笔触行为有意修正，历史像素保持原样。涂抹继续运输强度语义。每个预设拥有实际 radius。实现与边界见 `docs/brush/dynamics/brush-stroke-opacity-design.md`。

## ABI 1.34：VBR来源交换

`PAINT_FEATURE_VBR_SOURCES=536870912`。现有library_package_read/write支持VBR 1.0：sourceResource及完整源JSON、原ABR和导入时/当前PNG并集。来源字段为元数据，不执行未支持PS行为。读取全包验证及caller容量/指针通过后才持久资源，caller JSON保持来源引用；required失败保持0，BUFFER_TOO_SMALL给准确长度。当前mask与来源凭据待准确发布采用，只有报告引用的基线mip不长期持有。

写入按来源引用冷读取分组原数据/记录，再收集基线和当前PNG并编译全包，原子目标保存；任何缺失/损坏不能覆盖现有目标。调用者在JSON里提供sourceResource表示要求包含完整原始ABR，可能包含同包其他预设和资源；GUI必须让用户明确选择此范围。有无来源均写出 VBR 1.0。无ABI DTO布局变化。

## ABI 1.33：ABR整包导入

`PAINT_FEATURE_ABR_IMPORT=268435456`。后台 `paint_brush_library_abr_read` 返回ABR交接JSON，独立于VBR和catalog state：version1、format abr、presets数组；每项name/parameters/tipResource（可null）/sourceResource/partial。最多2MiB。6/10×1/2基础执行子集，逐项完整来源报告保留；旧参数/16位/不支持desc明确拒绝，不能把partial成功宣称完整PS效果。

先清零required，解析/编译与输出序列化通过后才检查caller容量；不足返回BUFFER_TOO_SMALL及准确长度，无资源写入。路径/指针/其他失败保持required=0，失败不写buffer。输出容量足够后持久已验证资源及来源，库保留小型资源/来源交接，当前目录不改变；caller追加UUID并经准确prepare发布。当前/在途独立持有，采用后释放交接。多文件IO错误可留下未引用身份文件，不能宣称文件级事务回滚。

## ABI 1.32：笔刷来源准备桥

`PAINT_FEATURE_BRUSH_SOURCES=134217728`。绑定的图片资源目录下使用 `sources` 来源命名空间；准备状态支持 v4/sourceResource。首次完整校验、共享仓库中的存活身份凭据复用，当前修订与已捕获任务独立持有；凭据不包含大原始字节，不进入绘画参数。冷恢复仍完整校验，失败保留上一合法目录。

`paint_brush_library_source_info` 是后台同步调用，输入64字符来源身份，返回原JSON记录（无NUL，最多2MiB），每次重新校验记录及原始ABR文件；不输出原始ABR。先清零required，容量不足返回BUFFER_TOO_SMALL及准确长度，其他失败不写buffer。外部文件损坏不会改变已捕获修订，但查看/冷恢复必须报错。Qt v4/报告与ABR入口后续接入。

## ABI 1.31：VBR 图片资源

`PAINT_FEATURE_VBR_IMAGES=67108864`。后台调用 `paint_brush_library_package_read/write` 支持 VBR 1.0 程序包及图片包；JSON仍最多2MiB，预设可含tipResource哈希引用。写入从库资源仓库读取完整PNG字节，校验并编译整个包后同目录原子保存。读取整包验证后再持久写资源，返回canonical小JSON，不直接修改库目录/文档。目录更新继续使用既有准备/原子发布。

read输出先清零，容量不足给required并且不写资源；空输出足够容量拒绝。库必须有资源目录才能导入图片包。完整包验证失败不写资源；后续资源写入失败可能留下已保存的未引用哈希文件，库目录保持不变。多个资源以导入交接持有到成功准备采用它们；同步目录和正在使用的笔触继续持有自己的资源。资源文件不覆盖，PNG身份不得因重编码改变。

旧不带库的package接口继续仅支持程序包，图片包明确UNSUPPORTED。格式见 [VBR 1.0 契约](../brush/vbr/format.md)；高级动态/分组/ABR未在此能力中声明。

## ABI 1.30：图片笔尖基础轮廓

`PAINT_FEATURE_BRUSH_OUTLINE=33554432`。`paint_brush_library_tip_outline` 是后台调用，输入64字节资源身份，返回无NUL的SVG path几何。坐标以标称直径归一化、中心(.5,.5)，保持图片比例；caller缩放/圆度/旋转只改变显示，路径不进入文档。输出先清零，容量不足给出required并返回BUFFER_TOO_SMALL；透明覆盖返回OK/required=0。

覆盖阈值固定0.5；线性插值等值线保留孔洞与多个区域，65536线段上限超过则LIMIT_EXCEEDED，不返回截断路径。资源共享仓库缓存最多8条/16MiB几何，不持有覆盖资源。首次提取不持有全局注册表锁；准备中Qt使用简化轮廓，失败明确说明。其余旧DTO和接口保持兼容。

## ABI 1.29：共享图片笔尖资源

`PAINT_FEATURE_BRUSH_RESOURCES=16777216`。后台调用 `paint_brush_library_bind_resources` 为库绑定一次持久目录；相同目录可重复，换目录返回 BUSY。`share_resources(target,source)` 让隔离预览共享同一缓存、驻留预算和不可变资源；源未绑定返回 NOT_FOUND，源库销毁不使目标失效。

`paint_brush_library_import_tip_file` 在调用线程执行有界 PNG 读取/转换，因此调用方必须使用后台线程。interpretation 为 0 灰度×alpha、1 alpha、2 反向亮度×alpha。输出是无 NUL 的64字节 SHA-256 ID，source_depth 为原输入8或16（16说明规范覆盖已量化为8位）。输出先清零；容量不足返回 BUFFER_TOO_SMALL/required=64，不进行文件写入。成功只持久保存资源，不修改预设或文档。

状态准备新增 v3 `maskResource` 引用，目录缺失/资源错误明确失败；程序状态仍输出v2。当前版本、正在使用的笔触和导入交接保留资源。最近64条准备记录中的闲置旧图片快照可释放并保留准确配方，查询时重新异步读取且验证哈希；已经捕获的笔触不受影响。所有旧 DTO 和接口布局保持不变。

## ABI 1.28：异步准备与准确笔刷修订

`PAINT_FEATURE_BRUSH_PREPARATION=8388608`。`paint_brush_library_prepare_state` 是小型异步提交，复制最多2MiB参数状态JSON并返回准备序号；解析/编译在独立2线程、32项队列的元数据池完成。v1/v2迁移整包验证，未知字段/非法参数/缺少内置/身份冲突拒绝。同步目录修改在准备未完成时返回 BUSY；任务失败不覆盖上一合法目录。

`paint_brush_library_prepared_state` 读取准确序号的 canonical v2状态，准备中 BUSY，失败保留错误码；`current_state` 读取上一合法状态。最近64条记录可查询，裁剪后 NOT_FOUND；文档已捕获的记录不受裁剪影响。状态排除全局/工具偏好，当前图片资源需要后续资源编解码器，禁止恢复时悄悄移除图片。

`paint_session_begin_prepared_preset` 捕获准确准备记录，等待只发生在 Rust 文档执行线程。Move/End/Cancel依原顺序排队，GUI不阻塞或丢起笔。编辑/销毁库或新提交不能改变该笔参数；准备失败明确报错，不替换默认笔刷。旧同步入口与DTO仍可用。

## ABI 1.27：独立原生笔刷库

`PAINT_FEATURE_BRUSH_CATALOG=4194304`。`PaintBrushLibrary` 是独立不透明句柄，不归属任何 core/document。create(0) 使用128MiB库资源预算，非零参数可降低预算；destroy 显式释放，旧地址不能复用。目录同步操作由后台线程调用；未向 GUI 引入图片编译或 IO。

目录信息32字节，预设信息64字节；参数描述64字节，offset24为40字节笔尖；工具描述56字节，offset24为RGBA。读取预设/名称/ID要求非零准确 revision，过期 BUSY 并清空输出；名称/ID为UTF-8 caller-buffer，字段0/1。选择、参数编辑、改名、副本、删除先验证后提交；内置删除拒绝，编辑参数保留现有图片资源。首期目录1024预设，库预算不等同整个进程总内存预算。

`paint_session_begin_preset` 捕获指定预设的不可变定义，覆盖临时工具半径、颜色/操作和涂抹参数；原预设不被工具参数修改。复用原 Begin/Move/End/Cancel 队列与单条历史。库在提交后编辑/删除/销毁不会改动已捕获的笔触。旧入口与旧DTO不变。

## ABI 1.26：VBR 程序笔刷 IO

`PAINT_FEATURE_VBR=2097152`。`paint_brush_package_read/write` 是 worker-thread 同步 IO，Qt 后台线程调用；不操作文档或历史，不保留 caller memory。UTF-8 路径最长32768字节，JSON最长2MiB。读取整个 ZIP 验证成功后输出 canonical package JSON；小缓冲返回 BUFFER_TOO_SMALL 并填写 required，大小查询和重读之间文件可以改变。UI 使用固定2MiB缓冲一次读，避免查询/读取竞态。写入验证 JSON 的版本/能力与所有参数，然后同目录临时文件原子替换。

格式对应 [VBR 1.0 契约](../brush/vbr/format.md) 与 [设计](../../docs/brush/vbr/vbr-procedural-design.md)。package JSON 传输体包含 packageId 和完整 brushes 数组；归档 manifest 与每个 preset 分开。暂不支持图片资源/分组/未知扩展，明确拒绝，此入口保持程序子集能力边界，完整资源和组树使用对应库接口。

## ABI 1.25：共用程序笔尖

`PAINT_FEATURE_BRUSH_TIP=1048576`。`paint_session_begin_brush` 在返回前复制并验证输入，复用普通笔触队列、选层恢复、事务与结束/取消；旧画笔及涂抹入口仍使用兼容圆笔。

`PaintBrushTip` 为40字节：硬度/圆度/角度/流量、大小/流量压感和零 reserved；百分比归一化，圆度最低0.01，角度±180°。`PaintBrushStrokeDesc` 为120字节，offset8为旧56字节笔触描述，offset64为笔尖；操作0绘画、1擦除、2涂抹。涂抹参数仍为强度/所有层/手指绘画/强度压感，强度压感独立于绘画流量压感。

## ABI 1.24：画布 Ctrl 选层与移动

`paint_session_pick_layer` 复制文档坐标及可选位移，异步命中最上方可见内容图层（考虑透明、蒙版、祖先可见性及剪贴基底）。空白保持选择；位置锁定允许选层但不移动。只选层不增加历史，非零位移复用原子移动历史。所有 caller-memory 写入仍集中在 pointers.rs。


## ABI 1.23：图片模型与自由变换

- `PAINT_FEATURE_TRANSFORM=524288`、`PAINT_SESSION_TRANSFORM=16`。文件任务 `PAINT_FILE_PLACE=3` 导入 PNG/JPEG/WebP 到当前文档并开启待确认变换；不是新建/打开文档，不改变原保存路径，取消不改正式历史或 modified。
- `PaintTransform` 为 288 字节：前 16 字节为 struct_size/warp/interpolation/reserved；offset16 为 row-major 4×4 文档坐标 double 控制点（32 个值），offset272/280 为水平/垂直斜切角。所有输入在返回前复制；warp 为0/1，interpolation 为0–5，reserved=0，有限坐标±2,000,000、斜切角绝对值小于90度。非网格模式要求凸且非退化四边形。
- `paint_session_transform` 支持 BEGIN=0/UPDATE=1/CHECKPOINT=2/CANCEL=3/COMMIT=4/UNDO=5/REDO=6/RASTERIZE=7；UPDATE 需要 descriptor，其余必须 null。绝对 UPDATE 仅替换相邻未处理预览尾部，不能跨越 checkpoint；结束操作有队列保留槽。控件操作使用严格渲染屏障。
- `paint_session_begin_transform_layers` 接收1–4096个非零唯一ID，复制后入队；组包含子树，父子选择去重。全部预览成功才替换目标，整集合一条正式历史。普通单栅格层有选区时变换选区内像素并移动覆盖掩码；组、多选及图片层整体变换。
- `paint_session_transform_info` 与 `paint_session_layer_is_image` 读取精确 publication；过期返回 BUSY。活动期间发布独立的临时历史，步数沿用历史设置；确认一次正式提交，取消原样恢复正式历史/图层/选区。
- 历史种类36=置入图片、37=自由变换、38=栅格化。图片保留原始RGBA32F与几何，显示瓦片为派生数据；栅格化显式移除原图并可撤销。未栅格化图片拒绝画笔/橡皮、油漆桶和液化像素写入。
- 私有历史编码DVH7、层剪贴板DVL2（继续读取DVL1），MIME不变。含图片源的ORA使用扩展v5与校验的`drawverse/layers.bin`；兼容PNG预览与原始浮点模型分离。缺失模型/版本不匹配明确拒绝；纯栅格文档继续v4。


## ABI 1.22：液化会话历史

- `paint_session_liquify` 增加 STROKE_BEGIN=7/STROKE_END=8/UNDO=9/REDO=10，descriptor 必须 null。开始/结束按完整笔触生成临时历史；空笔触不记录，RESET 可撤销。结束使用队列保留槽，避免输入拥塞丢失笔触边界。
- 液化期间 session undo_depth/redo_depth 与历史条目发布会话状态；文档正式历史保留。取消恢复正式 undo/redo，确认只追加一次正式液化。步数沿用文档历史设置，位移差异使用校验暂存 payload。
- 历史种类 31=推拉/32=膨大/33=扭曲/34=重建/35=重置。既有结构布局与函数签名不变；旧客户端的 DAB/COMMIT 仍可使用。
- UNDO/REDO 使用严格渲染屏障；笔触边界无像素变化时不阻断有效预览。

## ABI 1.21：画布液化事务

- `PAINT_FEATURE_LIQUIFY=262144`，`PAINT_SESSION_LIQUIFY=8`，历史种类 30=液化。
- `paint_session_liquify` FIFO 接收 BEGIN=0/DAB=1/RESET=2/CANCEL=3/COMMIT=4/FOCUS_ON=5/FOCUS_OFF=6；只有 DAB 接收 `PaintLiquifyDab`，其余 descriptor 必须 null。所有指针在返回前复制，OK 只表示入队。
- `PaintLiquifyDab` 为 176 字节，from/to 位于 48/112；保留既有 DTO 不变。
- DAB：工具 0推拉/1膨大/2顺时针扭曲/3重建；半径0.5–1000px，强度/密度/速度0.01–1，时间0–250ms，压感0/1，保留字段为0；两个内嵌 PaintPoint 必须完整且合法。
- BEGIN 拒绝组、继承锁、普通笔触或活动文件任务；预览不改变 modified 或正式文档 undo/redo。RESET/CANCEL 精确恢复原层；COMMIT 一次真实像素历史，空确认无历史，历史失败仍可取消。活动期间普通修改返回 BUSY，文件任务以 BUSY 结束；读/渲染继续可用。资源与事务说明见 `docs/tools/liquify-design.md`。

- FOCUS_ON/OFF 仅修改显示快照，真实层可见性和历史不变；确认/取消后恢复全部原可见层。

## ABI 1.20：图层复制、剪贴板与合并

- `PAINT_FEATURE_LAYER_ACTIONS=131072`。`paint_session_layer_action` FIFO 接收 COPY/PASTE/DUPLICATE/MERGE；COPY 不改 revision/history，其他操作一次原子提交，失败不改变结构、像素或 ID 分配。
- COPY/DUPLICATE/MERGE：1–4096 个非零唯一 ID，target=0，bytes=null，length=0。父子重叠选择按独立子树去重；复制包含蒙版和属性。
- PASTE：ids=null、count=0、1–256MiB 字节；复制输入后返回，后台校验版本、校验和、层数、像素、层次与资源限制，放在显式目标（蒙版取其所有者）上方的同一父组。target=0 仅允许空栈的根插入；非空栈的缺失目标仍由 actor 拒绝。
- COPY 完成后用 `paint_session_read_layer_clipboard(sequence)` 查询最新成功复制的结果；null/0 查询返回 BUFFER_TOO_SMALL 与长度，成功读取无终止字符。结果留存到下一次成功复制或 session 销毁。Qt 工作线程读取并发布 `application/x-drawverse-layers-v1` MIME，支持跨文档。
- 历史种类 27=复制、28=粘贴、29=合并。单选普通图层向下合并同级下一层；单选组合并自身；多选合并所有所选子树。内核栅格化内部效果，产出普通图层，穿透组不再对外穿透。

## ABI 1.19：组穿透

新增 PAINT_BLEND_PASS_THROUGH=27 与 feature bit 65536。既有 PaintLayerAppearance.blend_mode 及 set/preview 路径支持此值，但只允许组节点；像素层与蒙版拒绝。请求同步校验数值范围，目标类型/锁和层 ID 仍由 actor 异步验证；UI 同步限制组专用选项。组影响范围、opacity/fill/蒙版与剪贴语义见 docs/layers/group-pass-through-design.md。旧混合 0–26、所有 DTO 和函数签名不变。


## ABI 1.18：涂抹

新增 88 字节 `PaintSmudgeStrokeDesc`：struct_size/all_layers/finger_painting/pressure_strength 为 u32，strength 为 f32（有限 0–1），reserved[3] 必须零；offset 32 为既有 56 字节 PaintStrokeDesc（独立 struct_size 校验，mode 必须 PAINT_MODE_PAINT）。布尔只接受 0/1，笔刷 opacity 与 strength 相乘；spacing 最低 0.02，对应直径的 1%。

`paint_session_begin_smudge(core,session,request,point,out_sequence)` 调用内复制所有参数及点，入队前失败 sequence 清零；沿用 stroke_to/end/cancel 和 actor 原子历史。feature bit 32768，历史 SMUDGE=26。旧结构、模式和保留位不变；蒙版禁止 all_layers，图层和选区限制沿用画笔。指针须对齐、可读/可写且输出不重叠输入。

## ABI 1.17：魔棒

新增 48 字节 PaintWandSelection：struct_size/x/y/sample_size/tolerance/antialias/contiguous/all_layers/operation 均为 u32，reserved[3] 为零；尺寸只接受 1/3/5/11/31/51/101，容差 0–255，布尔 0/1，operation 0–3（新建/添加/减去/交叉）。paint_session_select_wand 在调用内复制并验证请求，返回队列 sequence（入队前失败归零），actor 原子生成像素选区；坐标越界和资源失败通过既有异步错误发布。新增 feature bit 16384；历史使用已保留的 MAGIC_SELECTION=22。

paint_session_selection_mask(core,session,publication,index,bytes,capacity,out_length) 复制指定步的 row-major u8 覆盖（0–255）；掩码尺寸来自同 publication 的 selection_step。NULL/0 查询返回 BUFFER_TOO_SMALL 和 required length，容量不足不写 bytes；publication 变化返回 BUSY 并清零 length。缓冲和长度指针须可写且不重叠；非掩码步 INVALID_ARGUMENT、无效步 NOT_FOUND。最多 16MiB，无内部借用指针。旧二值选区 mask RLE 仍读取，灰度 mask 用 g 前缀；新核心保留原覆盖，旧核心明确拒绝未知灰度编码。新魔棒限制总编码为现有 1MiB 可持久化预算。

## ABI 1.14 多选新建组

新增 `paint_session_group_layers(core, session, ids, count, name, name_length, out_sequence)`。ids 为 1–4096 个按 u64 对齐且可读的唯一非零 ID；name 为可读 UTF-8 字节，name_length 为 u64。输入在调用内复制，out_sequence 为非重叠可写 u64，入队前失败清零。名称的核心限制为 1–1024 字节、无 NUL；核心校验失败异步报告，不修改结构或历史。

归一化后只移动独立选中子树，蒙版随所有者，禁止单独移动蒙版。所有选中根及后代先验证锁定。新组放在最近公共父组内、最高选中分支之后；选中根按原堆叠顺序归入组，未选节点保持父级和相对顺序。内部剪贴链保留，失去外部基底的剪贴释放。一次结构历史提交，不复制像素；任何失败保留结构、修订、历史和后续 ID。单层 `paint_session_group` 兼容保留，已有 DTO 与 ABI 符号不变。

## ABI 1.13 图层批量操作

新增 `paint_session_drop_layers(core, session, ids, count, target, placement, out_sequence)` 与 `paint_session_remove_layers(core, session, ids, count, mode, out_sequence)`。ids 是按 u64 对齐且可读的 count 个层 ID；count 为 u32，限定 1–4096，禁止零 ID / 重复 ID。在调用返回前复制全部 ID，不保留调用者指针；out_sequence 是非重叠可写 u64，入队前失败清零。

Drop placement 沿用 0=组内顶端（target=0 时为根顶端）、1=目标同级上方、2=目标同级下方。Remove mode 为 `PAINT_REMOVE_SUBTREES=0`（完整子树）或 `PAINT_REMOVE_KEEP_GROUP_CHILDREN=1`（仅移除选中组容器与组蒙版，保留内部图层；选中的普通图层仍删除）。未知 placement/mode、非法数组同步拒绝；不存在 ID、目标循环、锁定及深度限制在 actor 中报告，任何失败都不改变文档/历史。允许删除最后图层，空栈的 active_layer=0；合成为透明，撤销恢复。空栈可新增图层，Paste 的 target=0 仅在空栈表示根插入，非空栈仍需实际目标。

核心按原堆叠顺序归一化独立根，选择祖先与后代时后代不重复操作，蒙版跟随所有者。整批一次校验、一次提交、一次撤销/重做。移动到同级剪贴节点下方时，在存在下方基底的情况下自动加入上方剪贴链；旧位置丢失原基底的链仍释放剪贴。此规则亦用于既有单层 drop 接口。删除使用私有有界无损暂存，保留移除像素和前后结构；既有 DTO 与 ABI 符号保持兼容。

## ABI 1.12 视口吸色

`paint_session_sample_pixel(core, session, view, x, y, out_rgba)` 读取已完成合成视口的显示颜色；x/y 是文档整数像素坐标，按像素中心通过实际浮点视口区域换算到输出图像。支持不同缩放、LOD、设备 DPI 和分数原点，区域外坐标夹到输出边缘。输出为四个 f32，sRGB 非预乘 RGBA；透明像素返回全零。

视口槽为 0–3；未完成/禁用或过期世代/修订返回 BUSY，非法槽返回 INVALID_ARGUMENT。`out_rgba` 必须非空、按 f32 对齐且可写四个浮点数；失败不写像素。只读操作不进入历史，不增加修订、不标记文档修改。读取的是显示帧，缩小视图时包含渲染器的 LOD 结果，不是独立的原分辨率像素查询。Qt 主画布和独立吸色工具均经 PaintCoreClient 使用此接口。

本版本仍保留部分已退役功能的 DTO/常量以避免生成头漂移；魔棒和旋转相关功能没有可调用实现，UI 标为待支持。历史编号 22 不产生；自动释放剪贴为 `PAINT_HISTORY_RELEASE_CLIPPING=23`。

## ABI 1.11 历史条数上限

`paint_session_set_history_limit(core, session, max_commands, out_sequence)` 以 FIFO 命令修改最多保留的历史操作数；零上限为 INVALID_ARGUMENT，失败 sequence 为零。执行时裁剪最旧记录，裁剪边界查询为 TRUNCATED，不能伪装成可返回的初始状态；设置自身不添加历史操作。Qt 默认 100，偏好允许 10–1000 并持久化。

## ABI 1.10 历史操作元数据

新增 16 字节 PaintHistoryEntry（u32 struct_size/kind/depth/reserved，depth 偏移 8）与 paint_session_history_entry(core,session,expected_publication,index,out_entry)。索引 0 到 undo_depth+redo_depth 表示保留历史的时间顺序：0 是 INITIAL 或 TRUNCATED 边界，1 起为真实操作。kind 使用生成头中的 PAINT_HISTORY_* 常量（0–20）。画笔、橡皮、全选、矩形/椭圆选区、取消/反选及图层操作来自已提交命令，不从深度推测。

元数据与 Inline/Encoded 历史命令一起移动，撤销/重做保留标签，分支丢弃重做标签，裁剪后深度 0 标记 TRUNCATED，客户端不再将它显示为初始状态。读取无需访问/解码磁盘像素，不改变历史。发布不匹配返回 BUSY，越界 NOT_FOUND，失败重置输出；世代改变由新的 publication 隔离。既有 DTO、接口和命令磁盘格式保持。

## ABI 1.9 混合模式显示预览

`paint_session_set_blend_preview(core,session,layer_id,blend_mode,viewport,out_request)` 只覆盖视口槽 0/1 的完整文档合成快照，使用既有 0–26 混合编号与 PaintViewport DTO。必须指定存在的非蒙版图层/组；零 ID、非法混合或非 0/1 槽返回 INVALID_ARGUMENT，不存在/蒙版 ID 返回 NOT_FOUND，旧 document_generation 返回 BUSY；失败清零 out_request。调用者内存只在本次调用读取/写入，不保留。

预览共享快照瓦片，只修改快照副本的外观。文档修订、图层元数据、历史、modified 与文件保存不受影响；返回递增的该槽 request_id，沿用异步帧读取、取消和旧帧拒绝。再次调用替换预览，普通 paint_session_set_viewport 清除覆盖，enabled=0 释放帧，新文档清空覆盖。新增函数向后兼容，既有 DTO/函数/能力位保留，Qt 预览客户端要求版本至少 1.9。

## ABI 1.5 图层组

新增能力 `PAINT_FEATURE_LAYER_GROUPS=512`；旧 DTO 大小、函数和混合编号保留。`PaintLayerHierarchy=24` 字节，parent_id 偏移8，depth/effective_locks 为固定 u32；kind Pixel=0 / Group=1。`paint_session_layer_hierarchy` 与 layer_info 使用同一 exact publication，不匹配返回 BUSY 并清空输出。

`PaintGroupRequest=40` 字节，name 偏移24；`paint_session_group` 同步复制请求与 UTF-8 名称后排入 FIFO，返回 out_sequence，只表示接受。Wrap=1 需 layer_id 和 name，parent_id=0；Ungroup=2 仅 layer_id；Reparent=3 需 layer_id、parent_id（0=根）。后两种名称指针必须 NULL、长度0；未知 kind/无关字段非法，不入队且 out_sequence=0。不存在/循环/锁/深度/历史错误由 actor publication 报告。

组枚举仍为底到顶后序：子树连续，组在子节点之后。最大16级父组、4096节点，至少一个像素图层。组不拥有瓦片，自身位置固定0，不接受透明像素锁；完全锁和位置锁传递给后代，effective_locks 包含祖先的这两个锁。组按隔离语义合成，组 opacity/fill/blend 在子树合成后应用一次；不支持穿透模式。

Wrap 包含指定图层/组及其子树；新组成为活动节点。Reparent 将子树放到目标组顶层，位置保持文档坐标；Ungroup 移除组属性、保留未烘焙子节点。既有 Remove 删除整组子树，允许空栈及只含组的结构。既有 Move 移动所有后代像素层，一个历史事务；任一后代受锁或越界则整个操作拒绝。既有 Add 在所选组内或所选像素层同级上方新增，空栈新增根层。组不能 Begin 绘画，需选子像素层；预览接口对组返回子树合成，忽略目标组自身隐藏/opacity/fill/blend，保留子节点属性。

结构命令只保留元数据，删除子树可使用有界无损磁盘历史。组操作遵守 idle、撤销屏障与 modified 规则，撤销/重做仍忽略当前锁状态。层 ID 单调且不复用；ORA 重开重新分配 IDs，不能跨代际复用旧 ID。

正式源：`core/crates/paint-ffi/src/abi/api.rs` 与导出函数 `src/lib.rs` / `src/engine/session_api.rs` / `src/files/file_api.rs` / `src/engine/storage_api.rs` / `src/document/layer_api.rs`（均相对 paint-ffi crate）；其余入口按职责位于 `src/brushes` / `src/document`。正式头：`ui/include/paint_api.h`，固定 cbindgen 0.29.2 自动生成。旧 draft.h 仅为设计历史。1.0 验证见 `docs/validations/platform/ffi-validation.md`，1.1 设计/验证见 `docs/platform/task-render-design.md` / `docs/validations/platform/task-render-validation.md`。

## 版本与类型

全部函数返回 int32_t，包括 version/create/destroy；version 写 out 参数，当前 major/minor/patch=1/57/0。应用版本独立。DTO 全部 repr(C)，只含固定宽度数字和指针；无 enum/bool/size_t/long、Qt/STL/Rust 容器。支持 64 位；32 位 Rust 构建拒绝。

初始化 DTO 为零并写 struct_size=sizeof(本调用者结构)，输入至少含已发布前缀。库只访问已知前缀；过小为 INVALID_ARGUMENT，不覆盖调用者内存；扩展尾部保持不变。输出 struct_size 写库已知大小。reserved 必须为零；未知 mode/tool/format 为 UNSUPPORTED。输入 capability 位保留未知位，圆笔刷只实际使用 pressure，客户端为无压力设备提供 pressure=1。

已发布字段布局、语义、所有权、线程契约不能更改。破坏性修改提升 ABI 主版本；兼容新增函数/能力/新结构提升次版本。现有 DTO 不重新排序或更换字段类型。客户端启动先 version 再 capabilities，按位检查所需功能并忽略未知能力位。完整能力位以生成头中的 PAINT_FEATURE_* 和运行时 capabilities 返回值为准。

## 错误

OK=0，INVALID_ARGUMENT=1，INVALID_HANDLE=2，BUSY=3，NOT_FOUND=4，LIMIT_EXCEEDED=5，UNSUPPORTED=6，CANCELLED=7，IO_ERROR=8，INTERNAL_ERROR=9，BUFFER_TOO_SMALL=10，LAYER_LOCKED=11。文件任务返回 IO_ERROR；异步会话产生 CANCELLED。未知非零按失败处理。原同步 API 的空 undo/redo 成功；无活动笔触的 end/cancel/to 为 INVALID_ARGUMENT；活动笔触期间图层操作/undo/redo 为 BUSY。

paint_error_message 读取当前线程最近错误，UTF-8 字节，无 NUL。NULL/0 查询，非空消息返回 BUFFER_TOO_SMALL；空消息 OK、required=0。该查询包括无效/小缓冲结果均不替换原错误；其他成功函数清空 TLS。回调重入使用同一 TLS，但最外层 API 返回前保存其最终结果。

可展开 Rust panic 在 ABI 边界 catch_unwind 转 INTERNAL_ERROR。文档锁被 poison 后拒绝读写，仍可销毁；不盲目恢复潜在不一致状态。OOM/abort、野指针、非法回调和外部异常不能依靠 catch_unwind 修复。C++ 包装捕获本地异常，回调必须 noexcept/自行捕获，不能跨 C ABI 抛异常。

## 句柄生命周期

core_create/new_document/subscribe 写出拥有句柄，失败写 NULL。调用者先释放旧拥有句柄，再复用创建输出存储。文档和订阅归属指定 core；跨 core、错误类型、旧或未登记句柄为 INVALID_HANDLE。句柄不得解引用、复制为第二个拥有者或跨动态库实例传递。

core 有存活 document/subscription/session 时销毁为 BUSY，保留句柄。销毁接受 handle**，成功置 NULL；已有 NULL 幂等释放，document/subscription/session 仍要求有效 core。handle** 自身必须有效、可写、非 NULL。销毁文档丢弃未提交笔触，像素/历史正常释放。

大笔触历史可用内部无损编码/系统临时文件，history_bytes 仅报告保留的内存历史计账，不包含磁盘字节或解码工作区。内存预算不是单笔大小限制；存储失败可能返回 IO_ERROR，失败的 undo/redo 不修改像素与历史栈。实现/限额见 docs/document/history-storage-design.md；C ABI 布局与版本未变。

注册表从不解引用调用者句柄。每次创建分配唯一地址 token，释放后保留小型 token 至进程结束阻止 ABA；进程累计最多 65,536 个句柄（core/document/subscription/session），达到上限 LIMIT_EXCEEDED。关闭再创建不能绕过；文档内存与 token 分开释放。帧采用固定槽与 ID，不分配句柄。

## 内存与输出

所有参数内存须有效、正确对齐、完整初始化，保证读/写权限及声明长度。输入/输出/缓冲/user_data 存储互不重叠。库可检查 NULL、显然不对齐、句柄登记与数值长度，不能检测野指针、虚假容量或悬空回调数据。

层名称输入为 UTF-8 指针+uint64_t 长度，1..=1024 字节，禁 NUL；输入在同步返回前复制。名称/错误输出为调用者缓冲+capacity+required，无 NUL。容量不足不部分写入。错误时可写 metadata/ID 初始化为空（ID=0）；像素输出保留输入 data/capacity/stride/format，只清空 required/width/height/revision。过小 DTO 不写完整结构。

## 像素读取

paint_core_read_tile 为同步参考采样，仅后台客户端调用；只支持 PAINT_TILE_RGBA32F_LINEAR_PREMULTIPLIED（2），不是显示编码或完整帧 Renderer。输出普通混合的线性预乘 RGBA32F，原生字节序，每像素四个 IEEE 754 f32（16 字节）。字节缓冲无需 f32 对齐，解释时用 memcpy 或适当对齐。

ROI 在文档内，宽高各 1..=256；不隐式裁剪，不接受空 ROI。stride 以字节计，至少 w*16。required=(h-1)*stride+w*16；乘加/长度/ROI 全部在写前核验。NULL data/capacity=0 查询返回 BUFFER_TOO_SMALL 与 required，width/height/revision 保持零。失败不写像素，行 padding 保持不变，最后行不要求尾部 padding。全部复制成功后发布尺寸和快照 revision。

## 线程与事件

原同步 API 由客户端工作线程调用，UI 不直接执行绘制/读取/销毁等待。每文档 Mutex 串行访问，不同文档只共享短注册表锁。调用者维护 begin/to/end 因果顺序。销毁不得与使用同一句柄的调用并发，特别不能并发写同一 handle**；查询可以并发，各自独立输出。1.1 session 的小命令提交、视口更新和已发布元数据查询可在 GUI 线程调用；生命周期与大帧字节复制仍须在后台调用。

订阅属于 core，接收其全部文档事件，按 document_id 路由。借用 PaintEvent 仅回调期间有效，UI 须复制并排队。DOCUMENT_CHANGED（1）为像素/结构 revision 变化，目前 dirty ROI 为整张文档；DOCUMENT_STATE_CHANGED（2）为笔触状态/历史深度/活动层变化，revision 可以相同，width/height=0。task_id/reserved 为零；status 可携带像素回滚的资源错误。无初始订阅事件，创建/销毁文档不发事件，客户端在 API 成功后更新文档列表。

回调在触发调用线程执行，无注册表/文档/gate 锁；多个线程事件允许交错，以 revision 判断像素帧新旧。回调允许只读查询，禁止修改/创建/销毁/订阅/撤订阅，合法请求返回 BUSY，避免递归和自等待。callback/user_data 保持有效至 unsubscribe 返回；unsubscribe 先禁用、等待在途回调、移除再置 NULL。GUI 关闭顺序：工作线程撤订阅 → 终止客户端工作 → 销毁文档 → 销毁 core。

## 1.1 异步 Session

新增十个函数：create/destroy/submit/info/layer_info/layer_name/error_message/set_viewport/frame_info/read_frame。Session 独立拥有文档，不与 PaintDocument 共享可变状态；可与旧接口共存，但 session 不发送旧 document subscription 事件，使用轮询已发布状态。

64 位布局固定：PaintCommand=176（point offset=64）、PaintSessionInfo=104、PaintViewport=64、PaintFrameInfo=96 字节。嵌套 stroke/point 也初始化 struct_size。命令 kind 为 new/begin/move/end/cancel/undo/redo/add/remove/select/properties（1..11），只读取该 kind 使用的字段；未知 kind=UNSUPPORTED。Begin 的 stroke.layer_id=0 在实际执行时解析活动层，非零显式选层；失败恢复原活动层。名称同步复制后不借用调用者指针。

submit 成功只表示已接受，out_sequence 单调递增。session_info.completed_sequence 表示完成前缀，最新异步失败由 last_error_status/last_error_sequence 和 session_error_message 读取，不能以提交线程 TLS 判断执行结果；序号匹配时读最新错误，不是持久化逐任务结果。渲染错误归于当时完成前缀，同一前缀可能产生多个错误。新文档成功重置执行错误和 modified。flags RUNNING/MODIFIED/FAILED 为 1/2/4；文档 worker panic 后停止、拒绝提交/帧/视口，保留安全元数据供诊断，仍能销毁；完成前缀不声称包含未完成命令。

每 session 有 4096 项普通 FIFO。Move/End 遇满队列会剔除该笔连续 Move 尾部、注入预留 Cancel、返回 LIMIT_EXCEEDED/out_sequence=0。之后该笔 Move/End 为 CANCELLED，下一次成功接受 Begin 才重启输入。显式 Cancel 亦剔除 Move 尾部，无活动笔触时幂等成功；前序控制与完整笔触不越序。失败的 BUSY 控制请求不能取消正在绘制的事务。取消/过载不创建撤销项。

info/layer_info/layer_name 读取不可变 publication，图层枚举必须传 info.publication；不匹配 BUSY，应重试整次枚举。顺序从底到顶。generation 隔离新文档；旧文档代际的视口请求 BUSY。每 session 有四个视图槽 0..3，0 优先，其余背景优先级；Qt 使用 0 主画布、1 导航、2 空闲图层预览。enabled=0 取消并释放该槽帧，帧查询 BUSY。输出每边最多 4096、总像素最多 4,194,304。区域有限、正尺寸，支持画布外透明区域；工作坐标/尺寸绝对值不超过 2,000,000。

set_viewport 返回新的 request_id；替换请求使旧帧不可读。frame_info 返回完整帧的 generation/revision/request_id/frame_id/文档区域/物理像素大小。read_frame 必须精确匹配两个 ID；已替换或被撤销屏障失效返回 BUSY，重新查询。generation 防跨文档，request_id 防跨视口，revision 不倒退；持续绘制可显示有序较早帧，撤销/取消/结构修改拒绝屏障前的完成帧。

read_frame 仅接受 RGBA8 sRGB 预乘格式（3），与字节序无关；stride>=width*4、required=(height-1)*stride+width*4，复用 PaintTile 原内存/容量/不写 padding 契约。NULL/0 查询 BUFFER_TOO_SMALL，容量/格式/ID/溢出失败不写像素。每槽最多一份最新完整 Rust 帧及一个当前渲染作业；显示字节和模型线性像素分开。销毁从注册表移除，再清空队列、取消并 join，整个等待过程不持有注册表或 GUI 桥接锁。


## ABI 1.2 文件任务

旧前缀、符号和 session 行为保留。PaintFileRequest=64 字节（path 偏移16，generation偏移32），PaintFileJobInfo=64 字节；C11/C++20 实际 static/shared 调用验证布局与保存/重开。四函数为 paint_session_file_submit/info/cancel/message。每 session 最多一个 Queued/Running 作业；job_id 非拥有句柄，单调递增，结果保留到下次提交，旧ID NOT_FOUND。提交失败 out_job=0，提交成功不代表文件操作成功。

kind Open=1 / Save=2；format Auto=0 / PNG=1 / JPEG=2 / WebP=3 / OpenRaster=4。打开要求Auto、按内容签名识别；成功结果format返回实际格式，不以扩展名决定是否可直接保存文档。路径为OS UTF-8路径+长度（1..32768字节，禁止NUL），调用内复制；无Qt URL类型。quality=1..100，linear_background RGB为有限0..1、alpha=1，reserved全零。

状态 Queued=1 / Running=2 / Succeeded=3 / Failed=4 / Cancelled=5，终态由info.status和message读取，消息UTF-8无NUL。info包含captured source_generation/revision、result_generation/revision；Open仅在捕获代际/修订未改变、无活动笔触时应用新文档，否则BUSY且旧文档保留。Save读取不可变快照，后台编码，不阻塞actor/GUI；仅同代际同修订、无活动笔触的成功ORA保存清除modified。扁平导出与保存期间的新修改保持modified。

文件任务使用独立CPU工作线程；codec单次调用不能强杀，行/瓦片/读写及提交点检查取消。cancel只对Queued/Running有效，已终结返回BUSY。同目录临时文件flush/sync后persist原子替换为提交点；提交后成功不可伪称已撤回。销毁取消作业、清空队列并后台join。格式/资源/ORA限制见docs/document/io-design.md；PSD/ICC/GPU等能力未报告。


## ABI 1.3 暂存盘配置

所有旧函数/DTO 布局保留。新增能力 PAINT_FEATURE_STORAGE_SETTINGS=128；PaintStorageOptions 大小 48 字节，path 偏移 8，PaintStorageInfo 大小 32 字节（64 位平台）。reserved 为 0，struct_size 规则不变。

paint_core_create_with_storage(options, out_core) 先验证和检查暂存目录，成功才生成拥有 core，失败输出 NULL。配置不可变，所有后续文档/session、新建与导入继承此配置；创建新 core 或重启应用应用新设置。旧 paint_core_create 使用默认懒初始化存储，保持原调用兼容。销毁 core 仍要求先释放文档/session/订阅；存储后备可以由后台快照延长寿命。

path_length=0 选择系统临时目录，此时不读取 path；否则绝对 OS 路径 UTF-8 1..32768 字节、禁止 NUL，目录必须存在。resident_bytes 为 65536 的倍数，范围 65536..1073741824（64KiB..1GiB）；scratch_bytes 为单文档瓦片暂存区段预算，65552..68719476736（约64KiB..64GiB）；min_free_bytes 为文件系统保留空闲阈值，0..1099511627776（1TiB）。不是整个进程内存或所有文档总磁盘上限。磁盘历史的独立 4GiB 预算和 100 条限制不变。

paint_storage_inspect(options, out_info) 不修改现有 core；在候选目录检查空间、尝试创建可写 run、检查并清理已失活的标记目录，返回可用字节、本次清理目录/字节与跳过数量。探测失败把有效 out_info 前缀清空。探测并非纯查询：可创建专用命名空间及清理符合条件的残留，不会恢复绘画；完成后删除自身探测 run。清理扫描有界，陌生内容/链接/无有效租约/锁定实例跳过，旧版本 Temp 散落文件不凭前缀删除。

两个函数只在后台线程调用；禁止回调内调用。预算/UTF-8/相对路径错误 INVALID_ARGUMENT，维护锁争用 BUSY，可重试；不可访问、不可写、标记冲突、空闲不足 IO_ERROR。可用空间为即时采样，不能预留给本进程独享；增长检查每 1MiB 或 1 秒刷新，其他进程可继续消耗磁盘，真实失败仍按原有笔触/历史事务处理。具体清扫边界见 docs/document/storage-settings-design.md。


## ABI 1.4 图层配置与原始内容预览

旧 DTO/命令字段及符号保持原布局。能力 PAINT_FEATURE_LAYER_APPEARANCE=256。新增 PaintLayerAppearance 大小 32 字节：struct_size(0)、locks(4)、blend_mode(8)、reserved(12)、fill(16)、offset_x(20)、offset_y(24)、dissolve_seed(28)。reserved=0，fill 有限 0..1，locks 位为 TRANSPARENCY=1 / POSITION=2 / ALL=4，其他位拒绝；位置为有符号整数像素，绝对值最多 1,000,000，保留画布外内容。seed 是确定性溶解噪声的持久化参数。

blend_mode 稳定 ID：0 Normal，1 Dissolve，2 Darken，3 Multiply，4 ColorBurn，5 LinearBurn，6 DarkerColor，7 Lighten，8 Screen，9 ColorDodge，10 LinearDodge，11 LighterColor，12 Overlay，13 SoftLight，14 HardLight，15 VividLight，16 LinearLight，17 PinLight，18 HardMix，19 Difference，20 Exclusion，21 Subtract，22 Divide，23 Hue，24 Saturation，25 Color，26 Luminosity。未知 ID/flags、非有限 fill、过界位置 INVALID_ARGUMENT。原始透明度使用既有 Properties。当前 fill×opacity 缩放内容，特殊 Photoshop Fill 曲线未实现。

paint_session_layer_appearance 绑定精确 publication ID，过期 BUSY、缺失层 NOT_FOUND，有效输出前缀按既有规则清空。paint_session_set_layer_appearance 同步校验并复制 DTO，再将完整配置作为 FIFO 命令提交；out_sequence 的接受/执行含义与旧 submit 一致。paint_session_move_layer 提交相对整数 dx/dy，各绝对值最多 1,000,000；执行时检查最终位置和锁，不重采样像素。所有修改在 actor idle 时执行，否则 BUSY；成功变更进入历史；无变化不创建历史或 dirty modified 状态。

透明锁按原 Alpha 绘画，不改变 Alpha/透明区域，橡皮擦无效；位置锁只禁止移动，允许绘画；完全锁禁止绘画、移动、删除及 opacity/fill/blend/seed 变化。选择、显隐、锁位调整仍允许；解除完全锁与内容/外观修改分两个命令，不能用一个 DTO 的解锁绕过原锁。失败 LAYER_LOCKED=11 不产生部分修改。撤销/重做不受当前锁限制，完整恢复配置与像素；异步错误按既有 session error 契约查询。

paint_session_set_layer_preview(core,session,layer_id,PaintViewport*,out_request) 要求明确层 ID 和 view_id=2/3，每边最多 96 物理像素。复用原 generation / region / enabled / request_id 验证与 frame_info/read_frame，不增加句柄。背景任务仅在无活动笔触时复制目标层的瓦片引用，按原始内容渲染（visible=true、opacity=fill=1、Normal），保持位置；新修订/绘画取消过时任务。set_viewport enabled=0 释放该槽预览，set_viewport 普通请求覆盖预览模式。Qt 只请求可见委托，安静 200ms 后一个任务依次生成 64px 预览；滚动可请求任意层，PNG 编码留在后台线程。预览不用于导出或文档最终合成。


## ABI 1.6 蒙版、拖放和白色画布

保留 1.5 的所有 DTO 尺寸和旧命令语义，新增 PAINT_FEATURE_MASKS=1024 和层级 kind MASK=2；PIXEL=0/GROUP=1 不变。蒙版为专用节点，parent 为所属图层或组，depth 比所属节点多一层，每个所属节点最多一个蒙版。发布数组仍是后序从底到顶。老客户端遇到未知 kind 应拒绝编辑，不能当普通颜色图层处理。

PaintCommand 新 kind NEW_WHITE_DOCUMENT=12（width/height 同旧 NEW_DOCUMENT 验证）及 ADD_MASK=13（显式 layer_id）。旧 NEW_DOCUMENT=1 继续建立透明文档，保证旧调用行为。UI 默认提交 12，新建和启动得到真实白色“背景”，零历史且无需整图分配；显式透明画布可选。ADD_MASK 创建默认完全显示的稀疏蒙版并选择它。既有 BEGIN_STROKE 选择蒙版节点后绘制覆盖率，黑隐藏、白恢复，显示空间灰度控制覆盖率，压感控制半径和流量；橡皮擦恢复显示。蒙版 visible=false 禁用，opacity 为密度。REMOVE_LAYER 删除蒙版，父节点的子树删除包含蒙版；最后一个颜色图层保护不把蒙版算作颜色图层。

新增 PaintLayerDrop=24 字节：struct_size(0)、placement(4)、layer_id(8)、target_id(16)。paint_session_drop_layer(core,session,request,out_sequence) 同步复制与验证，placement 0 移入 target 组；target=0/placement=0 移到根顶端；placement 1/2 放在 target 同级上方/下方。完整子树移动，不改变像素位置。mask 不能单独归组，循环、缺失节点、深度、锁和存储失败由 actor 原子拒绝；无效同步参数返回 INVALID_ARGUMENT 并清空 sequence。异步接受不表示执行成功，继续按 session_info/error_message 查询。

选择、蒙版绘画/增删/显隐、拖放均复用异步队列、历史和代际。Qt 原生拖放负载另携带当前代际，过时文档负载拒绝。带蒙版的组暂时拒绝直接解组，避免丢失效果；删除蒙版后可解组。独立 UI 窗口共享同一客户端和核心文档。

ORA 自定义层扩展版本 2 记录节点 ID/parent/kind 和隐式白色矩形。含蒙版或隐式背景时，stack.xml 只包含 mergedimage.png 兼容图层并声明 dv:editable=2；drawverse/stack.xml 与 data/*.png 保存可编辑树。加载严格验证两个尺寸一致、版本、全部 ID、父节点、深度、预算和文件完整性；缺少/损坏清单必须失败，禁止降级为“保存成功但内容已扁平”。普通 ORA 阅读器可显示合成预览。当前 PNG 图层数据为 8 位 sRGB，不保证 32F 文件无损，内存/磁盘历史仍逐位无损。


## ABI 1.7 剪贴元数据

PAINT_FEATURE_CLIPPING=2048；所有旧 DTO、命令与节点 kind 保持原有布局/语义。新增 PaintLayerClipping 为 repr(C) 16 字节：struct_size u32 @0、enabled u32 @4、base_layer_id u64 @8。

paint_session_layer_clipping(core,session,publication,layer_id,out) 读取一致发布；publication 不匹配为 BUSY 并重置输出；enabled=0/1，base_layer_id 为同级最近非剪贴节点 ID；非剪贴/蒙版或孤立链为 0。孤立链 enabled 仍为 1，可被释放，不假装有基底。

paint_session_set_layer_clipping(core,session,layer_id,options,out_sequence) 输入 enabled=0/1、base_layer_id 必须 0；同步复制小 DTO 并立即验证，输出 sequence 先清 0；actor 校验 idle、所属祖先完全锁、节点类型与创建时下层基底。失败不改状态/历史；重复值无历史。蒙版节点不能剪贴。修改进入 FIFO、撤销屏障和 modified 状态；基底依当前同级层序动态解析，不跨组引用、不烘焙像素。蒙版 owner 内部仍有独立节点，UI 仅合并呈现。

剪贴链在父隔离组合成前保持基底 alpha：像素蒙版先作用于基底/上层源，连续剪贴层在基底颜色上混合，最后一次性应用基底 opacity/fill/blend。关闭或透明基底不显示上层，剪贴层隐藏不切断链。结构/删除历史保存剪贴标志；临时无损历史 DVH5 不属于对外工程文件。ORA dv:version=3 保存 dv:clipped，仍读取 v1/v2；有剪贴使用可编辑原始栈及合成预览。


## ABI 1.8 几何选区

能力位 PAINT_FEATURE_SELECTION=4096，所有旧 DTO/函数保留。PaintSelectionEdit=64 字节（x 偏移32），PaintSelectionInfo=16，PaintSelectionStep=48（x 偏移16）。均初始化 struct_size、其余字段为零。request 无指针，调用返回前复制；out_sequence 在拒绝时为0，OK 仅表示排入 FIFO。

paint_session_edit_selection 的 action 为 Shape=0 / Clear=1 / All=2 / Invert=3。Shape 时 operation 为 Replace=0 / Add=1 / Subtract=2 / Intersect=3；shape Rectangle=0 / Ellipse=1，antialias=0/1。几何为文档浮点坐标，finite 且绝对值不超过2,000,000，宽高必须正。非 Shape 的所有无关字段（operation、shape、antialias、geometry）为零；reserved[3] 始终零。未知参数不入队；活动笔触或超过64步骤等执行错误通过 session publication 报告，旧选区保留，不取消进行中的笔触。

paint_session_selection_info/selection_step 使用与 layer_info 相同的 exact publication，过期为 BUSY 并清空输出；不存在步骤为 NOT_FOUND。Invert 步骤 operation=4，shape/antialias/geometry 为零。步骤仅供显示，核心负责覆盖与绘画。enabled=0 表示不限制；enabled=1 且没有覆盖像素表示禁止绘画，不能将两者混同。默认禁用，新建清空；选区编辑使用统一历史、modified 规则和 revision，选择工具切换本身不改文档。当前选区作用于文档坐标的笔刷、擦除和蒙版绘画，不裁剪显示/导出或图层整数移动。

查询不暴露内部分配或句柄；调用者输出内存/生命周期/回调规则沿用本契约。旧同步 API 没有选区编辑入口；扩展功能通过异步 session 提供。OpenRaster 扩展 v4 保留选择步骤，图片导出保留完整画布像素。


## ABI 1.15：共用形状参数和套索

PaintSelectionShapeEdit=72 字节：struct_size/shape/operation/antialias/tool/point_count（0–20），feather（24），x/y/width/height（32–56），points（64）。shape=0 矩形、1 椭圆、2 多边形；前两种 tool/point_count=0、points=NULL，多边形 tool=1 自由套索或2 多边形套索，矩形字段全0。operation=0/1/2/3，antialias=0/1，feather 为有限的文档像素半径0–1000。路径3–4096点、坐标有限且绝对值不超过2000000，非共线；选区总路径点数不超过4096（actor拒绝超过预算）。奇偶填充支持凹形及自交，羽化用对称 smoothstep 距离过渡，50%边界不移动。旧几何编辑接口默认羽化0保持既有语义和 DTO 大小。

paint_session_edit_selection_shape 同步复制、校验 request 及所有路径点后提交既有 FIFO 选区命令，返回前不保留 caller 指针。所有空/未对齐指针、数量、形状/工具、参数失败清空 out_sequence；OK 只表示入队，仍查询完成序号和错误状态。无变化不创建历史。历史21为套索、24为多边形套索，20为椭圆、4为矩形；22为魔棒（ABI 1.17 起产生）。

PaintSelectionDetails=16字节，struct_size(0)、point_count(4)、feather(8)。paint_session_selection_details(core,session,publication,index,out,points,capacity) 与 selection_step 同一 publication，不一致返回 BUSY；容量不足返回 BUFFER_TOO_SMALL 并给出 point_count，不写点缓冲；NULL/0可查询需求。足够容量一次复制所有点，无点形状不解引用点缓冲。输出调用者需保证 capacity 可写点，生命周期与旧查询相同。选区文本 v3 保存半径/路径，读取 v1/v2；磁盘历史和 ORA v4 仍使用有界选区文本。旧核心拒绝未知文本版本，不静默丢失新选区。

## ABI 1.16：油漆桶

新增 64 字节 PaintBucketFill，offset 16 的 linear_rgba 为 straight linear sRGB，opacity 0–1、tolerance 0–255；坐标为文档像素索引。antialias/contiguous/all_layers 均只接受 0/1，reserved 和 reserved_tail 为零。新增 PAINT_FEATURE_BUCKET=8192 和历史 PAINT_HISTORY_BUCKET=25。paint_session_bucket_fill 复制并校验 DTO 后返回队列 sequence，像素填充由串行 actor 异步执行；完成/失败通过既有 session publication 和 last_error 读取。

种子越界、组目标、完全锁、蒙版 all_layers 取样、资源/存储错误均不提交像素；无变化不生成历史。all_layers 使用填充前的可见合成，仅写活动层；默认以当前层原始像素（不含显示 opacity/fill）比较。选区及 alpha lock 生效，整次填充只产生一条历史。容差按 straight sRGB/alpha 最大差 0–255，AA 在阈值之外一色阶内软化覆盖，透明色比较忽略 RGB。工作量与错误语义见 docs/tools/bucket-design.md，旧 ABI DTO 和函数保持不变。

## 历史时间平滑IO准备阶段（当时ABI1.45，现已接入1.46）

Rust IO新增state14/VBR 1.0/behavior11/time-grid-v1，旧默认不升级。当前原生准确准备拒绝非默认平滑，公开库包/程序包写入及库包资源交接拒绝含smoothing对象的包，以防尚只支持state13的Qt丢字段。未公布平滑路径能力，PAINT_FEATURE_PRESSURE_SMOOTH仍仅表示压力插值，不表示时间平滑设置已经接入。见docs/validations/brush/dynamics/brush-smoothing-io-validation.md。

## ABI1.49 复合基础笔尖几何

新增PAINT_FEATURE_COMPOSITE_BRUSH_POSE（17592186044416）和paint_brush_pose_read_composite；复用已存在的pose创建/准确prepared绑定/clear/destroy，单笔尖读取接口及DTO保持。PaintCompositeTipPose为64字节，struct_size/flags/两个24字节PaintTipPose/reserved u64。flags仅SECONDARY(1)、PRIMARY_IMAGE(2)、SECONDARY_IMAGE(4)、SECONDARY_OVERFLOW(8)。无第二时数据清零且嵌套struct_size有效；错误/Busy均清除两份几何与flags。

只读基础姿态，不抽取随机印章或模拟路径平滑；第二独立角度/倾斜/压力、相对标称与固定实际边宽共用Rust规则。有效overflow仅启用双色并允许溢出时为真。读取不等待编译、不访问文件，captured修订/资源寿命由pose保持；库变更或销毁不改变绑定结果。未新增state/VBR字段。本阶段Qt仍使用旧主笔尖入口，完整复合光标待下一阶段接入。

## ABI 1.50 — shared wet brush tools

`PAINT_FEATURE_WET_BRUSH` (bit45) exposes full state17/VBR 1.0 preparation and tool ownership; this does not imply a Qt settings page. Existing DTOs retain their layouts.

- `PaintWetLoadDesc`: size32, radius at8 and straight linearRGBA at16, both reserved words zero. radius0.5..1000; RGBA finite0..1.
- `PaintWetToolInfo`: size40/alignment8; flags at4 (INITIALIZED=1), carrier_revision at8, edge at16, reserved zero at20 and center premultiplied linearRGBA at24. Center sample is not a grid mean. BUSY/error clears all fields except returned struct_size40. Initially flags/edge/revision/color are zero.
- `paint_wet_tool_create/destroy`: typed independent tokens; destroy clears caller handle and does not cancel bound/in-flight sessions. At most64 external tools remain alive, counting destroyed tokens retained by sessions; registry Weak ownership records enforce this without stale-pointer ABA. Default per-session internal tools remain compatible.
- `paint_session_bind_wet_tool`: queued, copies shared ownership, matching core/session required. Busy session rejects asynchronously without changing its tool. Idle sessions may bind an occupied tool; actual begin reports Busy on conflict. Binding changes no document pixels/history.
- `paint_wet_tool_info/clean`: idle tool snapshot/manual clean. Painting lease returns Busy. No callbacks/document work execute under the tool lock.
- `paint_wet_tool_reload_prepared`: copies ID/32-byte descriptor, resolves exact preparation nonblocking (Busy while compiling), requires wet enabled, uses captured WetMix and current provided color/radius. Does not silently substitute latest preset. Invalid/stale/unknown inputs preserve carrier.
- `paint_brush_library_prepare_state_wet` and `paint_brush_library_package_read_wet/write_wet` opt in to complete wet policy, including closed nondefault values and resources. Legacy preparation/package entries keep returning Unsupported for new policy to protect older Qt consumers.

Existing prepared-state/pose/with_blend stroke exports consume the exact opted-in ticket. Engine lease adopts new ink only after successful document/history finish; cancel/error/replacement/stop drops uncommitted ink. Undo/redo restore pixels, not tool reservoir. Tool/manual actions and preparation/file operations retain the existing worker-thread/callback/pointer rules; no registry guard is held while waiting for preparation or executing tool operations.

### ABR opacity conversion / VBR 1.0 implementation update

The existing ABR read, prepared state, source-info and package APIs also carry basic converterVersion2 provenance when a newly imported non-100% Opct is executed as stroke opacity. Source record structure version remains1; existing version1 records are never recomputed. These source-bearing packages use VBR 1.0; source/converter records retain their independent validation. No DTO, ABI minor, feature bit or state field changes; unchanged full-source byte handoff and baseline parameters are sufficient. See `docs/brush/abr/abr-opacity-design.md` and its validation record for limits; this does not imply full ABR dynamic compatibility.

### ABR pressure conversion / versioned JSON handoff update

The existing ABR read API returns JSON handoff version2 only for a batch executing converterVersion3 pressure channels. The row layout is unchanged; parameters now includes the existing pressureDynamics payload and independent pressureSize/pressureFlow switches. Clients must reject unknown handoff versions; version1 remains unchanged for older conversions. Updated Qt accepts1/2 and validates/copies the full pressure object through existing state5/6. Source record structure remains version1; converter3 requires a pressure baseline and still rejects other dynamics. VBR 1.0 carries these sources; source/converter records retain their independent validation. No C ABI layout/minor/feature bit or new executable brush behavior is introduced. Old UI explicitly rejects the new handoff rather than silently losing minima/opacity pressure. See docs/brush/abr/abr-pressure-design.md.

### ABR scatter conversion / JSON handoff3

The existing ABR read API returns handoff version3 when a batch includes converterVersion4 scatter provenance. parameters.scatterDynamics uses the existing validated state8 scatter payload, alongside an independent optional pressureDynamics object. Old handoff1/2 data remains unchanged. Updated Qt accepts1/2/3 and rejects scatter fields in older handoffs atomically; old Qt rejects3 before publication. Converter4 source baselines require scatter data, older source converters reject it. Packages containing such sources use VBR 1.0. No C ABI minor, DTO, feature bit, new state or executable behavior version is introduced. Nonzero random conversion is explicitly approximate; device/fade scatter controls remain unexecuted. See docs/brush/abr/abr-scatter-conversion-design.md.

ABR color conversion extends that JSON handoff to version4 for converterVersion5 sources. parameters.colorDynamics reuses validated state10 data, independently of optional pressureDynamics and scatterDynamics. Current Qt accepts handoff1..4; color fields in older handoffs are rejected before catalog publication. Older Qt rejects version4 instead of dropping new parameters. V5 baselines require color data; older converters reject it. Source-bearing packages use VBR 1.0; existing ABI1.50, state and execution behavior versions stay unchanged. Native linear HSV/random behavior is reported approximate; purity/device modulation remains unexecuted. See docs/brush/abr/abr-color-conversion-design.md.

ABR shape jitter conversion returns handoff5 for converterVersion6 sources. parameters.shapeJitter reuses state9, independently of optional pressure/scatter/color DTOs. Qt accepts1..5 and atomically rejects shape data in older handoffs; old Qt rejects5. V6 source baselines require shape data, older source versions reject it. Such sources use VBR 1.0; no C ABI minor, DTO, state or execution behavior change. PS random distributions and combined pressure/minimum rules remain approximate, while unsupported device controls retain field-level reports. See docs/brush/abr/abr-shape-conversion-design.md.

## ABI 1.51 — stroke taper parameter entry

`PAINT_FEATURE_STROKE_TAPER` (bit46) exposes full state18/VBR 1.0/behavior15 parameter support through `paint_brush_library_prepare_state_taper` and `paint_brush_library_package_read_taper` / `paint_brush_library_package_write_taper`. These accept prior supported data, including complete wet settings. The original and `_wet` entry points retain their support boundaries and reject nondefault taper rather than silently losing closed parameters. Existing DTO layouts and prepared stroke calls remain unchanged.

Preparation returns an exact asynchronous ticket; `paint_brush_library_prepared_state` retains all taper fields and resource identities. Failed drafts do not replace the last valid catalog. Recipes preserve the selected support level when unused image revisions are retired and rebuilt. Prepared strokes capture the full preset through existing `paint_session_begin_prepared_preset_with_blend`; modifying the catalog or destroying its external token does not change an active stroke. Taper distances remain document pixels under temporary tool size overrides.

Package reads use the existing size probe and atomic resource adoption contract: insufficient capacity or invalid output pointers must not install assets. Writes resolve all resources and validate the complete bundle before atomic save. Outputs, callbacks and worker-thread requirements are unchanged. This capability does not claim a Qt settings page; the UI must mirror state18, preserve reset baselines, select the new explicit entry points and continue rendering automatic previews in white before exposing the controls. See `docs/brush/wet/brush-taper-abi-design.md` and its validation record.

## ABI 1.52 — independent wet dual carriers

`PAINT_FEATURE_WET_DUAL_COLOR` (bit47) adds `paint_brush_library_prepare_state_wet_dual` and `paint_brush_library_package_read_wet_dual` / `paint_brush_library_package_write_wet_dual` for complete state19/VBR 1.0, including closed `wetMix.dualColor` policies and all prior supported fields. Legacy, wet and taper entry points retain their previous boundaries. Preparation recipes retain the opt-in across resource retirement/reconstruction. Existing prepared stroke calls capture both colors and use the existing Engine/Document pair implementation; no new stroke or tool token type is needed.

`PaintWetToolInfoDual` is 80 bytes, alignment8: struct_size/flags at0/4, primary/secondary_revision at8/16, primary/secondary_edge at24/28, two premultiplied linear center RGBA arrays at32/48, reserved[4] at64. INITIALIZED=1 marks the primary, SECONDARY_INITIALIZED=2 marks the secondary. One idle snapshot returns both roles; BUSY or handle errors clear the complete output and set struct_size=80. The original 40-byte info remains a primary-only view. Center colors are samples, not reservoir averages.

`paint_wet_tool_reload_prepared_dual` takes an exact library preparation/id, the existing 32-byte load descriptor (temporary primary radius and foreground) and 24-byte background descriptor. Both descriptors require zero reserved fields, finite valid colors and radius0.5..1000. Active painting returns BUSY without waiting for compilation. Active marked dual-color wet presets atomically load both carriers: secondary color follows the preset foreground/background selection, and nominal secondary diameter uses the same core independent/relative mapping as automatic start. Ordinary wet presets reload the primary while preserving inactive secondary ink. Disabled wet presets reject loading. Old reload rejects every marked policy, including closed values, with UNSUPPORTED; old clean already cleans both roles. Neither preset state nor document history serializes live tool ink.

Package probes/invalid output pointers install no resources; writes validate before atomic save. No existing layout is changed. Qt must mirror state19, preserve reset baselines, opt into these new entry points and retain white/Normal automatic previews before exposing the combination. See docs/brush/wet/brush-wet-dual-abi-design.md.

## ABI 1.53 — display-only image tip cursor PNG

`PAINT_FEATURE_BRUSH_CURSOR_IMAGE` (bit48) adds `paint_brush_library_tip_cursor_png`. Call it on a background worker with a live library and UTF-8 immutable coverage identity. It returns a cached PNG of `PAINT_BRUSH_CURSOR_IMAGE_EDGE` (256) square straight RGBA8: white RGB, actual base coverage alpha, aspect preserved with centered transparent padding. The raster integrates the base bilinear coverage with repeated edge samples; it is a bounded display representation, not a rendered stroke or a guarantee of pixel identity with every painting mip/scale.

The caller supplies nonoverlapping readable identity bytes and writable buffer/required storage. No caller pointers are retained. Null buffer/zero capacity queries the full byte count and returns `PAINT_BUFFER_TOO_SMALL`; insufficient capacity writes only required, never partial PNG bytes. Other errors clear required; a null buffer at sufficient capacity is invalid. Counts use the standard byte-length bounds. The implementation snapshots the resource store before generating, without holding the global handle registry. Shared resource stores outlive their original library owner.

Encoded display cache is limited to 8 entries/4 MiB and 512 KiB per PNG. A cache hit reuses a previously verified immutable result; cache misses load and verify the coverage resource. Encoded images do not pin source masks or mips. Cancellation is checked before/after bounded generation and encoding; there is no per-pixel interruption guarantee. This does not mutate the catalog, prepared recipes, document pixels/history or wet tool. No existing DTO or state/VBR layout changes. Qt consumption/global visibility and opacity preferences are a separate integration stage.

Grouped state forward guard (no ABI version change): Rust state20 can carry ordered-tree-v1 organization. Existing Legacy/Wet/Taper/WetDual preparation recipes reject any nonempty group record set before adoption, including empty folders; current Qt cannot mirror it. Flat older states retain their bytes and accepted semantics. This is not a grouped C ABI entry or grouped VBR support; those require explicit integration.

VBR 1.0 organization is currently a Rust IO capability only. The four library package readers (Legacy/Wet/Taper/WetDual) reject bundles with group records, including empty folders, before installing resources or emitting flat Package JSON; required is zero on rejection. Their strict flat writers reject an added organization field. Plain flat VBR behavior remains compatible. Grouped state20 preparation protection remains in place; no new ABI entry or capability is claimed in this stage. Destination-library atomic merge and the complete grouped native/Qt handoff remain future work. See docs/validations/brush/library/brush-groups-vbr-validation.md.

## ABI 1.54 — explicit grouped state preparation

PAINT_FEATURE_BRUSH_GROUP_STATE (bit49) adds paint_brush_library_prepare_state_groups with the same live-library, bounded readable JSON and writable nonoverlapping sequence contract as older preparation functions. It accepts validated state20 organization and all prior complete parameters, including closed wet dual/taper settings. Invalid call arguments clear sequence; accepted requests return a ticket whose eventual result may still be a validation/resource error. prepared_state exposes the exact canonical snapshot and normal current/mutate publication uses the same ticket pipeline, immutable source receipts and cold-rebuild support identity.

The four older preparation levels still reject group records, including empty folders. Existing legacy library package readers/writers remain protected: GROUP_STATE does not claim grouped VBR package handoff or directory commands, and current Qt has not yet mirrored group state. state20 and VBR 1.0 do not change. See docs/validations/brush/library/brush-groups-abi-validation.md.

## ABI 1.55 — complete grouped package handoff

PAINT_FEATURE_BRUSH_GROUP_PACKAGE (bit50) adds paint_brush_library_package_read_groups and paint_brush_library_package_write_groups with the existing bounded path/JSON and buffer contracts. JSON is the strict ordered-package-v1 wrapper with model string, package and required ordered-tree-v1 organization. Flat packages also have an explicit flat organization. PackageHandoff validates all package fields and the exact DFS preset projection, rejects unknown/null/wrong-model fields and applies the existing 2MiB limit to the complete wrapper. See contracts/brush/library/brush-package-handoff.schema.json and examples; global tree/projection semantics are checked in Rust.

Read capacity probes do not install assets or sources, and required is initialized to zero on errors. Successful read installs immutable resources and complete source batches before returning metadata, without replacing current catalog. Write validates the wrapper before resource lookup, retains source baselines/current resources and uses existing atomic file save. Legacy/Wet/Taper/WetDual readers/writers retain their group rejection. Group-state preparation remains the ABI1.54 entry; VBR 1.0/state20 versions do not change. Qt complete mirrored state, copied tree merge/reset baselines and group management are not yet enabled by this capability.

### ABI 1.68：蒙版移动链接

PAINT_GROUP_MASK_LINK=5 使用现有 PaintGroupRequest：layer_id 为蒙版，parent_id 为链接布尔值0/1，name=NULL/name_length=0。返回OK只表示排队，非蒙版/锁/忙/历史失败由actor返回；同值不增加修订历史。链接默认开启，关闭后移动/变换所有者或组跳过该蒙版，显式选中蒙版仍可移动。

paint_session_layer_mask_linked(core,session,publication,layer_id,out_u32) 按准确 publication 返回0/1；非蒙版返回1，未知节点NOT_FOUND，旧publication返回BUSY。输出须可写且不与其他参数重叠，失败清零；不借用内部指针，不改变已有DTO。

持久化：旧二进制 mask 字节0/1不变，3表示解除链接蒙版，其他值拒绝；历史与剪贴板保留该标志。ORA 无解除链接仍写4（栅格）/5（置入模型）；解除链接写6/7，dv:mask-linked=0（缺省1），只有5/7必须含drawverse/layers.bin。旧读取器拒绝6/7；所有候选及历史成功后才发布。
