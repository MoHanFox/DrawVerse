# GPU 算子与弹性液化验收

2026-10-19。对应[设计](../../tools/liquify-gpu-operators-design.md)。

状态：Windows 功能与目标回归通过，连续大范围输入手感待实机复核。用户在收尾阶段明确要求液化只做最小测试；此后仅运行算子最小回归、三个 Qt 目标用例和四项 ABI，没有重跑全量。

## 2000 px 实机反馈后的交互修订（2026-10-19）

用户确认初版 2000 px 拖拽粘滞、GPU 占用较高，初版性能不予实机通过。此次默认偏好改为“自动选择 GPU”，保留已保存的显式 CPU 选择。当前性能结果与实现以下述修订为准，后文保留初版记录供对照。

修订包括：按半径选择 2/4/8/16 px 网格、较小笔刷细化旧场且历史保存前后间距；无选区预览读回紧凑块颜色后展开，恒等标志恢复原始逐像素数据；复用经一次校验的不可变源数值和 GPU 源槽；小于等于 2048 源瓦片的图像进入液化时预热；GPU 上传/读回缓冲区复用。CPU/GPU 源缓存分别上限 2048 瓦片（各 128 MiB），GPU 持有弱身份，不 pin 已结束会话的 CPU 源。位移场、历史和文档仍由 CPU 管理，画布外恢复和最终全分辨率采样保留。

输入现在仅允许一个在途更新和一个待处理最新位置；忙时合并中间位置，反馈后继续，抬笔先提交末位置再结束历史。此策略会简化繁忙期间的中间轨迹，不能宣称每个设备原始样本均独立执行。单输入预算改计节点更新加一次预览面积，避免快划合并后仍按多次全像素预览误拒绝。

相同 RTX 4060 / Vulkan、2048×1536、2000 px、六输入非零位移的 Release 示例，包含 CPU 准备、GPU 运算、传输和结果采用；不包含 Qt 画面合成。单次本机对照：

| 工具 | 初版六输入预览 | 修订六输入预览 | 修订单次输入 | 修订进入模式预热 | 初版/修订最终完成 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 推移 | 2.433 s | 0.199 s | 31.5–35.7 ms | 47.5 ms | 0.811 / 0.377 s |
| 旋转 | 2.401 s | 0.193 s | 29.6–34.2 ms | 41.5 ms | 0.769 / 0.356 s |

六输入计算约为初版的 1/12；预热明确单独计时，不将其隐藏进初始化之外。两组六输入中执行器内部累计耗时约 28.1/26.0 ms，其余主要是 CPU 文档/像素处理。GPU 利用率百分比没有采集，也没有运行同输入的 Photoshop 基准，不能据此宣称与 PS 等速或 GPU 占用已达标。用户实体设备与全窗口连续手感仍需复核。过程日志：`artifacts/liquify-interaction-profile.txt`；可复现命令仍为 `cargo run -p paint-gpu --release --locked --example liquify_compute_performance -- auto`。

本轮最小回归：`cargo test -p paint-ops -p paint-gpu --release --locked -- --nocapture` 共 10 项通过，含紧凑结果/恒等恢复、GPU 源预热及同坐标不同源身份缓存隔离、CPU/GPU 数值对照和失效回退；`paint-core --test liquify --test liquify_elastic` 共 14 项通过、1 项手动性能忽略；`paint-core --lib large_mesh_refines` 1 项通过，验证画布外场在细化前后保持、撤销/重做恢复间距和场。相关三个库的 `cargo clippy ... --lib --locked -- -D warnings` 通过。日志为 `artifacts/gpu-interaction-final.txt`、`gpu-interaction-lint.txt`。

