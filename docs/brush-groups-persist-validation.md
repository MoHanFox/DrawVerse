# 分组导入资源发布验收

2026-10-17，Windows，对应[设计](brush-groups-persist-design.md)。ImageBundle::import_into_catalog_persisted先在私有候选中完整合并，再保存所有PNG及来源，最后检查取消并一次采用目录。当前目录在任一错误时保持原样；共享内容寻址文件沿用已有临时文件/sync_all/不覆盖发布和内容哈希检查。无ABI/state/VBR变化，旧原生有组拒绝继续保留。

实际验证：

- 最终IO五目标40项通过：brush_resource 9、brush_source 8、brush_state 5、brush_wet_dual_exchange 7、vbr_image 11。新增VBR资源发布2项覆盖实际程序/图片分组包、新目录持久化及全新ResourceStore重开，state字节/组织/图片覆盖保持；位置错误、预先取消、目标目录资源预算不足在写文件前拒绝；既有同名哈希PNG损坏不覆盖、不改变选择/revision/组织及全部旧预设Arc；修复后成功导入，重复导入失败且合法资源保持。
- 既有实际ABR6/2与三图片用途/湿混双色用例改为调用真实持久化入口，包含平包与分组映射包两种分支。完整导入目录state从全新ResourceStore和来源上下文恢复，组织与state字节一致，真实两笔像素/全部双载色格点/undo与redo历史一致，原ABR字节保持。此用例已计入7项，不额外计数。
- 同一用例的有组分支增加两个实际磁盘故障：sources路径被普通文件占用（PNG已发布）、已有来源记录内容损坏（原ABR已发布）。两次均拒绝目录采用，选择/组织Arc/revision保持，阻塞文件/损坏记录不被覆盖，合法PNG/原ABR保持准确字节，没有遗留临时文件。移除测试阻塞后重试成功，实际绘画保持。已发布未引用的合法缓存保留是设计行为，不声称多文件整体回滚或安全GC已经实现。
- paint-io/paint-ffi全目标Clippy（警告视为错误）通过。Windows Release/生成头、ABI及旧Qt结果见最终记录。未扩大到无关全仓或硬件验收。

此入口保证“资源先落盘，目录后采用”，不自行保存Qt偏好或state文件，也不提供新的原生交接/Qt分组功能；后续需串联准备票据/准确来源捕获和完整重置基准。来源/资源公共缓存不能按本次失败清单直接删除，否则可能破坏另一成功导入，独立GC仍待有所有权证据后设计。

本机日志artifacts/brush-groups-persist-{io,clippy,release,qt-console,abi}.log及brush-groups-persist-qt.txt，不入库。

最终Windows Release及生成头一致性检查成功，4项C/C++ static/shared ABI通过。旧Qt两项vbrImagePackageImportsIntoFreshStoreAndExecutesAfterOriginalRemoval、wetDualImageVbrFreshStoreAndReopenMatchCapturedPixels通过（含init/cleanup为4 passed）；它们不是分组用户界面验收。同步前检测到DrawVerse正在运行，因此未替换build/qt/bin/Release下的exe/DLL、未终止用户进程；本次新构建在build/verify-ui/bin/Release。
