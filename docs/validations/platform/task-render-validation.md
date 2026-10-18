# 任务与 CPU 视口渲染验证

2026-10-08，Windows 11 x64；Rust/Cargo 1.99.0 MSVC、MSVC 19.51、CMake 4.3.1/Ninja、Qt 6.8.3 MSVC2022 x64。设计先于实现写入 task-render-design.md。正式 ABI 已兼容扩展至 1.1.0，旧 DTO/函数保留，生成头由固定 cbindgen 管理。

## 已交付

- paint-task：标准库有界优先级池、FIFO 文档执行器、协作取消、结果领取、排队闭包释放、panic 隔离与后台 join。
- paint-render：Renderer trait、只读快照、线性正常合成、稀疏 2^LOD 方块平均、透明边界、预乘 sRGB RGBA8、资源与取消检查；没有 GPU 依赖。
- paint-ffi：十个 session 导出、版本化命令/元数据/视口/帧 DTO、代际与请求 ID、完成前缀/异步错误、撤销屏障、整笔过载回滚、句柄拥有与销毁。
- QML 接入：删除 C++ 绘画 FIFO、显示编码和整张 QImage；主画布用可见区域，导航另用低优先级槽，物理输出适配 DPR。主线程只提交小 DTO；生命周期/复制在 Qt worker，模型计算在 Rust。Qt 元数据与帧接收都有限额。
- 新建每边可达 1,000,000；显示帧每槽至多 4,194,304 像素（RGBA8 16 MiB），工作像素预算独立。模型瓦片/历史仍受预算约束，不能据此宣称可填满百万边长画布。
- 按用户要求，移除“灵感随笔”自动演示浮窗；--preview 只画三笔并生成主窗截图。自定义面板与浮动功能仍由用户创建/操作。

## 验收结果

统一 `python tools/check.py` 完整通过：fmt、Clippy -D warnings、65 项 Rust 单元/集成测试、doc tests、release 构建、生成头漂移检查、四组 C11/C++20 static/shared ABI 程序、Qt Test 和两倍 DPI 场景。日志：artifacts/task-render-check.log。

| 层 | 数量 | 实际检查 |
|---|---:|---|
| paint-core | 29 | 原笔刷/图层/瓦片/历史/预算/快照回归 |
| paint-task | 5 | 优先级与等优先 FIFO、满队列、排队/运行取消、panic、关闭 |
| paint-render | 4 | 与参考合成逐像素对照、先合成后平均、跨瓦片、百万稀疏尺寸、快照/透明/显示/取消/非法输出 |
| paint-ffi 同步及 actor panic | 19 | 原 18 项、文档 actor panic 后拒绝写与帧、诊断与可释放 |
| paint-ffi 异步集成 | 8 | DTO 布局、顺序/撤销、代际/视口、UTF-8/出版一致、stride/容量/旧 ID、过载/生命周期、输入复制、BUSY 控制不取消笔触 |
| 原生 ABI | 4 程序 | C11/C++20 × static/shared；原接口及异步创建/提交/等待/视口/像素/销毁 |
| Qt Test | 6 功能场景 | 异步笔触/图层/历史、压力/取消、布局/坏配置、百万尺寸/裁剪/导航/替换/撤销、30,000 点突发回滚、QML 鼠标/tablet/缩放/平移/拖放/浮窗/退出 |

Qt 输出的 8 passed 包含 init/cleanup 两个框架项，实际功能场景为 6；两倍 DPI 单独重跑 QML 场景，3 passed 同样包含 init/cleanup。导航先切换到实际标签后检查帧，主画布输出宽度校验窗口 DPR。--preview 经真实 ABI 绘制压力渐变三笔，等待显示 revision 追上模型后抓图，再异步关闭；最新主窗 artifacts/task-render-workspace.png，已检查完整布局、笔触、图层/颜色面板及就绪时无忙碌指示器。

Windows native qwindows 实际启动通过；Qt Test 用 offscreen/software，单独部署 qoffscreen 与字体目录。测试中一条 FT_New_Face 调试输出不影响字体回退；QQmlEngine 警告列表为空。Qt scene graph 显示加速不等于 Rust GPU 引擎。

额外修复本机中文 MSVC /showIncludes 探测编码导致 Ninja 丢失头依赖的问题：CMake 直接探测编译器原始输出前缀，不硬编码语言。经干净构建检查 Ninja 主程序目标的头依赖包含 CanvasItem.h / PaintCoreClient.h，避免增量构建混用旧对象；该修复在本机已验证，其他工具链由后续 CI 检查。

## 可重复运行

在已激活 MSVC/Rust/CMake 的项目根目录：

```powershell
python tools/check.py
cmake -S . -B build/qt -G Ninja -DCMAKE_BUILD_TYPE=Release -DDRAWVERSE_BUILD_UI=ON "-DCMAKE_PREFIX_PATH=.tools/qt/6.8.3/msvc2022_64"
cmake --build build/qt --config Release
ctest --test-dir build/qt -C Release -R "^(abi_|qt_ui_)" --output-on-failure
./build/qt/bin/Release/drawverse.exe
./build/qt/bin/Release/drawverse.exe --preview artifacts/task-render-workspace.png
ninja -C build/qt -t deps ui/CMakeFiles/drawverse.dir/src/main.cpp.obj
```

Qt 场景报告保留在 build/verify-ui/qt-ui-tests.txt 与 qt-ui-highdpi.txt；项目 UI 构建也在 build/qt 保留报告。已有 SDK 可用 --qt-prefix 指定，--core-only 明确仅测核心。正常启动为空白文档，不自动添加演示浮窗。

尚未验收 macOS/Linux、MSRV、真实 tablet、多显示器硬件迁移；尚未实现 GPU、ICC/HDR、任意比例精细重采样、文件 IO、Python 插件。本轮不承诺 <8ms 或 60–120fps。下一模块为基础文件格式和异步保存，插件最后完成。
