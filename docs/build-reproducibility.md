# 跨平台检出与生成头一致性（2026-10-09）

性能对照使用 Windows 新建工作树时，core.autocrlf=true 将源文件与已生成头转换成 CRLF；cbindgen 的字节一致性检查随之失败。源目录当前手工生成的 LF 头能通过，不等于新检出能通过。

比较两种方案：检查时宽松忽略换行，会让生成物存在多个字节版本且无法防止源注释影响生成结果；在仓库声明文本 LF，则源文件、生成头和构建脚本在三个平台相同。采用 .gitattributes 的 text=auto eol=lf，常用图片/归档/字体显式 binary。保留 cbindgen 严格字节检查，不修改用户全局 Git 配置，不改变 ABI。

验收先于实现：真实临时 Git 仓库设置 autocrlf=true，检出中文 Rust/C 头/QML/Python/Markdown/PowerShell 文本后验证 LF，PNG/ZIP 逐字节保留；tooling unittest 接入既有 tools/check.py。独立新工作树直接执行生成头 --check，不先重写生成头。最终完整 check 通过后独立提交/推送。

构建输出必须按工作树隔离，多个不同源码目录共享同一个 Cargo target 可能污染相同包名的顶层库文件；对照构建使用独立输出目录。依赖下载可共用 CARGO_HOME，产物不跨工作树共用。先前性能对照产生的共享缓存已清理并从主源码重新完整验证。


验收结果：真实 autocrlf=true 检出 unittest 通过，中文文本 LF 与图片/ZIP 字节保留。python tools/check.py 完整通过（artifacts/build-reproducibility-check.log）：171项 Rust、严格头检查、四组 C/C++ ABI、30项 Qt 与12项两倍 DPI，六个 CTest 无失败。新建独立工作树直接 cargo fmt --check、paint-api-gen --check 通过，未运行生成模式；git ls-files --eol 确认为 i/lf w/lf attr/text=auto eol=lf（artifacts/fresh-checkout-header.log）。未改变全局 Git 设置、ABI 或绘画路径。工具测试使用 Python 标准库 unittest，不是插件 pytest；Python 插件仍最后实施。