Qt 只执行 `computeHardwareSelectionPersistsAndFallsBackVisibly` 和 `maximumLiquifySizePublishesPixelsAndKeepsLocalAndDocumentHistory`，2 项功能通过（含初始化/清理共 4 passed）。前者增加新配置默认 auto 检查；后者在同一事件循环内送入 64 个位置并抬笔，确认没有逐事件排队，末位置有像素结果且局部/文档历史精确恢复。512 px 画布上该用例至帧完成约 51 ms，含 UI/轮询等待，不能外推全尺寸窗口。日志为 `artifacts/gpu-interaction-qt.txt`。

Windows Release 构建与 Qt 运行库部署通过。原 `build/verify-ui/bin/Release/drawverse.exe` 被用户运行中的进程锁定，保留该进程，使用本地 CMake `CMAKE_RUNTIME_OUTPUT_DIRECTORY_RELEASE` 构建到 `build/verify-ui/bin/Release-wgpu/`；不修改仓库构建配置、不终止用户进程。日志为 `artifacts/gpu-interaction-ui-build.txt`。本轮未重跑全量验证，不改变后文记载的既有全量图层断言失败。

## 实现范围

分支为 `feat/wgpu-support`。`paint-ops` 提供弹性场更新/重映射契约与 CPU 执行器，`paint-gpu` 使用 wgpu 26.0.1；设备由 FFI 装配，核心不依赖 wgpu。性能设置可选 CPU、自动或真实硬件，并保存偏好、显示失败回退原因。ABI 升至 1.61，新增三个计算设备接口及 bit 56，生成头同步。

液化采用不可变原图、2 px 稀疏弹性网格、局部增量更新和逆向采样。会话中画布外的源与场保留，鼠标/笔均可从画布外向内拖入；支持反向推移、重建、取消及精确历史恢复。预览只生成画布相交区域，大笔刷使用 2 px 双线性预览；完成使用同一权威场生成 1 px 三次插值结果，成功后原子采用并记录一次文档历史。完成期间显示不确定进度条。

## 已执行验证

环境：Windows 11、Qt 6.8.3、MSVC x64；真实计算设备为 NVIDIA GeForce RTX 4060 Laptop GPU / Vulkan。Qt 用例使用 `QT_QPA_PLATFORM=offscreen`、`QT_QUICK_BACKEND=software`，这只控制 Qt 展示，计算测试仍执行真实 wgpu 硬件。

下列 Cargo 命令在 `core` 目录执行。收尾前已经通过的结果保留，不为缩小测试范围而重复执行。

