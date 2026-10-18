# Document真实输入分页验收

2026-10-17，Windows。对应[设计](brush-input-paging-document-design.md)，前置[数值调度](brush-input-paging-kernel-validation.md)。本轮完成Rust Document明确入口；Actor和公开C ABI笔触仍走旧同步入口，当前Qt大跨度输入问题尚未宣称修复。

新增begin_stroke_input / continue_stroke_page / begin_stroke_finish和StrokePageResult。原始滤波一次捕获，主副调度事件直接执行既有像素算子；结束不能越过输入和尾部重放。输入分页不创建历史，完整结束才提交单历史并返回完整WetInk，包括无像素修改但已拾色的载体。取消或任意执行/存储错误清空在途状态，恢复整笔before。

起收笔准备、重放页和采用分离。保留原活动尾日记和检查点，一次恢复后按页重放；索引获取TaperUpdate因素为O(1)，不按每个事件扫描整尾。主副RGBA、几何场、随机和载色均用同一原算子。旧同步渲染也复用这个执行器，但保留原累计8M/8192预算。新执行页继续受同一页预算限制，总调度和尾部重放累计512M/131072预算；每页先收窄到剩余额度。

最小相关结果：

- `paint-core --lib input_paging_document_` 新13项通过。24组湿混开/关、双色开/关、起收笔开/关及页事件上限3/19/8192，逐输入对照旧同步真实像素，结束后主副完整采样网格/revision/几何、单历史及undo/redo保持。
- 原64px密集图片湿混双色在明确新入口完成，旧同步入口仍明确拒绝并回滚。8组图片/纹理、配对或独立双流、起收笔开/关、页大小5/8192，与不使用活动尾重放/待执行输入的已知终点一次性参考比较，完整像素和两个载色保持。参考共享已独立验过的原始段调度及原写入算子，独立取消活动尾而使用已知终点因素，不能把它说成另一套Photoshop引擎。
- 时间平滑/压力/旋转跨角度/缺失设备能力/散布，以及原地压力主副单事件页通过；无采样输出的subtick输入只采用滤波状态、不重放、不修改可见revision，后续实际笔触保持。
- 擦除、湿混涂抹、蒙版、椭圆AA选区、偏移和透明锁、低驻留分页组合与原同步像素保持。不同目标不绕过现有取样限制。
- Busy保护、输入与结束不能越过未完成页、早期或重放中Cancel、非法坐标、缩小页预算、瓦片限额、暂存写入和历史提交失败均恢复整笔像素及历史；无在途输入残留或假成功载色。每个待执行页观测工作增量在预算内；总预算包含尾重放，耗尽后失败回滚。零不透明度仍拾色并返回完整双载色，不生成伪历史。
- `--lib input_page` 10项通过（前9项加关闭独立副流正无穷哨兵1项）；接入时发现现有Document使用该哨兵，调度器仅在副流关闭时保留接受，活动副流仍拒绝。旧14400组预算分类保持。
- `--lib taper` 12项通过（包括新增分页长输入1项，其余相关旧11项），`--lib wet_dual_` 10项通过；筛选间有交叉，不能相加说成全部互不重复测试。
- paint-ffi旧Actor/拒绝保护7项及公开API9项通过，检查旧会话和准备/包/实际两笔像素、历史及工具所有权兼容。新Actor路径仍待接。
- paint-core全目标Clippy `-D warnings`、Windows Release、cbindgen头字节一致性、4项C/C++ static/shared ABI通过。无C ABI、state或VBR版本变化。没有运行无关Qt整套、跨平台或整体设备手感验收，后者由用户进行。

初始检查有两处API类型/连接问题：压力变化检查传参误用InputPoint而非f32 pressure导致编译失败；未兼容关闭副流的正无穷哨兵导致首轮像素测试准备失败。均修正并新增哨兵回归。Clippy指出枚举尺寸悬殊，改为Box持有调度器。未通过放宽像素预算或缩短64px原始输入消除测试失败。

本机日志`artifacts/brush-input-paging-document-{check,tests,kernel,legacy-taper,legacy-wet,clippy,actor,public,release,abi}.log`，不入库。下一步：Actor接入分页、页间让出执行和渲染发布、完成序列/Move/End顺序、Cancel/New/stop及共享WetTool原子采用，最后通过公开API和Qt真实64px复现才能解除用户入口限制。
