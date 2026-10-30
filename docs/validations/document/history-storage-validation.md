# 大笔触历史存储验收

## 2026-10-21：长任务共享结果历史扩展

变换、整图翻转、栅格化和油漆桶复用已完成液化的 PagePool 结果引用。LayerEdit 保留图层及置入源瓦片身份，选区按实际保留的遮罩、点和索引计账。未托管、跨池或元数据超预算保留编码后备，小命令仍 Inline。测试发现常量遮罩的压缩文本远小于真实内存，已改为不序列化的保守内存计账并补回归。

验证结果（Windows x64）：

- 核心 result_history 六项通过、两项手工基准默认忽略；历史存储五项、直接历史三项、进度一项、公开变换七项和原生 deferred_transform_packets 三项通过。覆盖热引用身份、低驻留预算实际冷读、选区保持、redo 分支丢弃、置入源恢复，以及损坏 backing 后 undo/redo 的像素/历史/修订不变。
- 显式运行新的 Release 基准：600块原始及结果瓦片，共75MiB逻辑像素，64MiB历史预算、4096驻留瓦片。编码准备271.691ms、结果引用准备0.111ms；五次热物化中位275.930ms与0.129ms，引用身份保持。只测核心历史物化，不包含文档应用、actor、GUI显示或冷磁盘读取。
- 软件和D3D11各九行真实Qt用例通过：4K选区变换五行、长任务进度一行、直接历史选择一行、整图翻转一行、油漆桶一行；没有QML警告。这些用例不等于75MiB历史界面响应基准。
- paint-core/paint-ffi全部目标严格Clippy、格式、生成头、四组C/C++静态/动态ABI检查及完整Windows Release通过。ABI1.75/state45/behavior37/VBR1.0保持，无新增GPU算子；标准 build/qt/bin/Release/drawverse.exe 更新于17:16。

本机证据：artifacts/shared-results-{core-final,unit,public,native,clippy,perf,release-build,abi}.txt，以及 shared-results-{qt,workflow,bucket}-{software,d3d11}.txt。未运行全量测试、跨平台或实体持续输入；液化会话字段历史、其他命令类型、冷读取时延及完整8K/16K负载仍待。暂存容量和IO失败保护继续生效。

## 2026-10-08：原始编码历史验收

2026-10-08，Windows 11 x64 / Rust 1.99.0 MSVC release / Qt 6.8.3。ABI 保持 1.2.0，生成头无漂移，常用 build/qt 启动目录已更新。

移除的是 dab 中按整笔 touched tiles 预测 64MiB 历史后直接回滚的门槛，不是隐藏错误。小命令保留 Arc 快照；大命令采用 RGBA32F 位模式无损编码（游程或原始瓦片），超内存限额时写独占临时文件。保留磁盘历史限额 4GiB、最多 100 命令；超过预算淘汰最旧记录。临时文件使用系统临时目录，正常淘汰/退出自动删除。

全画布 release 基准一笔覆盖 1536×1536 的 576 个瓦片，每个像素 alpha=1，触碰数量超过旧门槛。撤销清空全部瓦片，重做逐像素位模式与原快照一致。

| 实测项 | 结果 |
|---|---:|
| 576 瓦片涂画耗时 | 266.848 ms |
| 收笔编码/提交历史 | 4.340 ms |
| 当前历史计账内存 | 517033 字节，约 505KiB |
| 此规则颜色场景的磁盘历史 | 0 字节（压缩已足够） |
| 整笔撤销 / 重做 | 15.789 / 15.460 ms |

这是 release 内核同步基准，使用 spacing=1 的固定圆笔网格，不代表真实数位板所有负载的耗时。Qt Test 另外通过 PaintCoreClient/C ABI 连续绘制同样的全画布网格（UI 原有 spacing=.15），验证不报错误、主线程提交不等待整笔编码、撤销/重做与最终帧正确。

复杂随机颜色场景测试使用小内存预算强制实际写入临时磁盘文件，验证大笔触、删除层、取消、undo/redo、丢弃 redo 分支和命令淘汰；存储往返保持每个 float 通道 bit 相同。错误注入验证写失败保留旧历史并回滚当前事务；截断/损坏 backing 后 undo 和 redo 均不改变画面、修订或历史栈。磁盘配额淘汰只删除最旧命令文件，释放历史自动清理文件。

