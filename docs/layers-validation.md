# 图层控制模块验收

2026-10-08，Windows 11 x64 / MSVC / Rust 1.99.0 / Qt 6.8.3。设计见 [layers-design.md](layers-design.md)，正式 ABI 见 [契约](../contracts/abi.md)。用户已澄清：第一个锁为透明像素锁；“图层样式”仅指混合下拉，本轮不实施 fx。macOS / Linux、最低工具链及真实数位板仍待跨平台验收，Python 插件最后。

本轮实现：

- 三种锁：透明像素锁逐位保留 Alpha，不增加透明区域，不削减半透明边缘，橡皮擦不产生变化；位置锁只禁止移动；完全锁禁止绘画、移动、删除、opacity/fill/blend/seed 修改，仍可显隐、选择和解锁，撤销/重做不受锁限制。
- 27 种混合模式的实际线性预乘颜色合成，透明源/目标正确处理；溶解使用持久化种子和坐标噪声，保存重开不因层 ID 重新编号改变。fill 与 opacity 独立保存；当前作为内容透明度相乘，PS 特殊八种 Fill 曲线未实施，不承诺 PS 逐像素一致。
- 整数偏移保留画布外内容，移出再移回不丢像素，移动后可写入有符号局部瓦片。V 选择移动工具，拖动松手提交一个历史项，方向键 1px / Shift 10px，Esc 取消；拖动阶段暂不实时预览移动结果。
- QML 顶部混合、不透明度、三锁、填充；百分比支持 0..100 键入与弹出滑条，拖动松手只产生一个历史项。紧凑列表提供眼睛、真实缩略图、名称、锁状态、名称搜索、新建/删除；同一组件支持停靠/标签组合/独立浮窗。
- ListView 按可见行创建；仅可见委托请求 64px 原始内容缩略图，安静 200ms 后一个任务依次更新，绘画/新修订取消。只克隆目标层元数据，不在 publication 或每个笔触点复制全部瓦片；PNG 编码在 backend worker。纯透明、隐藏、fill=0 图层不触发无效合成，零填充层跳过冷页读取。
- ABI 1.4.0 兼容新增 LayerAppearance=32 字节、配置读取/提交、移动、背景预览接口，旧结构/符号保留。UI 仍只通过 PaintCoreClient/C ABI 访问文档，重算不进 UI 线程。
- OpenRaster 标准 15 种 Normal/颜色混合名称读写；DrawVerse 层扩展 v1 保存 fill、blend、locks、整数位置、原 opacity、local 坐标与 seed，保留画布外像素。未知版本、非法字段、冲突混合明确拒绝。层像素仍是 8-bit sRGB，特殊模式的标准 composite-op 为 src-over；外部应用忽略扩展后的逐层合成可能不同，mergedimage 为正确最终预览。保存仍受既有边长、像素/归档预算，包括移出画布后扩展的层位图边界；失败保护旧文件。

验证：

- 完整 `python tools/check.py`：133 项 Rust 测试，fmt、clippy -D warnings、doc test、release 构建、cbindgen 一致性全部通过；日志 artifacts/layers-full-check.log。
- 核心覆盖透明锁半透明跨瓦片/橡皮擦/透明区域不分配，位置/完全锁拒绝且无部分修改，解锁/显隐/撤销，独立 fill/opacity/no-op/非法参数，移动后负坐标绘画与取消，冷页及磁盘编码历史删除恢复完整配置。混合有独立数值期望与 alpha 边界，27 种模式保持有限预乘值。
- 渲染覆盖配置、位移、fill、溶解及全部混合改变后的缓存失效、LOD 0/1/3/7、与逐像素文档采样及无缓存输出比对，undo/redo；另外 pin 底层并监测 page_reads，验证 fill=0 的冷图层没有被加载。
- 文件覆盖标准外部 SVG 混合导入/再保存、扩展回环、隐藏/锁/位置/负瓦片与画布外内容、版本/flags/模式/坐标冲突拒绝，既有四格式、归档预算及原子保存回归保持通过。
- FFI 共 34 项回归；旧布局、32-byte 新 DTO/偏移、publication 过期、非法参数先于入队、输入同步复制、actor 锁错误、预览 raw-content 与槽/尺寸限制。真实 C11/C++20 各 static/shared 共四组通过。
- Qt Test 18 个功能项（加初始化/清理共 20）通过；完整 CTest 六组、两倍 DPI 均通过。实际 QML 百分比输入/滑条一次提交、混合/三锁/显隐、浮窗内解锁、鼠标移动/键盘步进/位置锁/取消，async 保存重开与后台缩略图回环通过。常用 build/qt 原生目录同样六组通过；日志 layers-native-build.log / layers-native-tests.log。
- 原生 Windows Qt Test 实际调用 C ABI 画笔后截图，检查了控件、百分比、图层缩略图、眼睛与锁符号的布局/对比度；artifacts/layers-panel.png，日志 layers-native-preview.txt。

性能在构建/测试之外依次运行。600 点原生压感负载：DPR 1.5、1529×1019 输出，累计进程 CPU 328.125ms，画面更新间隔 p50/p95 16/17ms，最终画面追上 31ms，GUI 定时器间隔 p95 9ms。上一模块单次 CPU 234.375ms、再之前 375ms；本轮首次测量 437.5ms，优化普通合成/临时分配后改善。CPU 包括启动/退出与所有线程，不能视为任务管理器瞬时百分比，也不保证所有场景零额外开销；画面 p95 与上一模块同为 17ms。报告 artifacts/layers-ui-performance.json。

核心 120 帧 / 1800×1200 热缓存基准：平均 5.877ms、p95 7.419ms，13152 次命中 / 369 次未命中，约 9.54MiB / 122 条缓存项；日志 artifacts/layers-core-performance.log。这是固定简单笔刷场景，包含笔触、快照、渲染及释放。

8K 全幅单笔：绘画 3910.373ms、提交 273.334ms、冷/热概览 469.071/12.414ms、整笔撤销/重做 322.257/298.535ms。通用混合路径的初次冷概览 905.864ms，普通层专用路径恢复后为上述结果。16384 live 瓦片逻辑像素 1GiB、驻留 256MiB、初始后备区段 8MiB；旧快照/redo 保留后 16MiB。验证全幅不透明、概览正确、撤销重做逐位相等。日志 artifacts/layers-8k-performance.log。非普通混合、移动图层、复杂纹理的冷页/合成更昂贵，<8ms/120fps 仍需按负载验收，Rust GPU 尚未实施。

```sh
# 激活 MSVC / Rust / CMake / Qt SDK 后在项目根目录
python tools/check.py
cmake --build build/qt --config Release
ctest --test-dir build/qt -C Release -R "^(abi_|qt_ui_)" --output-on-failure
./build/qt/bin/Release/drawverse.exe

# 性能测量按顺序运行
python tools/measure_painting.py build/qt/bin/Release/drawverse.exe artifacts/layers-ui-performance.json
cargo run --manifest-path core/Cargo.toml -p paint-render --example painting_bench --release --locked
cargo run --manifest-path core/Cargo.toml -p paint-render --example paging_bench --release --locked
```

后续模块继续图层组/蒙版、选区和一般变换，之后色彩/GPU/PSD。fx、特殊 Fill 曲线另行设计，插件最后。
