# 源码与契约目录调整验收

2026-10-18，Windows x64，Rust 1.99.0、MSVC、Qt 6.8.3，验收范围为本机可构建运行。设计见 [目录职责](../../platform/source-layout-design.md)。

## 变更范围

Rust 按实际模块责任拆分目录，保持 crate 名和既有公开导入路径；文档执行代码仍属于 Document。C++ 按功能目录组织，头文件 include 显式表达依赖。QML 按应用装配、功能与公共组件组织，更新 import、Theme 单例、资源和测试入口，目录依赖无环。

80 个已有契约文件按 FFI 和画笔领域迁移，源代码嵌入、生成器和文档引用同步更新。43 个历史 schema 移至 `paint-io/tests/fixtures/schema-history/`，内容及 `$id` 保持；contracts 只保留当前 VBR 1.0 外壳与组织交接契约。生成 C 头只改变契约路径注释，函数、DTO 和 ABI 版本保持。

## 已执行

| 验证 | 结果 |
| --- | --- |
| `cargo check --manifest-path core/Cargo.toml --workspace --all-targets --locked` | 通过，包含测试和 example 编译 |
| `cargo fmt --manifest-path core/Cargo.toml --all --check` | 通过 |
| `cargo run --manifest-path core/Cargo.toml -p paint-api-gen --locked -- --check` | 通过，Release 构建也执行生成头一致性检查 |
| `cmake --build build/qt --config Release` | 通过，应用、Qt 用例、C/C++ 静态及动态 ABI 目标与部署完成 |
| `ctest --test-dir build/qt -C Release -R '^abi_' --output-on-failure` | 4/4 通过 |
| `cargo test --manifest-path core/Cargo.toml -p paint-io --locked --test vbr --test vbr_image` | 17/17 通过，覆盖 1.0 写出、历史 minor 读取、缺项补全、图片与组树交接 |
| JSON Schema Draft 2020-12 校验 | 47 个 schema 自检通过；当前组树/交接合法与非法样例结果正确；当前 manifest 接受 1.0 示例、拒绝 writer minor44；历史快照与 HEAD 内容一致 |
| 原生 Windows 应用 `drawverse.exe --preview …` | 退出码 0，2.24s 完成隔离配置启动、实际画布三笔绘制和截图；最终日志无 QML 警告 |
| `git diff --check` | 通过 |

本机忽略产物为 `artifacts/source-layout-final-build.log`、`source-layout-vbr-tests.log`、`source-layout-final-launch.log` 和 `source-layout-final-preview.png`。Schema 检查复用既有 `.tools/schema-check` 和本机 Python 3.12，没有新增产品依赖。部署工具提示现有 Qt translations/catalogs.json 缺失，项目部署参数为 `--no-translations`。

## 范围外的交互检查

额外抽查原生 Qt 交互：`qmlMouseTabletAndFloatingWindows` 通过；`floatingCanvasAndToolsDockAcrossNativeWindows` 在模拟绘画后的 undoDepth 断言失败，`brushLibrarySettingsAndEnginePreviewsShareSelectionAcrossWindows` 在键盘设置半径断言失败。使用重构前已有 `build/bin/Release/qt_ui_tests.exe` 分别复核，两项在同一断言复现。它们不是本次新增失败，也没有在目录任务中修复。

offscreen 下额外 Qt 交互进程发生访问异常，因此不宣称 offscreen 交互通过。实际应用运行使用原生 Windows 平台。本次未运行全量 `tools/check.py`，不代表跨平台、实体设备、完整交互或 GPU 验收。

## 版本清理评估边界

缺项兼容仅说明 VBR 1.x 的功能解析不依赖 minor；不能外推为所有协议均无版本约束。当前四个 schema 的文件名已移除版本后缀，保留 `$id` 和 `$ref` 身份；历史快照的版本标识、schema 的引用身份、manifest 主版本、state、ABI、模型和 ABR 来源/转换器版本具有不同职责。本轮不以统一删除版本字段替代兼容设计。VBR 主/次版本常量统一于 `vbr.rs`，程序包与资源包写出均引用这两个常量。

常量与稳定文件名调整后再次通过 workspace 全目标检查、17 项 VBR 测试、47 个 schema 自检、Release 完整构建与 4 项 ABI 检查。原生预览退出码 0，2.31s 完成绘制截图。对应日志为 `artifacts/source-layout-vbr-constants-{tests,build}.log`，截图为 `artifacts/source-layout-constants-preview.png`。

## 当前契约展示统一

contracts 中的 VBR 文件名使用稳定名称、描述统一为 1.0，43 个开发快照移至测试数据目录。13 个湿混/ABR 示例文件名去除容易与 VBR 混淆的测试迭代号，其中 ABR 的独立版本明确写为 `converter-N`。测试使用本地相对路径引用快照，包含快照的测试名称明确为 historical；旧 minor 兼容和独立来源版本断言继续保留。

此次迁移后，workspace 全目标编译、格式检查、47 个 schema 自检（4 个当前契约加 43 个测试快照）、80 项专项测试、20 项 historical 快照测试、Release 构建和 4 项 ABI 检查通过。原生应用预览退出码 0，2.98s 完成绘制截图。专项覆盖 ABR 转换、起收笔、湿混双载色、VBR、资源包和来源记录。日志为 `artifacts/current-contract-{targeted,snapshot}-tests.log`、`current-vbr-contract-build.log` 与 `current-vbr-contract-launch.log`。

尝试全套 `paint-io --tests` 时，在 `brush_groups_state::groups_state20_invalid_downgrade_types_reference_cycles_and_projection_reject` 中断：测试将 state22 列为非法，而现有解码器允许 20–37 版本携带组织树。已逐字节确认解码器与 HEAD 一致，失败测试函数体也与 HEAD 一致，本次未修改 state 协议或这个断言。因此不宣称全套 IO 测试通过。记录见 `artifacts/current-vbr-contract-tests.log`。

## 验收归档与项目入口

198 份独立验收文档统一迁入 `docs/validations/`，按原有领域分类，文件名、日期、测试结论与未完成范围保留。设计文档留在原领域目录，跨目录相对链接与 AGENTS 路由同步更新。统一分类入口为 `docs/validations/README.md`，总入口仍为 `docs/README.md`。

根目录 README 重写为当前项目概览、功能、目录职责、构建运行、验证与 VBR 1.0 说明，移除过期的迭代流水和相互矛盾的能力描述。全仓 1,021 处本地 Markdown 链接解析通过，198 个目标文件齐全，旧验收路径引用归零。此次只调整文档，不重复此前已完成的应用构建与运行测试。

AGENTS.md 中 62 处 VBR 数字版本描述统一为 1.0，并清理旧 minor 功能门槛表述；逐项核对 ABI、state、behavior、converter 与 handoff 的独立版本标记保持不变。

工程工具检查 `python -m unittest discover -s tests/tooling` 6/6 通过，`git diff --check` 通过。迁移审计脚本与日志位于忽略的本机 `artifacts/`，不随提交上传。