完整 python tools/check.py 通过：96 个 Rust 测试；Qt Test 11 个功能项（连同初始化/清理共 13 项）；两倍 DPI、C11/C++20 static/shared 四组 ABI，fmt、clippy -D warnings、文档测试与生成头检查通过。新增 history_bench 示例另经 clippy 全目标检查和实际运行。build/qt 再构建并通过同样六组 CTest。

绘画性能复测维持上一轮改善：同一原生 QML 窗口、DPR 1.5、1529×1019 物理帧、600 点压感输入，累计进程 CPU 时间 390.625ms，画面更新间隔 p95=17ms，收笔追赶=16ms，错误为空。上一轮修复后为 CPU 406.25ms / p95 17ms，未出现可见回退。1800×1200 内核 120 帧基准平均 4.947ms / p95 5.265ms。基准没有与编译或其他测试并行运行，不承诺所有设备/复杂画布的帧率。

```sh
# 已激活平台编译器、Rust、Qt/CMake 的项目根目录
python tools/check.py
cmake --build build/qt --config Release
ctest --test-dir build/qt -C Release -R "^(abi_|qt_ui_)" --output-on-failure
cargo run --manifest-path core/Cargo.toml -p paint-core --example history_bench --release --locked
python tools/measure_painting.py build/qt/bin/Release/drawverse.exe artifacts/history-painting-ui.json
./build/qt/bin/Release/drawverse.exe
```

本机记录：artifacts/history-check.log、history-native-tests.log、history-fill-bench.txt、history-painting-ui.json、history-painting-render.txt。这些文件为忽略的本机验证输出。

边界：live 文档仍默认 4096 个瓦片（约 256MiB 像素）且历史解码/进行中 before 会暂占额外 RAM，外部快照也需及时释放。256MiB live 容量对应单层完全填满约 4096×4096，多图层共享预算；并非百万尺寸文档能无限填满。更大真实文档下一步需要瓦片分页/加载调度和可配置暂存盘。磁盘满/权限失败仍会返回明确 IO 错误，内核先保护完整旧状态；异常终止后的临时文件清扫尚待完善。当前历史为会话内易失数据，不提供崩溃恢复。

## 2026-10-21 翻转名称及共享冷页历史恢复

图像菜单及自由变换纯水平/垂直反射分别记录kind39/40和中文名称，局部历史按相邻确认几何、永久历史按初末几何判断；复合变化保持自由变换。ABI1.76添加常量，DTO及state45/VBR不变，Qt最低1.76。

共享Inline/Paged历史先按Tile身份去重，冷页仅校验原编码checksum，不解码、不触碰LRU；恢复只换不可变引用，显示时再按需读取。保持两侧错误校验与失败原子性。新测试验证同一冷页在before/after只验证一次、验证不加载/淘汰像素、后续实际读取和损坏拒绝。既有桶历史失败测试仍注入不再使用的独立编码写入，改为1块驻留时注入真实PagePool写失败，像素/修订/历史回滚及恢复undo/redo保持。

固定1024密集图、256瓦片、驻留16、5次中位数，六次历史往返核心368.722→214.985ms。磁盘后备处于OS热缓存，仍有完整校验IO，不含GUI/显示或冷盘时间，不承诺任意大图瞬时切换。日志artifacts/flip-history-{before,after}.txt。

Release核心单元192通过、4手工项忽略、1项耗时mix测试过滤；公开paging3/history_storage3/transform7/liquify11通过（液化1既有手工项忽略），原生整图翻转及变换各1，严格Clippy/fmt/生成头通过。实际Qt图像/自由变换×水平/垂直四行，均反复直接历史恢复精确像素；4K选区变换五行及结束工具快捷键一行，软件/D3D11各10行通过。四ABI及完整标准Release通过，build/bin/Release/drawverse.exe更新。

首次Debug宽范围单元运行发现上述旧桶断言及新冷页测试在把受损页读热后的测试顺序错误；均修正并在Release回归通过。Debug耗时mix用例主动停止，未称全量通过。最终日志artifacts/flip-history-core-release.txt、flip-history-public.txt、flip-history-native.txt、flip-history-transform-native.txt、flip-history-final-{software,d3d11}.txt、flip-history-abi.txt及final-build/clippy/header-check。未跑跨平台、实体连续时延及16K整图。


## 2026-10-22 单次跳转复用冷页校验

新增4项覆盖：两个独立结果命令共享冷页只读取一次且不改变驻留数，下次请求重新读取并拒绝损坏；临时身份表满时仍校验/拒绝损坏；真实512px密集层四次局部清除形成分页结果，六次直接往返与逐步undo/redo逐瓦片相同且保留redo；成功往返后损坏冷页，下次跳转失败完整回滚引用/选区/修订/dirty/历史。

