+# DrawVerse

> **使用警告：当前为开发中的实验版本，请勿作为重要作品的唯一存储工具，使用前请备份并及时手动保存。**
> 部分交互测试仍未通过；暂存文件和撤销历史不等同于自动保存或崩溃恢复，异常退出可能造成未保存内容丢失。

DrawVerse 是一款正在开发中的桌面绘画软件。Rust 内核负责笔触、文档、历史、文件交换和 CPU 合成，C++ / Qt Quick / QML 负责界面与输入，两者通过 C ABI 连接。

目前主要在 Windows x64 上开发和验证。`master` 仍处于持续开发阶段，功能边界和实际验证范围以对应验收记录为准。macOS / Linux、GPU 绘画、完整色彩管理、PSD 和 Python 插件仍待推进。

[文档索引](docs/README.md) · [验收记录](docs/validations/README.md) · [当前契约](contracts/README.md) · [代码结构](docs/platform/source-layout-design.md)

## 当前功能

| 领域       | 已有能力                                                                                                     |
|------------|--------------------------------------------------------------------------------------------------------------|
| 绘画与笔刷 | 程序与图片笔尖、压感、独立流量与不透明度、形状与散布动态、纹理、双重笔刷、平滑、起收笔、湿混、涂抹与时间喷枪 |
| 画笔库     | 预设与分组管理、搜索、多选、拖放排序、ABR 导入、VBR 1.0 导入导出、资源与来源记录保存                         |
| 工具       | 画笔、橡皮、吸色、矩形与椭圆选区、自由与多边形套索、魔棒、油漆桶、自由变换和液化                             |
| 图层与历史 | 图层组、蒙版、剪贴、27 种混合模式、锁定、批量操作、撤销与重做，以及瓦片分页和磁盘历史                        |
| 工作区     | 多文档、画布与工具面板停靠、浮动窗口、布局恢复、导航与偏好设置                                               |
| 文件交换   | PNG / JPEG / WebP 图片读写，OpenRaster 文档读写，图片置入与导出                                              |

ABR 导入按实际支持范围转换，部分 Photoshop 行为采用近似并保留来源报告，不代表完全等价。当前支持范围见 [笔刷剩余功能与边界](docs/planning/brush-system-remaining.md)。

## 项目结构

```text
DrawVerse/
├─ core/
│  ├─ crates/          Rust 内核、任务、渲染、存储、IO 与 FFI
│  └─ tools/           C ABI 头文件生成工具
├─ ui/
│  ├─ src/             按功能组织的 C++ 界面与核心桥接
│  ├─ qml/             应用装配、功能面板和公共组件
│  ├─ include/         生成的 C ABI 头文件
│  └─ tests/           Qt 界面与交互测试
├─ contracts/          当前格式与接口契约
├─ docs/               按领域组织的设计与说明
│  └─ validations/     按领域分类的验收记录
├─ tests/              原生 ABI 与工程工具测试
└─ tools/              构建检查、环境安装与性能测量脚本
```

| **Rust crate** | **职责** |
| --- | --- |
| `paint-ops` | 可复用数值算子、CPU 执行器、采样与批次规划，以及复刻描边等纯计算；不管理文档或历史 |
| `paint-gpu` | wgpu 设备选择、受支持算子的 GPU 执行与 CPU 后备；CPU 数据保持权威 |
| `paint-core` | 文档、笔刷执行、图层、蒙版、选区、变换、液化、瓦片分页与历史事务 |
| `paint-task` | 有界并行与串行任务调度、优先级、协作取消及进度 |
| `paint-render` | 文档快照合成、CPU 视口取样与显示帧生成 |
| `paint-storage` | 暂存目录创建、容量检查、受管临时文件、租约与过期数据清理 |
| `paint-io` | 图片、OpenRaster、ABR、VBR、笔刷状态及资源与来源数据编解码 |
| `paint-ffi` | C ABI、句柄、异步会话、命令与显示数据发布，以及调用方内存边界 |
| `paint-api-gen` | 从 Rust ABI 定义生成 C 头文件，并检查生成头一致性（构建工具） |

文档计算和大块像素处理在 Rust 后台执行；UI 通过会话提交命令、接收元数据和显示帧。目录职责与依赖方向见 [源码结构设计](docs/platform/source-layout-design.md)。

## 构建与运行

需要 Git、Rust（含 rustfmt 和 clippy）、CMake 3.24+、Python 3.9+、C11 / C++20 编译器及 64 位链接工具。界面需要 Qt 6.5+ 的 Core、Gui、Qml、Quick、QuickControls2 和 Test 模块。

Windows 使用 x64 MSVC 与 Windows SDK，在 Developer PowerShell 中运行以下命令，并确保 Rust 和 CMake 已加入 PATH。项目内 Qt 安装脚本需要 Python 3.10+；本机验证使用 Qt 6.8.3 MSVC x64。

Windows 默认在构建期预编译共享移动/液化材质，需要 Developer Shell 的 Windows SDK `fxc` 在 PATH 中；构建会检查生成的 DXBC，缺少工具或预编译失败会报错。可在配置命令中显式加入 `-DDRAWVERSE_PRECOMPILE_MOVE_SHADERS=OFF` 使用原运行时 HLSL 编译路径。其他 shader 和非 Windows 目标保持原行为；运行完整部署包的用户不需要安装 `fxc`。

```powershell
# 可选：安装到项目的 .tools/qt；已有 Qt SDK 可直接使用自己的路径。
powershell -ExecutionPolicy Bypass -File tools/setup-qt.ps1

# 从仓库根目录配置并构建完整应用。
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release -DDRAWVERSE_BUILD_UI=ON -DCMAKE_PREFIX_PATH=.tools/qt/6.8.3/msvc2022_64
cmake --build build --config Release

# 启动；Windows 构建会部署所需 Qt DLL 与 QML 插件。
./build/bin/Release/drawverse.exe
```