| 命令 / 范围 | 实际结果 |
| --- | --- |
| `cargo test -p paint-ops --locked` | 收尾最小检查：1 项单元、5 项集成通过；含预览块采样复用与独立逐像素计算精确一致 |
| `cargo test -p paint-ops -p paint-gpu --release --locked -- --nocapture` | 算子 6 项、GPU 设备丢失回退 1 项、GPU 数值/缺失设备 2 项、真实 GPU 文档 1 项通过 |
| `cargo test -p paint-core --test liquify --test liquify_elastic --locked` | 11+3 项通过，1 项手动性能用例忽略；含画布外恢复、最终质量、压力/密度、选区/偏移/锁、历史和 CPU 回退 |
| `cargo test -p paint-core --lib liquify --locked` | 4 项通过；含场预算/历史故障原子性和四工具确定性 |
| `cargo test -p paint-ffi --test async_session compute_api --locked` | 1 项通过；枚举、CPU/缺失设备、忙时拒绝、新建保留执行器 |
| 上项增加 `--no-default-features` | 纯 CPU 构建同一目标用例通过 |
| `cargo run -p paint-api-gen --locked -- --check` | 生成 C 头严格检查通过 |
| `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过 |
| CMake Windows Release 应用、Qt 测试构建 | 通过；Windows 静态链接补充 wgpu 所需系统库 |
| `ctest --test-dir build/verify-ui -C Release -R "^abi_" --output-on-failure` | 收尾 4 项通过，C/C++ 静态/动态 ABI |
| Qt 三项目标（见下） | 收尾连续执行通过：3 项功能、初始化/清理，共 5 passed，0 failed，约 6.5 s |

Qt 收尾命令：

```powershell
build/verify-ui/bin/Release/qt_ui_tests.exe computeHardwareSelectionPersistsAndFallsBackVisibly liquifyMouseAndPenCanStartOutsideClippedCanvas maximumLiquifySizePublishesPixelsAndKeepsLocalAndDocumentHistory
```

三项分别验证硬件枚举/偏好保存/真实 GPU 选择/可见 CPU 回退，真实 Main.qml 窗口中鼠标与笔从裁剪区外起笔，以及最大 2000 px 液化的完成状态、局部与文档撤销重做。最后一项在 512 px 画布上单次发布至帧完成约 206 ms，不能外推全尺寸连续输入。设置面板的控件存在性与持久化入口另已通过，截图人工检查布局可读。

收尾另修正硬件轮询在状态不变时重复通知界面的行为，防止未保存选择被绑定刷新；只重跑 `computeHardwareSelectionPersistsAndFallsBackVisibly`，增加跨越 650 ms 轮询周期无重复通知的检查，1 项功能通过（连同初始化/清理共 3 passed），日志为 `artifacts/gpu-qt-settings-final.txt`。增量 Release 构建通过，没有重复液化/性能测试。

GPU 对照覆盖四种弹性工具，以及双线性/三次采样、不同预览步长和选区。实际最大浮点误差 `4.9471855e-6`，小于验收阈值 `1e-4`。CPU/GPU 不保证逐位相同，RGBA8 量化摘要可能不同；同一后端撤销/重做按实际像素精确恢复。设备丢失用故障信号注入验证回退，未拔除物理显卡。

本机过程日志位于忽略的 `artifacts/gpu-ops-minimal.txt`、`gpu-final-targets.txt`、`gpu-core-targets.txt`、`gpu-qt-minimal.txt`、`gpu-ui-build.txt`、`gpu-final-clippy.txt`；它们不是仓库交付物，以上命令和测试源码是可复现入口。

## 性能实测与限制

在用户要求缩小测试前，已运行 `paint-gpu` 的 `liquify_compute_performance` Release 示例：2048×1536 图像、直径 2000 px、六次输入，包含源准备、上传、读回、CPU 采用和最终重采样，排除设备初始化。单次同机结果如下，并非统计基准：

| 工具 | CPU 预览合计 | GPU 预览合计 | CPU 完成 | GPU 完成 |
| --- | ---: | ---: | ---: | ---: |
| 推移 | 2.557 s | 2.433 s | 1.392 s | 0.811 s |
| 旋转 | 2.543 s | 2.401 s | 1.302 s | 0.769 s |

该场景 GPU 最终计算约缩短 41–42%，预览仅约 5%。当前每批仍上传输入并读回，CPU 准备与提交占比明显，大范围连续输入尚不能宣称流畅。新弹性场/恢复/最终采样语义与旧逐像素液化不同，旧 0.77/0.87 s 数据不能直接当作同算法加速比；本轮不宣称性能全面优于旧实现。后续可研究驻留场和源缓存，未在本轮扩展。

此前启动过 `python tools/check.py`，在既有 `layer_batch::subtree_delete_normalizes_overlaps_and_failures_are_atomic` 的第 278 行停止：旧断言要求删除全部图层报错，与当前空图层栈行为不符。该测试和对应图层代码本轮未修改，未修复无关行为，也不声称全量检查通过。为使现有测试目标编译，补了三个旧 `BrushDefinition` 初始化的 outline 字段；另移除旧测试/示例的 Clippy 告警，不改变产品笔刷行为。

未验收：用户实体笔与连续拖拽手感、DX12/Metal 与跨平台设备、智能对象持久保存位移场、其他工具/笔刷 GPU 计算、GPU 画布合成。液化会话内可恢复并不代表提交退出后仍保留可编辑液化网格。
