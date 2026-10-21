# 真实历史标签验收（2026-10-10）

设计见 [history-labels-design.md](../../document/history-labels-design.md)。ABI 1.10 提供 16 字节 PaintHistoryEntry 及 exact-publication 查询；历史列表共用 PanelContent，显示真实操作名称和工具图标。20ms 逐条跳转保持。

Rust 的 StoredCommand 在 Inline/Encoded 命令旁保存小型 HistoryAction 元数据，不改变磁盘像素命令格式。画笔和橡皮在笔触提交时记录；全选、矩形/椭圆选区、取消/反选与图层外观/结构记录按已提交操作分类。无变化、失败与取消不生成假记录。撤销和重做移动同一记录；新分支移除被替代的重做。历史数量/内存/磁盘上限裁剪最旧项时标记原始边界不可达。

深度 0 在原始边界可达时显示“初始状态”，裁剪后为 TRUNCATED，QML 不显示该边界。其余条目使用真实深度，点击后按原有逐条方式前进/回退。后台从同一 publication 枚举历史元数据，GUI 不读取 Rust 模型或磁盘像素。

验证结果：

- Rust 150 笔、3 条保留上限和真实编码历史，检查裁剪标记、画笔/橡皮标签、撤销重做与新分支；磁盘配额裁剪测试保留正确的擦除元数据。
- FFI 检查 16 字节布局、depth 偏移 8、画笔/全选/橡皮序列、旧发布 BUSY、越界 NOT_FOUND 和失败清零；C11/C++20 的 static/shared 实际调用新增查询。
- Qt 绘制 150 笔后，保留条目中不再存在初始状态，名称/图标正确；全选、擦除、撤销、重做和点击历史记录对应真实深度，无 QML 警告。
- 完整 python tools/check.py 通过：Rust fmt/clippy/tests、生成头严格字节检查、真实 Git 检出构建、4 项 C ABI、66 项普通 Qt 与 37 项两倍 DPI；6 个 CTest 全通过，64.36s。日志 artifacts/history-labels-final-check.log。
- Windows 原生专项 4 个槽加初始化/清理共 6 项通过，11.403s；覆盖真实历史列表、悬停混合预览、颜色交换与数位板事件轮廓。日志 artifacts/history-labels-native.txt。另检查原生工作区截图 artifacts/refined-ui.main.png，标签间距/背景、详情栏、HSB 箭头与共享深色控件正确。

验证环境 Qt 6.8.3 / MSVC 2022 / Windows 11。独立文档重开使用新历史；既有逐条回放、画布像素、文件格式与撤销语义保持。

## 2026-10-21 翻转名称及共享冷页历史恢复

图像菜单及自由变换纯水平/垂直反射分别记录kind39/40和中文名称，局部历史按相邻确认几何、永久历史按初末几何判断；复合变化保持自由变换。ABI1.76添加常量，DTO及state45/VBR不变，Qt最低1.76。

共享Inline/Paged历史先按Tile身份去重，冷页仅校验原编码checksum，不解码、不触碰LRU；恢复只换不可变引用，显示时再按需读取。保持两侧错误校验与失败原子性。新测试验证同一冷页在before/after只验证一次、验证不加载/淘汰像素、后续实际读取和损坏拒绝。既有桶历史失败测试仍注入不再使用的独立编码写入，改为1块驻留时注入真实PagePool写失败，像素/修订/历史回滚及恢复undo/redo保持。

固定1024密集图、256瓦片、驻留16、5次中位数，六次历史往返核心368.722→214.985ms。磁盘后备处于OS热缓存，仍有完整校验IO，不含GUI/显示或冷盘时间，不承诺任意大图瞬时切换。日志artifacts/flip-history-{before,after}.txt。

Release核心单元192通过、4手工项忽略、1项耗时mix测试过滤；公开paging3/history_storage3/transform7/liquify11通过（液化1既有手工项忽略），原生整图翻转及变换各1，严格Clippy/fmt/生成头通过。实际Qt图像/自由变换×水平/垂直四行，均反复直接历史恢复精确像素；4K选区变换五行及结束工具快捷键一行，软件/D3D11各10行通过。四ABI及完整标准Release通过，build/bin/Release/drawverse.exe更新。

首次Debug宽范围单元运行发现上述旧桶断言及新冷页测试在把受损页读热后的测试顺序错误；均修正并在Release回归通过。Debug耗时mix用例主动停止，未称全量通过。最终日志artifacts/flip-history-core-release.txt、flip-history-public.txt、flip-history-native.txt、flip-history-transform-native.txt、flip-history-final-{software,d3d11}.txt、flip-history-abi.txt及final-build/clippy/header-check。未跑跨平台、实体连续时延及16K整图。
