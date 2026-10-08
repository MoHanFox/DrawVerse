# 文档分页验收（2026-10-08，Windows）

默认 4096 瓦片不再是文档写入上限，而是共享像素驻留缓存预算。冷页无损存入系统临时目录；笔触、快照、渲染、历史和文件导出均使用可失败读取。C ABI 仍为 1.2.0，头文件无漂移，Qt 仅使用 PaintCoreClient。

验收覆盖：默认预算下 4096×4160 整幅绘画，4160 瓦片全不透明，保持一个撤销命令；低驻留预算多图层画面/快照/取消/整笔撤销重做逐位相等；压缩与原始页、校验损坏、写失败、重复失败不累积驻留缓存、原子导出保护原文件、区段复用/正常删除、并发读与淘汰计账、私有缓冲复用且旧 Weak 缓存失效、共享快照强制复制、并发 Weak 读者保留原版本、跨文档 owned clone 脱离源暂存文件。分页渲染在不同 LOD 下与原 CPU 金图相等；缓存命中不读磁盘。PNG/JPEG/WebP/ORA 分页保存/重开通过。

Qt 实际流程验证 4096×4160 PNG 打开、绘画、显示正确颜色、撤销/重做、后台保存，以及既有停靠/浮动、自定义面板、压感、取消、退役笔记迁移和两倍 DPI。12 个功能项（含初始化/清理共 14 项）。完整 check 含 110 个 Rust 测试、clippy -D warnings、fmt、release、cbindgen，以及 C11/C++20 static/shared 和 Qt/DPI 六组 CTest。

8K release 基准绘制单笔黑色全幅 8192×8192，验证全部像素不透明、1024×1024 概览正确、完整撤销重做逐位相等。16384 live 瓦片的逻辑像素为 1GiB，驻留为 4096 瓦片 / 256MiB。绘画结束后后备区段分配 8MiB、历史编码约 512KiB；撤销/重做时基准故意保留旧快照，因此后备还会增长以保存两个版本（本负载重复像素多，复杂纹理不会得到同样压缩率）。测量数据记录在 artifacts/document-paging-8k.log。

小画布回归使用既有 960×640 文档、600 压感点、8ms 输入定时器、DPR 1.5、1529×1019 物理输出；进程累计 CPU 时间包括启动/退出和所有线程，不能当成任务管理器瞬时百分比。原生结果 artifacts/document-paging-ui-final.json，内核 1800×1200 / 120 帧结果 document-paging-render.txt。初版额外复制导致的 CPU 回退已通过独立 Box 像素数组和私有笔触缓冲复用修复；早期未优化报告不作为最终结果。

本机最终构建测量（单次计时随系统负载波动，记录完整报告）：

| 负载 | 结果 |
|---|---:|
| 8K 单笔全幅绘画 / 提交历史 | 3863.724 / 292.801 ms |
| 8K 冷页概览 / 热缓存概览 | 470.161 / 12.711 ms |
| 8K 整笔撤销 / 重做 | 307.293 / 296.748 ms |
| 保留旧快照并重做后，驻留 / 后备区段 | 256MiB / 16MiB |
| 1800×1200 内核 120 帧平均 / p95 | 6.255 / 8.185 ms |
| 原生 600 点，累计进程 CPU 时间 | 375 ms |
| 原生帧更新间隔 p50 / p95 | 16 / 17 ms |
| 原生输入结束至最终画面追上 | 24 ms |

历史模块上一版同一原生负载为 390.625ms CPU / 17ms 帧间隔 p95；最终没有重现最初 750ms 的 CPU 回退。开发过程私有缓冲优化后另一次为 187.5ms，单次计时波动较大，不用最低值作为最终承诺。1800×1200 微基准每次分配/释放帧，与原生窗口负载不同；这两项和大画布首次读取均不能证明所有场景达到 <8ms / 120fps。

预算范围：默认最多 1,000,000 个逻辑 live 瓦片，8GiB 暂存区段（含可重用空闲块），4GiB 磁盘历史 / 100 条命令。像素缓存包含快照和 inline 历史持有的旧版本；工作 pin 允许短暂超额。256MiB 不包含元数据、渲染缓存、输出、Qt 纹理、编码/解码工作区。格式最大边 16384、128M 像素，输入 256MiB，解码器分配限额 1GiB，ORA 解压总量仍限 512MiB；独立限制可能使较大/复杂文件被明确拒绝，不宣称完全流式 IO。历史无损，不代表 8 位 sRGB 文件格式无损保存浮点文档。暂存容量/磁盘失败仍可报错并保护原状态。

默认使用系统临时目录，正常释放清理；暂存盘路径设置、空闲空间策略和异常退出清扫留到下一模块。仍为 Rust CPU 绘画/合成，Qt 场景图显示；GPU 内核、ICC/HDR、PSD、真实数位板矩阵、macOS/Linux/MSRV CI 尚未验收。插件最后实施。

```sh
# 已激活平台编译器、Rust、CMake 与 Qt SDK
python tools/check.py
cmake --build build/qt --config Release
ctest --test-dir build/qt -C Release -R "^(abi_|qt_ui_)" --output-on-failure

# 基准依次运行，避免与构建/测试并发干扰计时
cargo run --manifest-path core/Cargo.toml -p paint-render --example paging_bench --release --locked
cargo run --manifest-path core/Cargo.toml -p paint-render --example painting_bench --release --locked
python tools/measure_painting.py build/qt/bin/Release/drawverse.exe artifacts/document-paging-ui-final.json

# 直接运行已更新的 Qt 程序
./build/qt/bin/Release/drawverse.exe
```

本机工具链为 Rust 1.99.0、MSVC x64 / Windows SDK、Qt 6.8.3；完整日志 document-paging-check.log，常用运行目录构建/测试日志 document-paging-native-build.log / document-paging-native-tests.log。暂存文件不是可恢复原生工程格式，不提供断电恢复保证。
