# DrawVerse

Rust 内核 / C++20 Qt 6.5+ UI / 独立 Python 插件进程的跨平台专业绘画软件项目。

已完成架构、paint-core、**paint-task + CPU paint-render**、**paint-ffi + cbindgen（ABI 1.8.0）**，**paint-storage**，以及 **Qt Quick / QML 绘画 UI**。QML 已接 Rust 异步文档与视口渲染；**paint-io（PNG/JPEG/WebP/OpenRaster）** 已接入后台任务；色彩/GPU 等继续逐模块实施，插件最后完成。

- [架构与工作区设计](docs/architecture.md) / [模块状态](docs/roadmap.md)
- [跨平台检出和构建一致性](docs/build-reproducibility.md)
- [FFI 方案](docs/ffi-design.md) / [正式契约](contracts/abi.md) / [生成头](ui/include/paint_api.h)
- [最新 FFI 验证](docs/ffi-validation.md) / [首轮内核历史验证](docs/validation.md)
- [QML UI 设计](docs/qml-ui-design.md) / [UI 验证与启动说明](docs/qml-ui-validation.md)
- [参考图 UI / 停靠设计](docs/reference-ui-design.md) / [菜单与材质验收](docs/reference-ui-validation.md)
- [画布与工具条自由停靠](docs/flexible-docking-design.md) / [最新工作区验收](docs/flexible-docking-validation.md)
- [多文档与面板列设计](docs/document-workspace-design.md) / [工作区验收](docs/document-workspace-validation.md)
- [图标列与临时面板修复](docs/icon-rail-design.md) / [当前交互验收](docs/icon-rail-validation.md)
- [矩形/椭圆选区设计](docs/selection-design.md) / [选区验收](docs/selection-validation.md)
- [画笔库与当前画笔设置](docs/brush-library-design.md) / [画笔模块验收](docs/brush-library-validation.md)
- [任务与 CPU 视口设计](docs/task-render-design.md) / [最新模块验证](docs/task-render-validation.md)
- [绘画性能修复设计](docs/painting-performance-design.md) / [修复前后实测](docs/painting-performance-validation.md)
- [图层控制设计](docs/layers-design.md) / [图层控制验收](docs/layers-validation.md)
- [并排蒙版、剪贴与预览修复设计](docs/clipping-design.md) / [最新验收](docs/clipping-validation.md)
- [蒙版/拖放/默认背景设计](docs/masks-design.md) / [本轮验收](docs/masks-validation.md)
- [图层组设计](docs/groups-design.md) / [组与透明背景验收](docs/groups-validation.md) / [棋盘格背景设计](docs/transparency-design.md)
- [暂存盘设置设计](docs/storage-settings-design.md) / [设置与清扫验证](docs/storage-settings-validation.md)
- [大笔触历史存储设计](docs/history-storage-design.md) / [历史存储验证](docs/history-storage-validation.md)

## 当前能力

64×64 稀疏瓦片、线性预乘 RGBA32F、多图层、16 级隔离图层组、可编辑图层/组蒙版、连续剪贴蒙版、拖放排序和归组、稀疏白色背景、27 种混合模式、可见性/不透明度/填充、透明像素锁/位置锁/完全锁、非破坏性整数移动、矩形/椭圆选区与组合/反选、压感圆笔刷、橡皮擦、距离插值、取消、图层与笔触撤销/重做、历史/瓦片预算、脏区域、只读快照。

C ABI 保留原同步文档/事件/线性读取，新增异步 session、FIFO 命令、原子元数据、四槽视口及完整 sRGB 显示帧。任务池支持有界优先级、协作取消与 panic 隔离，文档单线程独占写入；Renderer 在快照上合成和稀疏 LOD。core/task/render 禁 unsafe，core 使用已锁定的 tempfile 管理磁盘历史，caller-memory/回调 unsafe 限于 paint-ffi。cbindgen 为独立构建工具依赖，固定版本与 Cargo.lock。

