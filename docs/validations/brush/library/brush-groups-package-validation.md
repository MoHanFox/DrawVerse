# 分组包原生交接验收

2026-10-17，Windows，对应[设计](../../../brush/library/brush-groups-package-design.md)。ABI1.55新增BRUSH_GROUP_PACKAGE（bit50）及显式package_read_groups/package_write_groups。交接为ordered-package-v1封装，包含完整package和必需organization；整体2MiB上限、严格模型/字段/参数及DFS投影检查。VBR1.24和state20不变；旧四类读写仍拒绝分组，不向旧Qt返回丢组的平JSON。

实际结果：

- IO三目标33项通过：vbr_image 12（含新交接1）、vbr_source 14、湿混双色交换7。新交接用例覆盖平/组/程序包、组织与参数规范字节、旧平Package隔离、null/未知字段/非字符串和未知模型/缺少组织/错误投影与2MiB预算拒绝。交接类型不开放未验证Deserialize构造，公开JSON入口和明确构造均校验。
- 新原生整链1项通过，包含平包与嵌套/同名/空文件夹两种分支。实际ABR6/2来源加主图片/纹理和源基线图片资源，经新目录容量探测、短输出哨兵不变/资源零安装、正式读入完整封装且当前库保持、资源来源安装、原子写回、包重开与实际绘画/undo/redo像素比较。原ABR与全部来源记录字节保持；错误平JSON写新接口拒绝且原目标keep字节保持。继而专用state20准备保留完整状态/源ID，证明交接与准备可串联。
- 旧原生保护5项及既有湿混双色公开API 3项通过。新入口没有解除四旧入口的版本/策略保护，也没有自动把包替换为当前库。
- [交接Schema](../../../../contracts/brush/library/brush-package-handoff.schema.json)复用VBR1.24预设与组织定义，经Draft202012Validator/显式本地引用注册验证；[合法例](../../../../contracts/brush/library/examples/brush-package-handoff-valid.json)与真实writer结构一致，[非法null例](../../../../contracts/brush/library/examples/brush-package-handoff-invalid.json)同时经Schema和Rust拒绝。Schema描述结构/数量，预算、投影和全局树语义仍由Rust校验。
- paint-io/paint-ffi全目标Clippy（警告视为错误）通过，生成头由paint-api-gen生成。Windows Release/4 ABI/旧Qt结果见最终记录；不扩展为跨平台或硬件验收。

容量足够后的资源安装仍使用共享内容寻址缓存，磁盘失败可能留下合法未引用文件；当前库不半采用，文件保存使用原子目标替换。该特性是包交接，不是完整Qt导入事务：整树副本ID映射、目标目录合并/准备采用以及Qt完整镜像/重置基准和分组管理仍需用户入口整合。

本机日志artifacts/brush-groups-package-{io,native,legacy,old-public,schema,header,clippy,release,qt-console,abi}.log及brush-groups-package-qt.txt，不入库。

最终Windows Release及生成头一致性检查成功。C/C++ static/shared 4项ABI通过，两种语言共同直接调用新读写符号并验证错误输出，C检查ABI55和新能力位；旧Qt的VBR图片导入/真实绘画移除原件、湿混双色VBR新库重开2项通过（含init/cleanup为4 passed），不当作Qt分组用户验收。
