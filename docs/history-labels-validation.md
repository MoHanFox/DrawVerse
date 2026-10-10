# 真实历史标签验收（2026-10-10）

设计见 [history-labels-design.md](history-labels-design.md)。ABI 1.10 提供 16 字节 PaintHistoryEntry 及 exact-publication 查询；历史列表共用 PanelContent，显示真实操作名称和工具图标。20ms 逐条跳转保持。

Rust 的 StoredCommand 在 Inline/Encoded 命令旁保存小型 HistoryAction 元数据，不改变磁盘像素命令格式。画笔和橡皮在笔触提交时记录；全选、矩形/椭圆选区、取消/反选与图层外观/结构记录按已提交操作分类。无变化、失败与取消不生成假记录。撤销和重做移动同一记录；新分支移除被替代的重做。历史数量/内存/磁盘上限裁剪最旧项时标记原始边界不可达。

深度 0 在原始边界可达时显示“初始状态”，裁剪后为 TRUNCATED，QML 不显示该边界。其余条目使用真实深度，点击后按原有逐条方式前进/回退。后台从同一 publication 枚举历史元数据，GUI 不读取 Rust 模型或磁盘像素。

验证结果：

- Rust 150 笔、3 条保留上限和真实编码历史，检查裁剪标记、画笔/橡皮标签、撤销重做与新分支；磁盘配额裁剪测试保留正确的擦除元数据。
- FFI 检查 16 字节布局、depth 偏移 8、画笔/全选/橡皮序列、旧发布 BUSY、越界 NOT_FOUND 和失败清零；C11/C++20 的 static/shared 实际调用新增查询。
- Qt 绘制 150 笔后，保留条目中不再存在初始状态，名称/图标正确；全选、擦除、撤销、重做和点击历史记录对应真实深度，无 QML 警告。
- 完整 python tools/check.py 通过：Rust fmt/clippy/tests、生成头严格字节检查、真实 Git 检出构建、4 项 C ABI、66 项普通 Qt 与 37 项两倍 DPI；6 个 CTest 全通过，64.36s。日志 artifacts/history-labels-final-check.log。
- Windows 原生专项 4 个槽加初始化/清理共 6 项通过，11.403s；覆盖真实历史列表、悬停混合预览、颜色交换与数位板事件轮廓。日志 artifacts/history-labels-native.txt。另检查原生工作区截图 artifacts/refined-ui.main.png，标签间距/背景、详情栏、HSB 箭头与共享深色控件正确。

验证环境 Qt 6.8.3 / MSVC 2022 / Windows 11。独立文档重开使用新历史；既有逐条回放、画布像素、文件格式与撤销语义保持。