大笔触不再因超过 64MiB 内存撤销预算而回滚：小命令用 Arc 快照，大命令做浮点位模式无损压缩，必要时转存配置的暂存目录，仍可整笔撤销/重做。保留磁盘历史预算 4GiB / 历史命令最多 100 条，准备新命令会暂时另占空间；淘汰/正常退出自动删除临时文件。磁盘失败会明确报错并保护旧状态。文档瓦片分页已实现：默认 4096 个驻留瓦片（像素约 256MiB）、最多 1,000,000 个逻辑瓦片、8GiB 暂存区段，冷页按需读取；快照/历史不固定整张像素于 RAM。预算不等于整个进程 RAM，格式解码/导出工作区另占空间。paint-storage 已实现路径设置、保留空闲空间检查与标记/租约锁保护的残留清扫。编辑 → 性能与暂存盘可保存配置，重启生效；默认系统临时目录，保留 512MiB 空闲。只清理专用目录内验证过的失活 run，跳过陌生内容/链接/活动实例，不是自动恢复绘画。设计和实测见 [文档分页](docs/document-paging-design.md) / [分页验收](docs/document-paging-validation.md)。

绘画性能回退已修复：源瓦片合成与显示编码按身份复用，稀疏矩形输出；QML 图层只在定义/属性变化时更新模型。固定 600 点压感场景进程累计 CPU 时间下降约 90%，画面更新间隔 p95 从 96ms 降至 17ms；这是本机固定负载实测。每视口缓存计账上限 96 MiB，四槽最多 384 MiB，另有容器开销与现有文档/帧预算。

QML 窗口提供压感圆笔/橡皮擦/移动图层、多图层、选区、撤销/重做和颜色/画笔/图层/历史/导航面板。新建/打开添加独立文档，各自保留真实 session、文件任务、历史与视口；工具面板随活动文档切换。画布只与画布组合为浮窗标签，通过主文档区上边缘归位；工具面板不能挂到画布上。全部画布浮动或关闭时工作区保留 #17191C 留空区。工具条可在工作区两侧及任意工具面板两侧停靠；垂直面板共用最上方导航栏，整体折叠为图标，内部为 1px 可拖分隔线；面板列仅保留图标折叠，接收新组继承图标状态；图标可直接拖出，临时面板可切换、侧边上下滑动并点击空白关闭。主工作区左右边缘在画布独占或无画布时仍接收工具归位。单画布浮窗只有一行标题，多个画布组合时恢复窗口标题与文档标签，标题显示该浮窗活动文档名；新建入口保留在文件菜单，文档条加号移除。关闭在面板菜单中操作。实际浮窗跟随鼠标，位置与停靠预览即时更新，Esc 还原，Ctrl/Command 暂停停靠，自己不作为目标。布局 v6 保存工具树、图标列和浮窗，旧布局迁移保留工具面板。关闭文档/退出逐个处理未保存文档，工作区布局不自动重建文档像素。数位板经 QTabletEvent 接入并抑制合成鼠标。详见 [设计](docs/document-workspace-design.md) / [验收](docs/document-workspace-validation.md)。

QML 新建每边支持 1–1,000,000 像素，只为可见区域和导航分配显示帧，每槽输出最多 4,194,304 像素（RGBA8 16 MiB），不按文档尺寸分配整张 QImage。Qt 接收队列每槽至多一张待处理帧。瓦片/历史仍有资源预算，尺寸上限不代表能无限量填满画布。当前 CPU 降采样为全局对齐的 2^LOD 方块平均；任意比例精细重采样后续优化。

像素蒙版缩略图并排位于内容预览右侧，点击缩略图切换绘画目标；Shift 点击蒙版启用/禁用。Alt/Option 点击两层之间创建/释放剪贴蒙版，也可使用 Ctrl+Alt+G（macOS Command+Option+G）和图层菜单；连续剪贴层共用同级基底，基底名称下划线、上层缩进箭头。缩略图的棋盘格只覆盖真实图像区域，边缘 LOD 不再混入画布外透明像素。

图层顶部提供混合模式、不透明度、填充和三个锁；百分比键入或拖滑条松手一次提交。图层列表按可见行创建，空闲 200ms 后后台更新原始内容缩略图；绘画时取消。V 选择移动工具，拖动松开提交一次移动，方向键 1px、Shift 10px、Esc 取消。移出画布的内容保留，移动后仍可绘画。外观、锁和移动均可撤销，OpenRaster 保存这些参数。混合采用线性 sRGB 公共公式，未实现 Photoshop 特殊八种混合的 Fill 曲线，不承诺 Photoshop 逐像素一致。

