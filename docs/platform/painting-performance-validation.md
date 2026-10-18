# 绘画 CPU 与延迟回退验收

2026-10-08，Windows 11 x64、Rust 1.99.0 MSVC release、Qt 6.8.3、16 个逻辑处理器。先保存优化前原生程序再修改引擎，两次用相同基准运行；未并行执行编译或其他基准。测量记录位于忽略的 artifacts 目录，可按下列命令重跑。

原因及修复：原 CPU 输出按整幅物理视口逐像素查表，放大后对重复采样的像素重复做 sRGB 幂运算，且每帧重合成所有瓦片。新实现按稀疏矩形输出，提前按轴映射，每个源 mip 像素编码一次；每视口有界缓存只重合成源身份或图层属性改变的瓦片。Qt 原 layers/stateChanged 每次笔触发布都通知 Repeater，现使用独立 layersChanged 并按值变化发出状态通知。

| 固定负载 | 优化前 | 优化后 |
|---|---:|---:|
| 内核 120 帧：1800×1200，平均每帧（含笔触、快照、渲染、释放） | 135.711 ms | 5.200 ms |
| 内核同场景 p95 | 148.331 ms | 6.910 ms |
| 原生 QML 窗口，600 个压感点，进程累计 CPU 时间 | 4000 ms | 406.25 ms |
| 原生画面更新间隔 p50 / p95 | 64 / 96 ms | 16 / 17 ms |
| 输入结束至最终画面追上文档 | 168 ms | 32 ms |
| 原生收到的新修订画面数 | 79 | 304 |
| GUI 输入定时器间隔 p95 | 9 ms | 9 ms |

原生窗口两次均为默认 960×640 文档、24 像素画笔、600 点分为六笔、8ms 输入定时器，DPR 1.5、最终物理帧 1529×1019。测量绘画阶段约 5 秒，进程墙钟约 6 秒。累计 CPU 时间包括所有进程线程、启动与退出，不能当作任务管理器的瞬时百分比；下降约 89.8%。frame_updates 为到达 Qt 的新文档修订帧，presentations 单独记录，不能等同显示器刷新率。该固定场景说明回退已改善，不代表所有大画布、复杂图层或设备均达到 <8ms / 120fps。

内核热缓存场景：13152 次命中、369 次未命中，最终计账约 9.54 MiB / 122 条目；修改中的瓦片正确失效。上限与工作内存说明见 painting-performance-design.md。仍为 Rust CPU 绘画/合成，Qt 场景图负责显示，未宣称已实现 Rust GPU 后端。

验证覆盖：85 个 Rust 测试；新增 5 个缓存/稀疏输出回归，与保留的优化前 dense_reference 逐像素严格相等，覆盖两种输出格式、LOD 0–20、分数 ROI、跨瓦片、越界、层透明度/隐藏、百万像素稀疏文档、相同尺寸新文档、撤销/重做/取消、弱引用写入失效及资源上限。Qt Test 10 个功能项（含初始化/清理共 12 项）通过，真实 QML 图层委托在连续笔触与多次帧更新中保持同一对象，修改图层属性仍更新；保存未完成修改保护与文件 IO 回归通过。两倍 DPI、四组 C11/C++20 static/shared ABI、clippy -D warnings、cbindgen 无漂移检查通过。ABI 仍为 1.2.0。

```sh
# 已激活平台编译器、Rust、Qt/CMake 的项目根目录
python tools/check.py
cmake --build build/qt --config Release
ctest --test-dir build/qt -C Release -R "^(abi_|qt_ui_)" --output-on-failure

# 有界缓存 release 渲染基准；--uncached 可测本轮无缓存后端
cargo run --manifest-path core/Cargo.toml -p paint-render --example painting_bench --release --locked
cargo run --manifest-path core/Cargo.toml -p paint-render --example painting_bench --release --locked -- --uncached

# 原生窗口完整输入/渲染路径与进程 CPU 时间，自动关闭
python tools/measure_painting.py build/qt/bin/Release/drawverse.exe artifacts/painting-ui-after.json
./build/qt/bin/Release/drawverse.exe
```

本机已保留旧可执行文件 drawverse-before-performance.exe 用于对比；工具不会自动制造优化前版本。报告 artifacts/painting-ui-before.json、painting-ui-after.json、painting-render-before.txt、painting-render-after.txt；完整检查 painting-check.log，常用启动目录的六组 CTest 结果 painting-native-tests.log。Python 这里只提供标准库测量工具，插件 SDK/宿主仍按用户要求最后实施。
