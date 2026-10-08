# 第一轮验证记录

日期：2026-10-08（用户时区 Asia/Shanghai）。范围：架构 + paint-core 最小内核。

## 实际运行结果

| 验证 | 结果 |
|---|---|
| cargo fmt --all -- --check | 通过 |
| cargo clippy --workspace --all-targets --locked -- -D warnings | 通过，无警告 |
| cargo test --workspace --all-targets --locked | 2 个单元测试 + 27 个集成测试通过，0 失败/忽略 |
| cargo test --workspace --doc --locked | 通过，当前无 doctest，不计入 29 项测试 |
| cargo build --workspace --release --locked | 通过 |
| 顶层 CMake 配置 + Release 构建 | 通过（Ninja） |
| CTest implemented_modules | 通过；重用 tools/check.py |
| stroke_preview release 运行 | 通过，512×320，31 个实际瓦片 |
| 示例 undo/redo 的逐像素校验 | 完全相同 |
| 预览视觉检查 | 三条不同颜色压力笔触，半径和透明度变化正常 |

测试覆盖：超大空白画布零分配、跨瓦片绘制、常压采样密度稳定、压感/橡皮擦、图层组合与历史、取消、redo 分支、非法数值、时间戳与数位板属性、首点/中途/末端资源失败整笔回滚、大图层删除失败无修改、历史淘汰、只读快照、脏区域和边缘裁剪。

环境：Windows x64，Rust 1.99.0 / Cargo 1.99.0，MSVC 14.51 x64 与 Windows SDK，CMake 4.3.1，Python 3.10.6。仅使用本机已有工具，不修改全局 PATH、不安装依赖。声明的最低 Rust 1.85 尚未单独验证，后续 CI 补 MSRV 作业。

本机复现（PowerShell；绝对路径仅记录本机环境，不参与项目构建配置）：

```powershell
# 导入当前进程的 MSVC 工具链环境
$taskVcOutput = & $env:ComSpec /d /s /c '"D:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat" >nul && set'
if ($LASTEXITCODE -ne 0) { throw 'MSVC environment initialization failed' }
foreach ($taskLine in $taskVcOutput) {
    if ($taskLine -match '^([^=]+)=(.*)$') {
        [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process')
    }
}
$env:PATH = 'D:\Rust\1.99.0\rustup\toolchains\1.99.0-x86_64-pc-windows-msvc\bin;D:\CLion 2026.2.1\bin\cmake\win\x64\bin;D:\CLion 2026.2.1\bin\ninja\win\x64;' + $env:PATH

python tools/check.py
cmake -S . -B build -G Ninja
cmake --build build --config Release
ctest --test-dir build -C Release --output-on-failure
cargo run --manifest-path core/Cargo.toml -p paint-core --example stroke_preview --release -- artifacts/stroke-preview.ppm
```

预览 PNG 由标准库脚本将示例 PPM 转为展示副本并实际查看；它不代表 paint-io 已实现 PNG 编码。输出位于 `artifacts/stroke-preview.ppm` / `artifacts/stroke-preview.png`，构建文件与预览均被 .gitignore 排除。

## 验证边界

未运行 macOS/Linux、Qt Test、pytest、实际数位板测试或 GPU/ICC/文件格式测试，因为对应模块尚未实施。没有证明 <8ms 延迟、60–120fps、PSD 完整兼容或操作系统沙箱。历史预算约束模型保留的像素快照，不覆盖调用者长期保留的外部快照、分配器/元数据开销或整个进程的 RSS。

C ABI 目前为草案，没有实现/生成正式头/跨语言链接；静态/动态 FFI 库于下一模块交付。当前 release 输出为 Rust rlib。