未实现自由套索/羽化/魔棒/一般变换、独立 Wintab 适配与真实硬件验收、ICC/16F/HDR/Rust GPU、PSD/自有格式、Python SDK。文件菜单已接入打开、保存/另存 OpenRaster、图片导出和取消任务；新建/打开/退出可先保存当前绘画。工具主窗与浮窗的二维分割、多文档画布标签和布局 v6 已接入；<8ms / 60–120fps 为后续性能验收目标。

文件模块支持 PNG 8/16 位导入、JPEG/WebP 导入和三格式 8 位合成导出；JPEG 使用线性白色背景，WebP 无损编码。OpenRaster 保留平面及嵌套隔离图层组、名称、顺序、可见性、不透明度及活动节点，并以 DrawVerse 层扩展 v1 保留填充、混合模式、三种锁、位置与溶解种子，保留画布外内容；标准定义的 15 种 Normal/颜色混合使用 SVG 名称互通，其他模式由扩展保存；合成预览为最终效果，但外部读者可能忽略扩展。以 8 位 sRGB 保存，不是 32F 无损原生格式。带 ICC、自定义 PNG 色彩元数据、动画、穿透组或未知混合/合成运算的文件会明确拒绝。栅格边长 ≤16,384、像素 ≤134,217,728（128M），输入 ≤256 MiB、解码器分配预算 1GiB；ORA 总节点 ≤256、嵌套 ≤16 级，图层总栅格亦受预算限制。保存先写同目录临时文件再原子替换，失败保留旧文件。详细范围与验证见 [IO 设计](docs/io-design.md) / [组验收](docs/groups-validation.md)。

新建默认一个不透明白色“背景”；新增像素层默认中文编号。蒙版选中后支持黑/白/灰绘制、显隐禁用和撤销，预览并排显示在所属节点右侧。图层/组可拖入组或拖到行间排序，组行只显示名称。主画布与导航均以白灰正方形棋盘格表示透明区域；全部隐藏时可见完整格子，不改导出 alpha。图层面板可分组、折叠、移入/移出和解组，组属性只应用一次，完全/位置锁继承；组与蒙版是不同功能。当前组为隔离模式，穿透尚未支持。

## 环境

Git（检出规则测试）、Rust >=1.85（rustfmt、clippy），CMake >=3.24，Python >=3.9（标准库验证脚本），C11/C++20 编译器及平台链接工具。首次生成器编译需下载锁定 crates，不需要全局安装 cbindgen。

UI 需要 Qt >=6.5 的 Core/Gui/Qml/Quick/QuickControls2/Test。本机实测 Qt 6.8.3 MSVC x64。Windows 可运行 `powershell -ExecutionPolicy Bypass -File tools/setup-qt.ps1` 安装项目内 SDK（Python >=3.10），不会改全局环境；也可指定已有 SDK。其他平台使用原生 Qt SDK。

Windows 使用 x64 MSVC + Windows SDK，建议 Developer PowerShell，将 Rust/CMake/Ninja 加入 PATH；本轮不混用 MSVC Rust 与 MinGW C++。macOS 用 Xcode 命令行工具，Linux 用 GCC/Clang。支持 64 位 ABI；已实测 Windows，macOS/Linux/MSRV 由后续 CI 验证。本机环境初始化见最新验证记录。

## 构建 / 运行 / 验证

