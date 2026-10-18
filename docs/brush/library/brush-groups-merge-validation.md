# 分组目录合并验收

2026-10-17，Windows，对应[设计](brush-groups-merge-design.md)。完整有序子树可插入现有目录根或指定组位置，组/预设跨角色冲突、投影不一致、合并后容量/深度和资源预算均在候选采用前拒绝。一次成功只推进一次revision；首个新预设选中，纯空文件夹保留原选择，完整空输入是校验目标位置后的无变化操作。旧平批量导入/同组复制复用同一事务。

实际结果：

- 新核心4项、既有分组6项和目录5项通过。新用例覆盖根/组内连续插入与DFS、同名/空文件夹、旧组织及预设Arc保持、在途笔触/像素及历史、空输入无变化、未知目标/位置、重复预设/组和双向跨角色冲突、输入预设顺序不符、合并深度超过8、组数超过256、预设超过1024、资源预算失败。失败检查revision/选择/树Arc与所有已有预设Arc保持。
- 修订耗尽单元1项通过：新增纯空文件夹合并在u64::MAX拒绝，完整空输入仍可无变化成功；已有修改失败保护保持。
- IO三目标21项通过：state 5、湿混双色交换7、VBR图片9（含新增真实包合并1）。实际程序/图片分组包经encode/decode后合并，保留主图片Arc及已有预设Arc；重复导入、取消、位置错误保持目标身份。既有三图片/真实ABR来源用例增加实际合并，验证源ID、分组和完整执行设置，真实两笔像素、全部双载色格点与历史与包定义一致；平包分支同样经过合并，不只验证新分组路径。
- paint-core/paint-io/paint-ffi全目标Clippy（警告视为错误）通过。Windows Release/ABI及旧Qt结果在最终记录补充。无ABI/state/VBR版本变化，旧原生保护继续保留。

首轮新像素用例误用了不存在的composite_rgba8方法，改用实际逐像素和历史API；修订边界用例变量遮蔽导致编译失败，改名后单项及全目标Clippy通过。未隐藏这些编译失败；生产逻辑没有因失败缩减验收要求。

这是Core与VBR的内存导入事务，不是完整用户文件导入。资源/来源耐久文件的暂存与失败清理、完整原生交接/Qt分组管理和重置基准全链仍待后续；跨平台及整体数位板手感由用户后续验收。

本机日志artifacts/brush-groups-merge-{core,revision,io,clippy,release,qt-console,abi}.log及brush-groups-merge-qt.txt，不入库。

最终Windows Release及生成头一致性检查成功，4项C/C++ static/shared ABI通过。两项既有Qt回归vbrImagePackageImportsIntoFreshStoreAndExecutesAfterOriginalRemoval、wetDualImageVbrFreshStoreAndReopenMatchCapturedPixels通过（含init/cleanup为4 passed）。它们验证既有用户路径保持，不作为分组Qt支持的证据。
