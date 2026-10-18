# 后台Actor输入分页验收

2026-10-17，Windows。对应[设计](brush-input-paging-actor-design.md)，接续[Document验收](brush-input-paging-document-validation.md)。会话实际Move/End现已使用Document分页，公开C ABI调用方式和Qt提交方式保持；不是仅增加未调用的Rust入口。

SerialWorker新增可恢复回调，明确报告后续工作和暂存命令数；队列、执行中及Actor暂存共同计入原容量与保留取消槽。页间接收命令，空邮箱立即继续；重页结束后重设周期时间，并保证即使再次错过周期也给排队命令一次执行机会。回调不持有邮箱锁，非法暂存信用或panic隔离工作线程。

Move的完成水位在全部输入页完成后推进，End等笔尾和单历史成功后才完成并采用载色。湿混租约跨页持有；取消、New、stop或执行错误丢弃在途载色，普通单载体仍保留工具已有副色。未完成的笔尾恢复/重放不发布，其他安全中间页可正常渲染。取消只跳过所属当前笔触的Move前缀，不越过End/其他命令屏障；后续笔触的取消不能丢弃已结束的前笔。

必要结果：

- 任务调度8项通过：新增3项覆盖立即续页与重页公平性、暂存/执行中容量及保留取消槽、stop和非法信用隔离；保留5项旧回归。
- 新Actor 6项通过：5项直接检验真实Engine的完成水位、笔尾发布、屏障、取消/New及预算失败状态；1项真实Session线程连续两条大跨度输入，完整像素、两角色网格/revision与Document参考保持，并在第三笔中途stop验证租约释放及上一成功载色保留。不是所有6项都称为真实线程测试。
- 旧湿混双色Actor及旧入口保护7项通过；公开准备/交换/两笔等9项通过。公开图片用例新增准确旧修订重建后单次64px原始输入，密集三图片角色与起收笔开/关均对照完整像素、历史及双载色，无缩短输入或拆成8px轨迹。
- 队列溢出整笔回滚、Busy控制不取消活动笔触，两项旧回归通过。
- Qt最终Release两项功能通过（连同init/cleanup为4 passed）：`wetDualImageVbrFreshStoreAndReopenMatchCapturedPixels`现直接提交16.5到80.5的一次64px输入后End；当前设置/颜色在途变化捕获、图片三用途VBR新目录导入及重开像素保持。`wetDualControlsLoadBothCapturedColorsPreviewAndHistory`验证两色装载、白色预览及历史。未运行无关Qt整套或硬件手感验收。
- paint-task/paint-core/paint-ffi全目标Clippy `-D warnings`、Windows最终Release及4项C/C++ static/shared ABI通过；生成头构建检查通过，无ABI、state或VBR版本变化。

初次编译漏补测试Queue新增字段及迭代器类型，修正后通过；Qt复现替换脚本因缩进未匹配时未改文件，核对后直接替换原帮助函数为64px单输入。最后补强重页调度公平条件，并重新运行任务/Actor相关回归、Clippy、Release、Qt两项及4 ABI。没有通过改变测试终点、放宽单页限制或丢弃印章消除失败。

此阶段修复原64px输入因旧单调用8M预算而整笔回滚的问题，不承诺所有大笔刷已流畅。每页8M访问/8192印章、单原子印章及原始输入总512M访问/131072印章等限制仍保留；超出明确拒绝并回滚。更广泛密集大笔刷测量/优化及其他剩余功能见[清单](../../planning/brush-system-remaining.md)。Windows整体设备手感由用户自行验收；前述偶发参数输入根因仍未定位，跨平台后置。

本机日志：`artifacts/brush-input-paging-actor-{task,tests,legacy,public,overload,busy,clippy,release-final}.log`、`brush-input-paging-actor-qt-final.txt`及控制台日志，不入库。开发exe/DLL仅在无运行中的DrawVerse时同步。