Release同机样本，驻留1瓦片、64块密集源、四条共享结果、六次跳转：独立逐步校验1554次读/141.698ms，单请求共享402次读/37.146ms。这里只是核心恢复和OS热后备文件，非真实UI/物理冷盘，不代表8K/16K时延。`artifacts/history-transaction-release-core.txt`。Debug同样读取数，计时不用于产品结论。

Release history定向65通过/4手工忽略（过滤已知mix性能项），公开history_storage3/paging3、原生历史元数据1、严格Clippy/fmt通过。首次Debug宽匹配误包含已知耗时mix用例，核对本人进程路径/PID/参数后停止，仅以上最终定向结果为验收，未声称全量通过。日志 `history-transaction-{core,travel,shared-final,public,native,clippy}.txt`。软件/D3D11直接历史三作用域及翻转四行、选区复制清除、鼠标/笔编辑结束回归通过，完整Release/四ABI/生成头通过；后端与构建日志 `history-transaction-{software,d3d11,workflow-software,workflow-d3d11,release,abi}.txt`。标准产物build/bin/Release/drawverse.exe；ABI1.79/state45不变。会话字段引用、Encoded恢复与真实冷盘/首次显示继续H1待办。


## 2026-10-30：统一共享结果存储入口

本地 PhotoCraft ops/history_select_cmds 只读参考，未复制代码。实现范围和架构取舍见
`docs/document/history-storage-design.md` 同日节。结构命令仍走既有编码；本轮不是全快照迁移。

- `cargo test --manifest-path core/Cargo.toml -p paint-core --release --locked --lib history`：88通过、5原忽略、163过滤，日志 `build/history-shared-core-validation.log`。
- 新增 shared_stroke_history_tests 两项：100次小笔触在最低历史预算保留、跳转与分支正确；画笔/橡皮拒绝历史编码写入时仍成功，重做复用 Arc 身份。
- history_storage/paging/storage 集成：3+3+2通过；公开 async_session/history_select_api：1通过。
- paint-core all-targets 严格 Clippy、fmt、diff whitespace、生成头及四ABI检查通过。
- 原编码故障注入测试改为显式选择编码后备，保留写失败/损坏回滚断言；原驻留/冷页损坏测试跟随新的 Paged 路径继续检验。

已有 history_bench，Release同工作树改动前：1536²/576瓦片，commit 6.615ms、undo 18.316ms、redo 14.258ms，历史内存18577字节。
最终复测：commit 6.205ms、undo 0.240ms、redo 0.123ms，历史元数据308864字节；首轮实现另测undo/redo 0.171/0.087ms。
基线单进程，最终构建存在并发负载；仅核心调用、OS缓存条件，不是屏幕延迟、PhotoCraft配对或冷物理磁盘结论。
引用表高于该易压缩例的旧历史体积；像素驻留和冷备份归PagePool预算，不是全进程内存上限。

初始回归因旧“必须Encoded/Inline”的后端断言及故障测试未显式选编码而失败；调整测试前提后原像素/回滚断言通过。
首次Release UI构建缺MSVC include环境而C1083 type_traits失败，已改用VsDevCmd环境重建。
未跑总check，未新增实体输入性能结论。

标准 `build/bin/Release/drawverse.exe` 已重新链接并部署，独立profile的实际D3D11 `--preview` 三笔绘画流程退出0，生成 `build/history-review-smoke.png`；此流程不是历史响应性能量测。

重建 qt_ui_tests 后，`textFieldsKeepSpaceAndHistorySelectsDirectly` 在 offscreen/software 和实际D3D11各通过1功能（各含初始化/清理共3通过），日志 `build/history-ui-software.log` / `build/history-ui-d3d11.log`；验证四笔历史点击回到0、直接恢复4、每次revision仅+1、无中间深度发布及redo保留。软件行有既有字体FT_New_Face调试输出，功能通过。


## 2026-10-30：实际原图历史与显示成本分离

用户4096×2160PNG的核心完整像素往返校验、驻留4096/8192对照，以及真实D3D11历史节点采用、
近期源传输/GPU缓存复用和旋转尺寸交接，统一见
`docs/validations/platform/general-gpu-composition-validation.md` 的“实际4K历史往返缓存与尺寸交接”。
核心冷页校验与显示瓦片重新上传是不同开销；本轮未提高默认驻留预算或跳过损坏校验。
