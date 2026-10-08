# Qt Quick / QML UI 验证

此文件保留首版同步 ABI 适配的历史验收。现已接 Rust 任务与 CPU 视口、ABI 1.1 和大画布；最新验证见 [task-render-validation.md](task-render-validation.md)，当前设计见 [qml-ui-design.md](qml-ui-design.md)。

2026-10-08，Windows 11 x64，Rust/Cargo 1.99.0 MSVC、MSVC 19.51、CMake 4.3.1/Ninja、Qt 6.8.3 MSVC2022 x64（项目内 `.tools/qt/6.8.3/msvc2022_64`）。生产 Qt/C++ 代码要求 C++20，编译启用 /W4 /WX /utf-8。本轮没有改变 Rust ABI、内核或生成头。

## 模块

- PaintCoreClient / BackendWorker：只在 worker 调用稳定 C ABI，FIFO 顺序、ABI/能力校验、显示转换、错误回传、关闭撤订阅与句柄释放。队列压力过大回滚整笔，不提交缺点笔触。
- CanvasItem：QQuickItem/scene graph 纹理显示，纹理拥有权留给 node；缓存未改变时仅更新位置。鼠标/数位板映射统一，左键绘画、空格/中键平移、指针锚定缩放。
- WorkspaceManager / QML：标签组/左右列分割/原生 MIME 跨窗口拖放/单面板和整组浮动、关闭归回、版本化布局校验、自定义笔记/色板/画笔面板。

## 已执行验收

Qt Test 五个实际场景：后台绘画/图层/撤销/重做/关闭；压力粗细/整笔取消/尺寸拒绝；工作区序列化/浮窗越界修正/合并/坏配置；输入队列过载回滚；真实 QML 加载/鼠标与模拟 tablet/指针缩放与平移/浮窗与真实 MIME DropEvent/关闭归回/应用退出。最后场景另在 QT_SCALE_FACTOR=2 下运行。QML 加载与操作没有 QQmlEngine 警告。

Windows 测试用 offscreen + software scene graph，平台 DLL 独立部署，测试字体使用 Windows 已安装字体。实际应用默认使用 Windows 平台插件；不能把测试 offscreen 环境变量设置给普通启动。已修正此前仅部署 qwindows 导致 offscreen 无法初始化的问题，并补了独立部署 qoffscreen 的构建步骤。软件帧更新回归捕获并修复了 node 拥有纹理时重复释放旧纹理的问题。

完整入口 `python tools/check.py` 验证 Rust 47 项单元/集成测试、fmt、Clippy、doc tests、release 构建、cbindgen 漂移检查、C11/C++20 static/shared 四个 ABI 程序、Qt Test 与两倍 DPI 场景。日志保存在本机忽略目录 `artifacts/qml-full-check.log`，Qt 逐场景报告在 `build/verify-ui/qt-ui-tests.txt` 和 `qt-ui-highdpi.txt`。

真实 Windows 窗口通过 `--preview` 模式以同一 PaintCoreClient 画三笔压力渐变曲线，保存 `artifacts/qml-workspace.png` 与 `artifacts/qml-workspace-floating.png`，并完成异步关闭。截图检查主窗内容、画布、图层/颜色面板以及浮窗笔记的对比度；默认启动仍是空白文档。

## 运行与测试

从项目根目录，在配置好 MSVC/Rust/CMake PATH 的 Developer PowerShell 中：

```powershell
# 首次可选，或者使用已有 Qt SDK
powershell -ExecutionPolicy Bypass -File tools/setup-qt.ps1
cmake -S . -B build/qt -G Ninja -DCMAKE_BUILD_TYPE=Release -DDRAWVERSE_BUILD_UI=ON -DCMAKE_PREFIX_PATH=.tools/qt/6.8.3/msvc2022_64
cmake --build build/qt --config Release
./build/qt/bin/Release/drawverse.exe
ctest --test-dir build/qt -C Release -R "^(abi_|qt_ui_)" --output-on-failure
python tools/check.py
```

`python tools/check.py --qt-prefix <SDK>` 可指定已有 Qt，`--core-only` 只验证核心。CMake 不启用 UI 时不要求 Qt。Qt Test 启动失败时逐场景文本会保留在 build 目录，CTest 返回失败，不跳过它。Windows build 自动部署 Qt runtime/QML/qwindows/qoffscreen；这些是开发阶段本地启动目录，正式安装器/签名/许可证分发在打包模块验收。

## 尚未验证或实现

实际数位板硬件、Windows Ink/Wintab 切换、macOS/Linux、多屏硬件迁移仍未完成验收。Qt 默认 scene graph 呈现不代表 Rust GPU 引擎；本轮未宣称 <8ms 或 60–120fps 已达标。ICC/HDR、Rust task/render、文件保存/格式、任意嵌套分割树、SplitView 分隔比例持久化、Python 插件继续按模块实施。
