# 跨平台检出与生成头一致性（2026-10-09）

## 2026-10-21：构建后启动报核心错误8

使用README两条CMake命令构建的Release仍会读取用户配置。此次错误可由同一Release的C ABI直接复现：已保存的 `D:/DrawVerse/tmp` 不存在，paint_core_create_with_storage返回8和Windows错误2；同一DLL使用默认系统临时目录返回0。它是暂存目录初始化失败，不是DLL或部署文件缺失。

按用户要求，ScratchSpace初始化先递归创建配置目录，再沿用原canonicalize、空间预留、命名空间与租约校验。已有目录保持，缺失父目录可补齐，不更换保存的路径，不回退系统盘；盘符离线、权限不足或文件占位时仍失败，并在错误中写明尝试创建的路径与系统原因。目录创建在既有后台路径执行，不把磁盘操作移到GUI。暂存关闭只清理受管文件，不删除配置的目录或父目录，配置和笔刷资源保持。

README的Rust职责表按core/Cargo.toml补齐全部八个运行库和paint-api-gen构建工具，区分paint-ops纯计算、paint-gpu执行设备与Qt显示职责。验收见[暂存目录自动创建](../validations/platform/scratch-directory-creation-validation.md)。

性能对照使用 Windows 新建工作树时，core.autocrlf=true 将源文件与已生成头转换成 CRLF；cbindgen 的字节一致性检查随之失败。源目录当前手工生成的 LF 头能通过，不等于新检出能通过。

比较两种方案：检查时宽松忽略换行，会让生成物存在多个字节版本且无法防止源注释影响生成结果；在仓库声明文本 LF，则源文件、生成头和构建脚本在三个平台相同。采用 .gitattributes 的 text=auto eol=lf，常用图片/归档/字体显式 binary。保留 cbindgen 严格字节检查，不修改用户全局 Git 配置，不改变 ABI。

验收先于实现：真实临时 Git 仓库设置 autocrlf=true，检出中文 Rust/C 头/QML/Python/Markdown/PowerShell 文本后验证 LF，PNG/ZIP 逐字节保留；tooling unittest 接入既有 tools/check.py。独立新工作树直接执行生成头 --check，不先重写生成头。最终完整 check 通过后独立提交/推送。

构建输出必须按工作树隔离，多个不同源码目录共享同一个 Cargo target 可能污染相同包名的顶层库文件；对照构建使用独立输出目录。依赖下载可共用 CARGO_HOME，产物不跨工作树共用。先前性能对照产生的共享缓存已清理并从主源码重新完整验证。

验收结果：真实 autocrlf=true 检出 unittest 通过，中文文本 LF 与图片/ZIP 字节保留。python tools/check.py 完整通过（artifacts/build-reproducibility-check.log）：171项 Rust、严格头检查、四组 C/C++ ABI、30项 Qt 与12项两倍 DPI，六个 CTest 无失败。新建独立工作树直接 cargo fmt --check、paint-api-gen --check 通过，未运行生成模式；git ls-files --eol 确认为 i/lf w/lf attr/text=auto eol=lf（artifacts/fresh-checkout-header.log）。未改变全局 Git 设置、ABI 或绘画路径。工具测试使用 Python 标准库 unittest，不是插件 pytest；Python 插件仍最后实施。

## CLion / 无终端 MSVC Ninja 配置修复（2026-10-09）

中文 MSVC 的 `/showIncludes` 输出随启动环境变化：终端可输出 UTF-8，无终端进程可输出 ANSI/GBK。原有 `ENCODING NONE` 探测把 GBK 前缀写入 CMake 内部 UTF-8 字符串，导致 `CMakeFiles/rules.ninja` 未落盘；首次配置甚至可能返回成功，再次配置则在 Ninja `recompact` 处失败。清缓存只能暂时改变症状。

