# 大笔触历史存储验收

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