这些命令构建并部署应用，运行时仍会读取当前用户已有的配置。Windows 默认配置位于 `%APPDATA%/DrawVerse/DrawVerse.ini`，笔刷资源在同目录的 `brush-resources`。暂存使用系统临时目录，或偏好设置中保存的自定义目录；自定义目录被删除时，应用会自动创建所需目录。若磁盘离线、权限不足或同名文件占用路径，仍会报告具体路径及原因，可在设置中更换暂存目录；无需删除笔刷或整份配置。

只构建 Rust 内核与 FFI：

```sh
cargo build --manifest-path core/Cargo.toml -p paint-core -p paint-ffi --release --locked
```

CLion 从仓库根目录加载项目，重新加载 CMake 后启用 `build`、`run` 两个 preset/profile；共享运行配置位于 `.run/`。选择 `build` 后点击构建，一键生成并部署 `build/bin/Release/drawverse.exe`；选择 `run` 后点击运行，先构建及部署，再启动 `cmake-build-debug/bin/Debug/drawverse.exe`。应用构建入口 `drawverse_runtime` 包含 Qt 运行库、QML 插件和 LICENSE，单独使用它不编译测试程序；上面的完整构建仍包含测试目标。

同一套预设也可在 Developer PowerShell 中使用（需 Ninja 在 PATH 中）：

```powershell
cmake --preset build
cmake --build --preset build

cmake --preset run
cmake --build --preset run
./cmake-build-debug/bin/Debug/drawverse.exe
```

Windows 不混用 MSVC Rust 与 MinGW C++；切换工具链时使用独立构建目录。已有 Qt SDK 可在 CMake profile 或本机 `CMakeUserPresets.json` 中覆盖 `CMAKE_PREFIX_PATH`。更多说明及 Release 复制后的权限排查见 [构建复现](docs/platform/build-reproducibility.md)。

## 验证

统一验证入口会检查 Rust、生成头、C / C++ 静态与动态 ABI，以及 Qt 测试：

```sh
python tools/check.py

# 仅验证 Rust 与 ABI。
python tools/check.py --core-only

# 使用已有 Qt SDK。
python tools/check.py --qt-prefix <Qt-SDK-path>
```

常用局部检查：

```sh
cargo check --manifest-path core/Cargo.toml --workspace --all-targets --locked
cargo fmt --manifest-path core/Cargo.toml --all --check
cargo run --manifest-path core/Cargo.toml -p paint-api-gen --locked -- --check
ctest --test-dir build/qt -C Release -R "^abi_" --output-on-failure
```

`ui/include/paint_api.h` 由 `paint-api-gen` 生成。调整 Rust ABI 后，使用不带 `--check` 的同一命令更新生成头。

通用GPU合成默认保持原显示；产品最低Qt6.5，实际GPU适配要求Qt6.6+ GuiPrivate，目前仅验证Qt6.8.3 / Windows D3D11。G0试验通过 `-DDRAWVERSE_G0_EXPERIMENT=ON` 后显式构建 `qt_gpu_composition_probe` / `qt_gpu_composition_baseline`，不进入默认构建或CTest。G1为ABI1.81/version1不可变计划/有界租约，`qt_composition_transport_tests` 是CPU传输对照；G2在实际Canvas用同窗口设备增量合成根Normal/链接自身蒙版并直接显示，设置 `DRAWVERSE_GPU_COMPOSITION=1` 才启用。原图及视口各最多4194304像素、边长4096、LOD≤6；复杂组合/预算/设备能力不足明确CPU后备。构建 `qt_gpu_composition_tests` 后可运行 `python tools/validate_general_gpu_g2.py artifacts/general-gpu-composition/G2/new-run` 采集两后端正确性，目录必须新建；G1原路径回归脚本继续有效。基线、原始证据、保留失败和支持边界见[通用GPU合成验收](docs/validations/platform/general-gpu-composition-validation.md)。G3–G5及首位移性能目标尚未完成，不能把GPU可用当作交互延迟已改善。

最近的结构整理已完成 Release 构建、原生启动绘制和专项回归；全套 IO 与额外 Qt 交互检查仍有已知失败项，详见 [结构整理验收](docs/validations/platform/source-layout-validation.md)。各阶段记录不代表当前全量测试或跨平台验证均已通过。

## VBR 1.0 与文档维护

当前 VBR 写出固定为 `major: 1, minor: 0`。读取接受 1.x，按实际字段递归补全缺项；可选功能对象整体缺失或为 `null` 时保持关闭。非法参数、未知字段和损坏资源仍会拒绝。

VBR 格式版本与 ABI、state、behavior、模型和来源转换器版本各自独立。当前规范位于 [contracts](contracts/README.md)；开发迭代 schema 仅存放在 `paint-io` 的测试数据目录，供历史回归使用。

设计与说明放在 `docs` 对应领域，独立验收记录统一放在 `docs/validations` 对应领域。修改功能时同步维护设计、验收与相对链接，完整分类入口见 [文档索引](docs/README.md)。

## 许可

本项目的原创代码与文档采用 **Creative Commons Attribution-NonCommercial-ShareAlike 4.0 International（CC BY-NC-SA 4.0，署名—非商业性使用—相同方式共享）**，完整条款见 [LICENSE](LICENSE)。使用与再分发须遵守署名、非商业性使用及适用的相同方式共享要求；商业使用需要另行取得授权。

该许可包含非商业限制，不属于 OSI 意义上的开源软件许可。第三方依赖、Qt 运行库及另有许可声明的测试素材仍遵循各自许可，不因本项目许可而改变。