设计：仅在 MSVC + Ninja 配置中，用 Python 编译 launcher 将编译器输出统一成 UTF-8；已是 UTF-8 的字节保持不变，否则用 Windows ANSI 编码解码后转换。依赖探测使用同一 launcher，实际提取并验证 `/showIncludes` 前缀，失败明确报错。实际编译将探测到的依赖前缀替换为 ASCII；CMake 4.3 特殊处理前缀，按配置进程的控制台编码写入，因此 ASCII 保证在 IDE 和终端之间切换也一致。逐行转发输出并保留编译器退出码、已有用户 launcher；不改全局环境、Qt/ABI、Rust 或 Ninja 工具链，不硬编码开发机路径。不依赖开发机是否安装英文 MSVC 语言包。

实施前验收条件：launcher 测试覆盖 UTF-8 原样保留、ANSI 输出转换、stderr 及非零退出码；Windows tooling 测试从 `DETACHED_PROCESS` 启动包含真实 MSVC C/C++ 编译的最小项目；配置和重新配置都成功且 `rules.ninja` 存在；Ninja 正确记录依赖，修改公共头后 C/C++ 对象均重编译，无修改再次构建不编译。保留用户 launcher 并确认执行。非 Windows 或缺少 MSVC/Ninja 的环境明确跳过该平台专用测试。最终执行完整 `python tools/check.py`，另在用户 CLion 构建目录验证 Debug UI 配置和构建。

验收结果：6 项 tooling 测试全部通过，无跳过；真实 MSVC C/C++ 无终端配置及重新配置成功，中文/空格路径、已有 launcher、Ninja 头依赖、修改头后重编译与无修改不编译均通过。用户 `cmake-build-debug` 从无终端进程重新配置并完成 Debug UI 构建，四组 ABI 通过；`rules.ninja` 前缀稳定为 ASCII。

完整验证在独立工作树以 `f7dbbc4` 为基线，仅加入本次修复，使用独立 Cargo target：`python tools/check.py --qt-prefix <Qt MSVC x64 SDK>` 通过，171 项 Rust、严格生成头、四组 ABI、Qt 35 项与两倍 DPI 16 项，6 个 CTest 无失败（`artifacts/clion-isolated-full-check.log`）。本轮主目录有同时进行的菜单 UI 修改：初次完整检查因菜单半径断言与 QML 不一致失败（`artifacts/clion-msvc-full-check.log`）；Debug Qt 首次无终端运行中断，普通终端复测仍有 UI 断言失败（`artifacts/clion-debug-check.log`、`artifacts/clion-debug-terminal-tests.txt`）。这些结果不算完整通过，也不将同时进行的 UI 修改纳入本次提交。独立工作树的 Qt 逐项报告另存 `artifacts/clion-isolated-qt-ui-tests.txt` / `artifacts/clion-isolated-qt-ui-highdpi.txt`。


## Windows Release 复制与目录权限（2026-10-18）

分发时复制完整的 Release 目录，保留 Qt DLL、插件及 QML 运行库；构建会同时将根目录 LICENSE 复制到应用旁。项目原创代码与文档采用 CC BY-NC-SA 4.0，第三方运行库和素材保留各自许可。

程序配置与笔刷资源使用当前用户配置目录，默认暂存使用系统临时目录，不要求在 EXE 所在目录写入。复制后出现 `核心错误：拒绝访问 (os error 5)` 时，应核对运行身份、已保存的暂存路径，以及配置与临时目录权限。

本次用户桌面副本的原因是桌面目录带有可继承的 Windows 低完整性标签，EXE 继承后以低完整性运行，不能写入普通中完整性的用户配置和临时目录。它与 Qt DLL 缺失、CWD 或 VBR 版本无关。可用 `icacls <Release路径>` 与 `icacls <Release路径>/drawverse.exe` 核对 Mandatory Label。处理应限制在已确认的应用副本范围，恢复其普通中完整性标签，或重新部署到具有正常权限的目录；不要放宽用户数据目录的写权限，也无需把应用设为管理员运行。

本机仅修正了用户指定的 `C:/Users/MOHAN/Desktop/Release` 及子文件，保留桌面本身的权限。新建的其他桌面文件夹仍可能继承桌面的低完整性标签，遇到同类问题需要重新核对。详见 [Release 目录权限验收](../validations/platform/release-directory-permissions-validation.md)。
