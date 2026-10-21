# 暂存目录自动创建验收（2026-10-21）

对应[构建复现说明](../../platform/build-reproducibility.md#2026-10-21构建后启动报核心错误8)。Windows x64 / Qt6.8.3 MSVC / ABI1.75，state45和VBR1.0不变。

使用原Release的C ABI复现：已保存的D:/DrawVerse/tmp不存在，paint_core_create_with_storage返回8与Windows错误2；同一DLL选择系统临时目录返回0。修复后调用同一接口，原路径从不存在变为自动创建，返回0，销毁核心后目录保持，未修改用户配置。记录于artifacts/scratch-create-user-path.json。

共享ScratchSpace在初始化时递归创建目录；文件占位、离线磁盘或权限不足仍失败，错误携带路径与原因。创建不改变原空间预算、租约、命名空间识别、已知文件清理边界或旧配置，既有后台启动/设置检查/分页/历史均复用。

验证结果：

- paint-storage cleanup九项通过：新增嵌套中文目录创建、重开/关闭后保留，以及文件占位不覆盖/错误标识路径；原租约、进程退出清理、未知文件、链接和容量保护均通过。日志artifacts/scratch-create-core.txt。
- 软件与D3D11各四项Qt通过：缺失目录启动/真实绘画/撤销重做/再次启动、设置界面、保存仅成功后发布并在重启生效、无法创建时从QML修正。旧测试将“缺失目录”失败输入换为真实文件占位，保持失败原子性断言并检查路径。日志scratch-create-{workflow,ui}-{software,d3d11}.txt，无QML警告。
- 原样执行README两条CMake配置和完整Release构建通过，含Qt DLL/QML部署与生成头检查；四组C/C++静态/动态ABI通过，paint-storage全目标严格Clippy及workspace格式检查通过。日志scratch-create-{configure,build,abi,clippy}.txt。标准build/qt/bin/Release/drawverse.exe已更新。
- README职责表核对core/Cargo.toml：八个运行库与paint-api-gen共九包齐全，区分数值算子、wgpu执行与Qt显示。

未跑全量测试或跨平台；本次不改变磁盘满、权限失败、暂存配额等错误处理，不提供历史崩溃恢复。
