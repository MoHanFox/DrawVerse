# Release 目录权限与许可验收（2026-10-18）

## 范围与原因

用户将完整 Release 复制到 `C:/Users/MOHAN/Desktop/Release` 后，界面出现核心错误和 `拒绝访问 (os error 5)`，原目录可运行。原构建与桌面副本 EXE 的大小和时间一致，Qt6Core.dll 的 SHA-256 一致。

实际检查表明桌面、Release 及 EXE 带有可继承的 Low Mandatory Level 标签；从桌面副本启动的进程 TokenIntegrityLevel 为 4096（低），原目录进程为 8192（中），两者均不是 AppContainer。低完整性副本不能写入普通中完整性的用户配置和临时目录。修正前，从桌面运行 Qt 存储用例连日志文件都无法创建，预览进程 0.09 秒退出 1，无截图；原目录同项存储用例及原生预览通过。

程序现有配置、笔刷资源及暂存路径不依赖 EXE 所在目录，此次没有修改 Rust 核心、VBR、存储配置或全局系统安全策略。

## 本机修复与复核

只将用户指定的桌面 Release 及子文件恢复为普通中完整性，保留桌面本身的标签和原有用户访问列表。共处理 1,411 个项目，0 失败；复核桌面 EXE 已为 Medium Mandatory Level，启动后的进程完整性为 8192。无需管理员运行应用，也未放宽配置或临时目录写权限。

| 检查 | 结果 |
| --- | --- |
| 桌面副本 `drawverse.exe --preview`，CWD 为桌面 Release | 退出 0，1.97 秒完成实际三笔绘制并生成截图 |
| 桌面副本 `storagePreferencesPersistOnlyAfterValidationAndApplyAfterRestart` | 通过，覆盖设置校验、保存及重启应用 |
| 桌面副本 `invalidStartupStorageCanBeCorrectedFromQml` | 通过，覆盖错误配置提示及界面纠正 |

Qt 报告含初始化与清理为 4/4 通过，其中业务用例 2 项。本机忽略日志与截图位于 `artifacts/portable-desktop-{after.log,after.png,storage-after.txt}`，权限处理日志为 `artifacts/portable-integrity-repair.log`，前后进程身份对照脚本为 `artifacts/inspect-portable-tokens.py`。

桌面父目录的低完整性标签仍在，新建副本可能重新继承，不能据此宣称桌面所有程序已修复。排查说明见 [构建与目录权限](../../platform/build-reproducibility.md)。

## 许可与项目入口

根目录 LICENSE 使用 Creative Commons 官方仓库提供的 CC BY-NC-SA 4.0 International 完整法律文本，原创代码与文档的适用范围及第三方许可边界写入根目录 README。README 明确非商业、署名与相同方式共享要求，并说明它不属于 OSI 意义上的开源软件许可。

README 的 DrawVerse 标题下新增使用警告，说明开发阶段的已知失败、暂存与撤销不等于自动保存/崩溃恢复，以及及时保存与备份的原因；删除用户指定的双色入口句子。CMake 默认构建附带 LICENSE 到应用目录，验收见下方补充结果。

此次不涉及绘画行为变更，不重复全套 Rust、ABI 或跨平台验证；此前已知失败仍以 [源码整理验收](source-layout-validation.md) 为准。


## 补充构建与文档核对

`cmake --build build/qt --config Release` 完整构建及 Qt 部署退出 0，日志为 `artifacts/license-release-build.log`。新增默认目标 `drawverse_license` 已执行，输出目录 LICENSE 与根目录逐字节一致，内容与取得的官方原文一致，仅规范化文件末尾空白；SHA-256 为 `1349a4b6148492b44f629e64eed676612e234fe9a839e4f3b277c1482c8849f1`。用户现有桌面副本也已附带同一 LICENSE。

全仓 1,027 处本地 Markdown 链接解析通过，验收目录共 199 份记录，索引数量一致。README 警告位于标题后第一个正文块，指定句子已删除，`git diff --check` 通过。
