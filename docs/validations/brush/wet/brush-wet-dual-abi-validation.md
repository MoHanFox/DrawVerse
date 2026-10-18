# 湿混双色原生入口验收

2026-10-16，Windows，ABI1.52，WET_DUAL_COLOR bit47。对应[设计](../../../brush/wet/brush-wet-dual-abi-design.md)。state19/VBR1.23沿用保存阶段，不新增格式版本。

增加明确的完整准备/包读写三入口、80字节双角色info及复用前景/背景描述符的原子reload，共5个导出。准备支持等级保留到图片修订重建；旧Legacy/Wet/Taper入口仍拒绝新策略，包括关闭值。既有prepared笔触完整捕获，经已有Engine/Document双载色执行。手动与自动装载共用核心标称尺寸映射，普通湿混只装主载体而保留未使用副载体，clean清洗两个角色。旧reload对新策略明确Unsupported。

最小相关验证：

- `paint-ffi --test async_session public_` 共9项通过（新增3、旧湿混3/起收笔3）。新真实绘画用例覆盖起收笔开/关、湿混开/关及配对/独立路径8组合；两笔逐次对照Document完整图层剪贴字节、主副载色revision/中心颜色、单历史与undo/redo。起笔后修改目录、第二笔销毁library令牌仍保持准确捕获。
- 新手动接口用例验证80字节及字段偏移、能力位、相对固定宽度的临时20/24px主副尺寸与两色、独立6px副笔及前景复用；活动Busy清零完整info、取消不采用、旧reload拒绝、普通载色保留副色、关闭策略持久但不可加载、无效半径/背景/空指针/票据、序列输出清零、clean及无效句柄输出清零。初次测试将销毁后的空句柄误写为INVALID_ARGUMENT，按既有注册表契约更正断言为INVALID_HANDLE；实现行为未改。
- 新包用例验证同一PNG用于主笔尖/第二笔尖/纹理，读容量探测和无效输出不安装图片、完整读取安装且JSON准确、错误保存保留原文件、准确旧图片票据退休后重建仍保留state19策略与三资源身份。完整ABR来源字节保留由前一IO阶段验收证明，本轮未重复其语料测试。
- `paint-core --lib wet_dual_` 10项通过，覆盖实际像素、独立网格预算、起收笔重放和失败回滚，确认共享尺寸计算保持原执行结果。
- `paint-ffi --lib wet_dual_` 7项通过：旧三类入口拒绝保护1项、原Engine双载色事务6项（包括跨会话Busy/目标失败/取消/New/stop/预算失败/单载体保留副色）。本轮未复制Engine算法。
- paint-core/paint-ffi全目标Clippy `-D warnings`通过；生成头、Windows Release及生成头字节一致性通过。
- 4项C/C++ static/shared ABI通过，验证80字节/字段偏移及5个新导出的真实链接调用；旧40字节info与32字节load布局保持。
- Qt6.8.3 windows、software Quick、环境倍率1：`wetPolicyPersistsSharesAcrossDocumentsAndImportsFromVbr`、`wetSmudgeUsesOwnToolStrengthAndFingerPolicyWithUndo`通过，含初始化清理4 passed/0 failed。仅证明旧界面兼容；Qt仍用旧入口并拒绝湿混双色组合。此前湿混设置页键盘输入偶发失败未定位，本轮未修改该输入路径或宣称修复。

本机日志：`artifacts/brush-wet-dual-abi-{check,public,core,actor,clippy,header,release,smoke}.log`及`artifacts/brush-wet-dual-abi-qt.txt`，不入库。未运行无关全量或跨平台验收。

后续：Qt完整state19镜像/关闭值/重置基准、准备及库包入口、双角色载色显示与装载、白色正常模式真实预览和设置组合。湿混双色涂抹/橡皮、非分隔、高级ABR/16位及VBR分组等完整系统目标继续后续。
