# C ABI 1.9.0 契约

## ABI 1.9 混合模式显示预览

`paint_session_set_blend_preview(core,session,layer_id,blend_mode,viewport,out_request)` 只覆盖视口槽 0/1 的完整文档合成快照，使用既有 0–26 混合编号与 PaintViewport DTO。必须指定存在的非蒙版图层/组；零 ID、非法混合或非 0/1 槽返回 INVALID_ARGUMENT，不存在/蒙版 ID 返回 NOT_FOUND，旧 document_generation 返回 BUSY；失败清零 out_request。调用者内存只在本次调用读取/写入，不保留。

预览共享快照瓦片，只修改快照副本的外观。文档修订、图层元数据、历史、modified 与文件保存不受影响；返回递增的该槽 request_id，沿用异步帧读取、取消和旧帧拒绝。再次调用替换预览，普通 paint_session_set_viewport 清除覆盖，enabled=0 释放帧，新文档清空覆盖。新增函数向后兼容，既有 DTO/函数/能力位保留，Qt 预览客户端要求版本至少 1.9。

## ABI 1.5 图层组

新增能力 `PAINT_FEATURE_LAYER_GROUPS=512`；旧 DTO 大小、函数和混合编号保留。`PaintLayerHierarchy=24` 字节，parent_id 偏移8，depth/effective_locks 为固定 u32；kind Pixel=0 / Group=1。`paint_session_layer_hierarchy` 与 layer_info 使用同一 exact publication，不匹配返回 BUSY 并清空输出。

`PaintGroupRequest=40` 字节，name 偏移24；`paint_session_group` 同步复制请求与 UTF-8 名称后排入 FIFO，返回 out_sequence，只表示接受。Wrap=1 需 layer_id 和 name，parent_id=0；Ungroup=2 仅 layer_id；Reparent=3 需 layer_id、parent_id（0=根）。后两种名称指针必须 NULL、长度0；未知 kind/无关字段非法，不入队且 out_sequence=0。不存在/循环/锁/深度/历史错误由 actor publication 报告。

组枚举仍为底到顶后序：子树连续，组在子节点之后。最大16级父组、4096节点，至少一个像素图层。组不拥有瓦片，自身位置固定0，不接受透明像素锁；完全锁和位置锁传递给后代，effective_locks 包含祖先的这两个锁。组按隔离语义合成，组 opacity/fill/blend 在子树合成后应用一次；不支持穿透模式。

Wrap 包含指定图层/组及其子树；新组成为活动节点。Reparent 将子树放到目标组顶层，位置保持文档坐标；Ungroup 移除组属性、保留未烘焙子节点。既有 Remove 删除整组子树；最后像素层删除拒绝。既有 Move 移动所有后代像素层，一个历史事务；任一后代受锁或越界则整个操作拒绝。既有 Add 在所选组内或所选像素层同级上方新增。组不能 Begin 绘画，需选子像素层；预览接口对组返回子树合成，忽略目标组自身隐藏/opacity/fill/blend，保留子节点属性。

结构命令只保留元数据，删除子树可使用有界无损磁盘历史。组操作遵守 idle、撤销屏障与 modified 规则，撤销/重做仍忽略当前锁状态。层 ID 单调且不复用；ORA 重开重新分配 IDs，不能跨代际复用旧 ID。

正式源：`core/crates/paint-ffi/src/api.rs` 与导出函数 `lib.rs` / `session_api.rs` / `file_api.rs` / `storage_api.rs` / `layer_api.rs`；正式头：`ui/include/paint_api.h`，固定 cbindgen 0.29.2 自动生成。旧 draft.h 仅为设计历史。1.0 验证见 `docs/ffi-validation.md`，1.1 设计/验证见 `docs/task-render-design.md` / `docs/task-render-validation.md`。

## 版本与类型

全部函数返回 int32_t，包括 version/create/destroy；version 写 out 参数，当前 major/minor/patch=1/9/0。应用版本独立。DTO 全部 repr(C)，只含固定宽度数字和指针；无 enum/bool/size_t/long、Qt/STL/Rust 容器。支持 64 位；32 位 Rust 构建拒绝。

初始化 DTO 为零并写 struct_size=sizeof(本调用者结构)，输入至少含已发布前缀。库只访问已知前缀；过小为 INVALID_ARGUMENT，不覆盖调用者内存；扩展尾部保持不变。输出 struct_size 写库已知大小。reserved 必须为零；未知 mode/tool/format 为 UNSUPPORTED。输入 capability 位保留未知位，圆笔刷只实际使用 pressure，客户端为无压力设备提供 pressure=1。

已发布字段布局、语义、所有权、线程契约不能更改。破坏性修改提升 ABI 主版本；兼容新增函数/能力/新结构提升次版本。现有 DTO 不重新排序或更换字段类型。客户端启动先 version 再 capabilities，按位检查所需功能并忽略未知能力位。当前能力为 document/history/linear tile read/events/async session/CPU viewport/file IO/storage settings/layer appearance/layer groups及 masks/clipping/selection（1/2/4/8/16/32/64/128/256/512/1024/2048/4096），无 GPU。

## 错误