```sh
# 根目录完整验证：Rust/生成头/C11+C++20 static+shared/Qt Test/两倍 DPI
# 自动识别项目内 MSVC Qt SDK；已有 Qt 可传 --qt-prefix <SDK>
python tools/check.py

# 明确只验证核心与 ABI，不代表 UI 验证
python tools/check.py --core-only

# Rust 内核及 FFI
cargo build --manifest-path core/Cargo.toml -p paint-core -p paint-ffi --release --locked

# 更新或检查正式头，检查不覆盖文件
cargo run --manifest-path core/Cargo.toml -p paint-api-gen --locked
cargo run --manifest-path core/Cargo.toml -p paint-api-gen --locked -- --check

# Qt UI 构建；在已激活 MSVC/Rust/CMake 的终端执行
cmake -S . -B build/qt -DCMAKE_BUILD_TYPE=Release -DDRAWVERSE_BUILD_UI=ON -DCMAKE_PREFIX_PATH=.tools/qt/6.8.3/msvc2022_64
cmake --build build/qt --config Release
ctest --test-dir build/qt -C Release -R "^(abi_|qt_ui_)" --output-on-failure

# Windows 启动（Qt DLL、QML 与平台插件已随构建部署）
./build/qt/bin/Release/drawverse.exe
# macOS/Linux 使用原生 SDK 与 CMake 对应输出；本轮未实测这两个平台

# 仅 ABI 调用测试
ctest --test-dir build/qt -C Release -R "^abi_" --output-on-failure

# 界面视觉验收：通过真实 ABI 画压感笔触，保存主窗截图后自动退出
./build/qt/bin/Release/drawverse.exe --preview artifacts/qml-workspace.png

# 原生绘画性能测量：真实 QML 窗口、压感输入与进程 CPU 时间
python tools/measure_painting.py build/qt/bin/Release/drawverse.exe artifacts/painting-ui.json
cargo run --manifest-path core/Cargo.toml -p paint-render --example painting_bench --release --locked
# 8K 全幅单笔 / 驻留预算 / 冷热概览 / 完整撤销重做
cargo run --manifest-path core/Cargo.toml -p paint-render --example paging_bench --release --locked

# 内核像素预览
cargo run --manifest-path core/Cargo.toml -p paint-core --example stroke_preview --release -- artifacts/stroke-preview.ppm
```

Windows 产物为 core/target/release/paint_ffi.lib（staticlib）、paint_ffi.dll 和 paint_ffi.dll.lib（动态导入库）；Linux 为 .a/.so，macOS 为 .a/.dylib。CMake 提供 drawverse_ffi_static/shared 导入 target 与平台链接依赖。ABI 程序位于 build/bin/Release/abi_c_static.exe 等路径；统一 check 构建于 build/abi/bin/Release/，可直接运行，不需要 Qt。

可传 `python tools/check.py --cargo <路径> --cmake <路径>`；rustc/链接器仍须有效 PATH。CMake 可配置 DRAWVERSE_CARGO_EXECUTABLE 与 DRAWVERSE_CARGO_TARGET_DIR。保持旧 build tree 的 generator 不变。

CLion 从仓库根目录的 `CMakeLists.txt` 加载，不能单独加载 `ui`。Windows 工具链选 Visual Studio、架构 `amd64`，C/C++ 编译器应为 `Hostx64/x64/cl.exe`；CMake profile 也须选择该工具链。UI 配置添加 `-DDRAWVERSE_BUILD_UI=ON` 与 `-DCMAKE_PREFIX_PATH=<Qt MSVC x64 SDK>`。从 MinGW/x86/ARM64 切换后执行 Reset Cache and Reload Project。中文 MSVC 在无终端 IDE 中的依赖输出由项目统一处理，见 [CLion 构建修复](docs/build-reproducibility.md)。

后续模块先设计再实现，并补对应 Rust / Qt Test / pytest；禁止空实现或假成功。暂存盘设置、图层控制、隔离图层组与透明棋盘格已完成；下一模块继续深化文档模型（选区与变换），之后色彩/GPU/PSD；插件最后完成。


选区：M 矩形，Shift+M 切换椭圆；拖动时 Shift 添加，Alt/Option 减去，Shift+Alt/Option 相交。Esc 取消拖动，Ctrl/Command+A 全选、Ctrl/Command+D 取消、Ctrl/Command+Shift+I 反选。边界使用静态黑白虚线，选区限制笔刷/擦除/蒙版，不裁剪导出。最多64个几何步骤，替换重置步骤，超限拒绝并保护当前选区；套索、羽化和魔棒继续后续实现。选区随 OpenRaster 扩展 v4 保存。
