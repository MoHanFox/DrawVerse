# 第 3 阶段验证记录：paint-ffi + cbindgen

日期：2026-10-08（Asia/Shanghai），ABI 1.0.0，应用 0.1.0。

## 实际运行结果

| 检查 | 结果 |
|---|---|
| cargo fmt --all -- --check | 通过 |
| cargo clippy --workspace --all-targets --locked -- -D warnings | 通过，无警告 |
| Rust 单元/集成测试 | 47 项通过：原内核 29 + FFI 18，0 失败/忽略 |
| 文档测试 | 通过，当前没有 doctest，不额外计数 |
| cargo build --workspace --release --locked | 通过，含 staticlib / cdylib 与生成工具 |
| 固定 cbindgen 0.29.2 生成正式头 | 成功；后续 --check 字节比较通过 |
| CMake / Ninja，C11、C++20，/W4 /WX | 编译链接成功，头文件 sizeof / offsetof 检查通过 |
| abi_c_static | 通过，链接 Rust staticlib |
| abi_cpp_static | 通过，链接 Rust staticlib |
| abi_c_shared | 通过，链接 Rust DLL 与导入库 |
| abi_cpp_shared | 通过，链接 Rust DLL 与导入库 |
| 统一 python tools/check.py | 整体通过，完整日志 artifacts/ffi-check.log |

最新验证包含输出 struct_size 前缀归一化与 DLL 单次部署的修改；并行消费者不再同时写同一 DLL。CTest 四个 ABI 程序都是实际执行，不是仅编译，也没有因 Release 的 NDEBUG 关闭检查（使用明确失败返回的 CHECK 宏）。

18 项 FFI 回归覆盖：64 位 DTO 尺寸/关键偏移；真实 capability；core 子对象 BUSY；跨 core/旧/错误类型/未登记句柄；NULL/不对齐/过小 DTO；扩展尾部保留；失败输出清空；UTF-8 与小缓冲不部分写；图层历史；非法 stroke_begin 保留原选择；压力像素/undo/redo；stride padding/容量/ROI/溢出/格式；TLS 及查询不覆盖错误；panic containment 与 poison 隔离；回调锁外只读重入、修改 BUSY；撤订阅等待回调退出；订阅所有权与幂等释放。

C 消费者同时验证 UTF-8 名称、跨瓦片笔触、像素查询/读取、undo/redo、图层透明度、pixel/state 回调与生命周期。C++ 消费者验证 RAII、noexcept 回调、压感、历史、4 个并发只读线程以及撤订阅后无回调。事件提交状态更新通过 C 的 state_calls 检查验证。

## 环境与复现

Windows x64，Rust/Cargo 1.99.0，MSVC 19.51 / 工具集 14.51，Windows SDK 10.0.26100.0，CMake 4.3.1，Ninja，Python 3.10.6。工具链原已安装；只下载 Cargo.lock 约束的生成器依赖到项目忽略目录 .tools/cargo-home，没有修改全局 PATH 或安装全局 cbindgen。cargo metadata 检查锁定依赖声明的 MSRV 未高于 1.85；实际 Rust 1.85 编译仍待 CI。

本机 PowerShell 初始化（路径仅记录本机，不进入构建配置）：

```powershell
$taskVcOutput = & $env:ComSpec /d /s /c '"D:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat" >nul && set'
if ($LASTEXITCODE -ne 0) { throw 'MSVC environment initialization failed' }
foreach ($taskLine in $taskVcOutput) {
    if ($taskLine -match '^([^=]+)=(.*)$') {
        [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process')
    }
}
$env:PATH = 'D:\Rust\1.99.0\rustup\toolchains\1.99.0-x86_64-pc-windows-msvc\bin;D:\CLion 2026.2.1\bin\cmake\win\x64\bin;D:\CLion 2026.2.1\bin\ninja\win\x64;' + $env:PATH
$env:CARGO_HOME = Join-Path (Get-Location) '.tools/cargo-home'

python tools/check.py
# 统一验证将四个程序构建到此处
./build/abi/bin/Release/abi_c_static.exe
./build/abi/bin/Release/abi_cpp_shared.exe

# 更新/检查正式头
cargo run --manifest-path core/Cargo.toml -p paint-api-gen --locked
cargo run --manifest-path core/Cargo.toml -p paint-api-gen --locked -- --check
```

## 产物与验证边界

正式可链接产物：core/target/release/paint_ffi.lib、paint_ffi.dll、paint_ffi.dll.lib；正式头 ui/include/paint_api.h。头与实现可供后续 PaintCoreClient 接入。旧 draft.h 的 task/IO/render_tile 不是现有库符号；当前 read_tile 输出线性 RGBA32F，不做显示 sRGB/ICC 转换。

本轮未验证 macOS/Linux、ARM64、MSRV 实际构建、MinGW、Qt Test、pytest、实际硬件数位板、GPU 或性能延迟。没有安装 Qt，也没有 GUI。callbacks 外部异常/野指针/用户提供错误容量属于调用者契约，不能声称被 Rust panic containment 保护。token 进程累计上限 65,536 是正式资源限制；大文档内存和历史会正常释放。

下一阶段：paint-task 的可取消优先级任务与文档命令串行化，以及 paint-render CPU 后端/快照合成；之后实施 Qt 界面和工作区。
