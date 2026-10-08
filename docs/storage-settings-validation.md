# 暂存盘设置与残留清扫验收

2026-10-08，Windows 11 / MSVC x64 / Rust 1.99.0 / Qt 6.8.3。方案见 [设计](storage-settings-design.md)，ABI 1.3 兼容新增接口见 [契约](../contracts/abi.md)。本轮未验证 macOS、Linux 或最低 Rust 版本，Python 插件仍最后实施。

编辑 → 性能与暂存盘：选择现有目录或留空使用系统临时目录，设置像素驻留缓存 64/128/256/512/1024 MiB、单文档瓦片暂存 1–64 GiB、保留空闲 0–16384 MiB。默认 256MiB / 8GiB / 512MiB。显示本次运行与下次启动的配置；后台检查成功并保存后重启生效，不迁移当前文档或清空历史。磁盘历史仍独立限 4GiB / 100 条，瓦片预算不是整个进程 RAM 或所有文档磁盘总额。

paint-storage 统一管理分页和历史后备，增长检查按 1MiB / 1 秒刷新。正常退出自动释放数据文件与空 run，保留小型命名空间标记。启动/检查候选设置只清扫验证过的失活 run：持有独占维护锁，验证租约标记、随机文件名、普通文件类型与规范路径；活动租约、未知内容、子目录、链接和未标记目录均跳过。最多扫描 256 个命名空间项、每 run 最多验证 2049 个项，不宣称一次清完任意多残留。旧版本 Temp 散落文件不会凭前缀删除。暂存清扫不是自动保存/崩溃恢复，也不是抵御同用户恶意进程的安全沙箱。

验证结果：

- 完整 `python tools/check.py`：121 项 Rust 测试，fmt、clippy -D warnings、文档测试、release 构建、生成头一致性全部通过。日志 `artifacts/storage-full-check.log`。
- 新增存储回归验证 Unicode 目录、真实子进程持锁/强制终止后的清扫、陌生文件/错误租约不删除、无标记非空命名空间与空标记不被接管、Windows junction 跳过、保留空间/稀疏增长失败先于数据写入、正常退出与后备文件持有管理器的寿命。
- 内核回归验证同一选定目录包含冷页和磁盘历史，低驻留缓存下撤销/重做逐像素相等，旧快照在文档释放后仍可读取；空间检查失败的笔触保留原像素及旧历史。四种格式导入均继承所选目录/像素预算，不落回默认配置。
- C11 / C++20 各 static/shared 四组通过；包含旧接口、新建 configured core / inspect / 错误配置与新 DTO 大小/偏移断言。ABI 次版本 1.3，旧结构布局不变。
- Qt Test 15 个功能项（加初始化/清理共 17）通过；完整 CTest 六组（含两倍 DPI）通过。验证 QML 控件默认值与保存按钮、持久化、保存失败保留旧配置、当前文档/历史不变、重启应用配置，以及坏启动目录时可打开设置修正。
- 常用运行目录 `build/qt/bin/Release/drawverse.exe` 已重新构建；原生目录 CTest 同样六组通过，日志 `storage-native-build.log` / `storage-native-tests.log`。设置窗口原生截图 `artifacts/storage-preferences.png`，已检查布局及文字对比度。

性能基准与构建/测试分开运行。相同原生 960×640 / 600 点压感输入，DPR 1.5、1529×1019 物理输出，累计进程 CPU 时间 234.375ms（上一模块单次 375ms），帧更新间隔 p50/p95 为 16/17ms，最终画面追上输入 24ms，报告 `artifacts/storage-ui-performance.json`。CPU 时间包括启动、退出和所有线程，单次波动不能视作通用性能提升比例。

8K 全画布基准绘画 3520.335ms、历史提交 251.337ms、冷/热概览 394.198/11.236ms、整笔撤销/重做 293.540/272.400ms；16384 live 瓦片逻辑像素 1GiB，驻留 256MiB，初次后备区段 8MiB，保留旧快照并重做后后备区段 16MiB。基准验证全画布不透明、概览正确、撤销重做逐位相等，日志 `artifacts/storage-8k-performance.log`。此负载重复像素多，复杂纹理不保证相同压缩率，冷页读取也不满足每帧 <8ms。

可用空间检查不是独占预留，其他进程仍可消耗磁盘；真实 IO 失败由原有事务返回错误并保护状态。磁盘路径/预算不会取消资源限制。未实现 GPU 内核、ICC/HDR、PSD、自动恢复、自有工程格式，真实数位板与其他平台仍待验收。

```sh
# 已激活编译器、Rust、CMake 和 Qt SDK
python tools/check.py
cmake --build build/qt --config Release
ctest --test-dir build/qt -C Release -R "^(abi_|qt_ui_)" --output-on-failure
./build/qt/bin/Release/drawverse.exe

# 基准依次运行，不与构建/测试并发
python tools/measure_painting.py build/qt/bin/Release/drawverse.exe artifacts/storage-ui-performance.json
cargo run --manifest-path core/Cargo.toml -p paint-render --example paging_bench --release --locked
```

后续模块深化文档模型：图层组、蒙版等。Python SDK/插件宿主保持最后实施。