OK=0，INVALID_ARGUMENT=1，INVALID_HANDLE=2，BUSY=3，NOT_FOUND=4，LIMIT_EXCEEDED=5，UNSUPPORTED=6，CANCELLED=7，IO_ERROR=8，INTERNAL_ERROR=9，BUFFER_TOO_SMALL=10，LAYER_LOCKED=11。文件任务返回 IO_ERROR；异步会话产生 CANCELLED。未知非零按失败处理。原同步 API 的空 undo/redo 成功；无活动笔触的 end/cancel/to 为 INVALID_ARGUMENT；活动笔触期间图层操作/undo/redo 为 BUSY。

paint_error_message 读取当前线程最近错误，UTF-8 字节，无 NUL。NULL/0 查询，非空消息返回 BUFFER_TOO_SMALL；空消息 OK、required=0。该查询包括无效/小缓冲结果均不替换原错误；其他成功函数清空 TLS。回调重入使用同一 TLS，但最外层 API 返回前保存其最终结果。

可展开 Rust panic 在 ABI 边界 catch_unwind 转 INTERNAL_ERROR。文档锁被 poison 后拒绝读写，仍可销毁；不盲目恢复潜在不一致状态。OOM/abort、野指针、非法回调和外部异常不能依靠 catch_unwind 修复。C++ 包装捕获本地异常，回调必须 noexcept/自行捕获，不能跨 C ABI 抛异常。

## 句柄生命周期

core_create/new_document/subscribe 写出拥有句柄，失败写 NULL。调用者先释放旧拥有句柄，再复用创建输出存储。文档和订阅归属指定 core；跨 core、错误类型、旧或未登记句柄为 INVALID_HANDLE。句柄不得解引用、复制为第二个拥有者或跨动态库实例传递。

core 有存活 document/subscription/session 时销毁为 BUSY，保留句柄。销毁接受 handle**，成功置 NULL；已有 NULL 幂等释放，document/subscription/session 仍要求有效 core。handle** 自身必须有效、可写、非 NULL。销毁文档丢弃未提交笔触，像素/历史正常释放。

大笔触历史可用内部无损编码/系统临时文件，history_bytes 仅报告保留的内存历史计账，不包含磁盘字节或解码工作区。内存预算不是单笔大小限制；存储失败可能返回 IO_ERROR，失败的 undo/redo 不修改像素与历史栈。实现/限额见 docs/history-storage-design.md；C ABI 布局与版本未变。

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

文件任务使用独立CPU工作线程；codec单次调用不能强杀，行/瓦片/读写及提交点检查取消。cancel只对Queued/Running有效，已终结返回BUSY。同目录临时文件flush/sync后persist原子替换为提交点；提交后成功不可伪称已撤回。销毁取消作业、清空队列并后台join。格式/资源/ORA限制见docs/io-design.md；PSD/ICC/GPU等能力未报告。


## ABI 1.3 暂存盘配置

所有旧函数/DTO 布局保留。新增能力 PAINT_FEATURE_STORAGE_SETTINGS=128；PaintStorageOptions 大小 48 字节，path 偏移 8，PaintStorageInfo 大小 32 字节（64 位平台）。reserved 为 0，struct_size 规则不变。

paint_core_create_with_storage(options, out_core) 先验证和检查暂存目录，成功才生成拥有 core，失败输出 NULL。配置不可变，所有后续文档/session、新建与导入继承此配置；创建新 core 或重启应用应用新设置。旧 paint_core_create 使用默认懒初始化存储，保持原调用兼容。销毁 core 仍要求先释放文档/session/订阅；存储后备可以由后台快照延长寿命。

path_length=0 选择系统临时目录，此时不读取 path；否则绝对 OS 路径 UTF-8 1..32768 字节、禁止 NUL，目录必须存在。resident_bytes 为 65536 的倍数，范围 65536..1073741824（64KiB..1GiB）；scratch_bytes 为单文档瓦片暂存区段预算，65552..68719476736（约64KiB..64GiB）；min_free_bytes 为文件系统保留空闲阈值，0..1099511627776（1TiB）。不是整个进程内存或所有文档总磁盘上限。磁盘历史的独立 4GiB 预算和 100 条限制不变。

paint_storage_inspect(options, out_info) 不修改现有 core；在候选目录检查空间、尝试创建可写 run、检查并清理已失活的标记目录，返回可用字节、本次清理目录/字节与跳过数量。探测失败把有效 out_info 前缀清空。探测并非纯查询：可创建专用命名空间及清理符合条件的残留，不会恢复绘画；完成后删除自身探测 run。清理扫描有界，陌生内容/链接/无有效租约/锁定实例跳过，旧版本 Temp 散落文件不凭前缀删除。

两个函数只在后台线程调用；禁止回调内调用。预算/UTF-8/相对路径错误 INVALID_ARGUMENT，维护锁争用 BUSY，可重试；不可访问、不可写、标记冲突、空闲不足 IO_ERROR。可用空间为即时采样，不能预留给本进程独享；增长检查每 1MiB 或 1 秒刷新，其他进程可继续消耗磁盘，真实失败仍按原有笔触/历史事务处理。具体清扫边界见 docs/storage-settings-design.md。


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
