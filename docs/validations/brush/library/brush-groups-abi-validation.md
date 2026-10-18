# 分组状态原生准备验收

2026-10-17，Windows，对应[设计](../../../brush/library/brush-groups-abi-design.md)。ABI1.54新增PAINT_FEATURE_BRUSH_GROUP_STATE（bit49）和paint_brush_library_prepare_state_groups，允许完整state20及旧参数。共用Recipe/准确票据/来源凭据/冷重建/current与mutate采用；旧四类准备仍拒绝有组状态，四旧包读写保护不解除。state20/VBR1.24不变。

实际结果：

- 新公开API 2项通过：嵌套/空组/相同名称与完整关闭湿混双色/起收笔参数，prepared_state准确字节；切换当前目录到另一平状态后持有分组票据，实际两笔普通/湿混双色像素、双载色中心/修订、单历史与undo/redo与Rust参考一致。版本54/bit49真实查询，空JSON指针和非法句柄清零sequence且有效旧票据保持。
- 新原生准备1项通过（启用/关闭两组）：实际ABR6/2导入的图片/来源，主/副/纹理共享图片与关闭参数；准确state20字节和来源凭据，mutate先采用准备状态再改组名，旧快照组名保持。未知模型、降级version19和坏JSON失败保持当前；清退图片票据缓存后，按原Groups支持级别冷重建仍恢复完整树/资源/源ID，原ABR字节保持。
- 旧保护5项通过：四种旧准备拒绝有组/旧策略，旧包入口拒绝分组且不装资源，旧湿混/起收笔/双色策略保护保持；原有湿混双色公开API 3项通过。
- paint-ffi全目标Clippy（警告视为错误）通过，生成头由paint-api-gen生成且Release检查一致；Windows Release构建成功。C/C++ static/shared 4项ABI通过，共用smoke直接链接并调用新符号验证参数拒绝/sequence清零，C检查54与新能力位。不是仅用旧符号测试新导出。
- 旧Qt 2项包/图片实际绘画重开回归通过（init/cleanup计为4 passed），不冒充分组Qt验收。跨平台与整体数位板手感后置。

首轮新公开用例误把点字段放到PaintCommand顶层导致编译失败，改用已有PaintPoint DTO后两项通过；未修改生产输入语义或缩减像素/历史验收。

本机日志artifacts/brush-groups-abi-{public,native,legacy,old-public,header,clippy,release,release-final,qt-console,abi}.log及brush-groups-abi-qt.txt，不入库。后续仍须专用VBR分组包交接、原生组织操作及Qt完整镜像/管理/重置基准，能力名只声明GROUP_STATE。

同步前确认DrawVerse未运行，开发目录exe/DLL已同步为验证构建并核对哈希。
