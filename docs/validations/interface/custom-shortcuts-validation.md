# 自定义快捷键验收

- Qt 单元验证：规范化、冲突拒绝、清除、默认恢复、持久化重载及保留交互键。
- Qt QML 验证：编辑菜单打开可编辑列表，真实渲染无警告。
- 构建 UI 与运行定向测试，结果在完成后记录；不以定向验证宣称全量通过。

## 2026-10-29 结果

- `cmake --build build --config Release --target qt_ui_tests` 在 MSVC x64 开发环境构建通过；生成 ABI 头与 shader 检查通过。
- `QT_QPA_PLATFORM=offscreen`、`QT_QUICK_BACKEND=software` 下运行 `qt_ui_tests customShortcutsPersistAndRejectConflicts editMenuOpensKeyboardShortcutList bucketOptionsMouseTabletAndFloatingCanvasFillOnce wandOptionsSelectRealMaskOnceAcrossMouseTabletAndFloatingCanvas textFieldsKeepSpaceAndHistorySelectsDirectly`：5 个功能测试通过，加初始化/清理共 7 passed、0 failed。
- 验证窗口真实录入 F8、动态绑定更新；画笔改为 F6 后 B 不再切换工具，F6 生效。验证冲突、固定键拒绝、录入暂停、清除、恢复默认、重新构造对象读取持久化配置。油漆桶/魔棒含鼠标、数位板与浮动画布既有回归通过；文本空格与历史回归通过。
- 初次缺 MSVC 标准头环境构建失败，切换 VsDevCmd 后解决；初次重载因 Delete/Del 规范化误判失败，已修复。UI 夹具先后遇到 QVariantList 类型与虚拟化委托查找失败，纠正夹具；真实录入发现赋值刷新销毁委托后 root 引用失效，改为稳定对话框函数完成赋值，最终通过。原失败日志保留于 build/shortcuts-tests*.txt。
- 标准 `build/bin/Release/drawverse.exe` 被运行进程占用，链接 LNK1104；保留用户进程。复用 CMake 生成的实际链接命令，将 exe/pdb 输出重定向到 `build/bin/ShortcutReview`，运行库沿标准包复制，最终链接成功。独立包 `--profile-dir` + `--preview` 软件渲染冒烟退出 0，生成有效 PNG。
- 未跑统一 `python tools/check.py`、全部 UI 测试或实体键盘/D3D11 验收；不宣称全量通过。未修改用户实际快捷键配置、文档格式与 ABI。同期其他任务的代码修改保留。

## 2026-10-29 套索快捷键补充

新增 L 激活上次套索子类、Shift+L 切换自由/多边形套索；两项可配置且工具提示动态更新。命令目录共 36 项，既有窗口测试的数量断言同步更新。新增默认键避让已保存的用户绑定。按用户明确要求，本轮未运行测试或应用冒烟，仅构建 UI 并重新编译/链接独立 `build/bin/ShortcutReview/drawverse.exe`；标准 Release 程序仍在运行，未覆盖或关闭。此前 34 项的测试结果不等于本次变更已测试。
