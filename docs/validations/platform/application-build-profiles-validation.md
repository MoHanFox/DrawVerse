# 应用构建配置验收（2026-10-21）

对应[构建入口设计](../../platform/build-reproducibility.md#当前应用构建与运行入口2026-10-21)。标准用户交付目录从build/qt/bin/Release改为build/bin/Release，Debug为cmake-build-debug/bin/Debug；历史证据路径不改写。

CMakePresets.json的build和run均启用UI，分别Release/Debug；共享.run配置的build只构建drawverse_runtime，run构建同目标后执行drawverse。应用专用目标包含LICENSE和对应Qt运行库/QML部署，完整默认构建仍部署测试及offscreen插件。共享XML最初使用codemodel子项目DrawVerseUI；CLion实际读取后规范化为根项目DrawVerse，已保留其写回结果并核验profile/目标映射。本机原Debug/UI关闭的旧profile已停用，新build/run开启且使用指定目录；没有修改用户全局工具链。

验证：

- cmake --list-presets=all成功；实际执行build预设配置/构建、README的-B build手动配置/完整Release构建，以及run预设配置/Debug应用构建。应用目标构建日志没有Qt测试编译；完整Release日志包含测试编译、链接及部署。生成头检查无漂移。
- 两个产物均从独立临时工作目录运行--preview，PATH移除Qt SDK路径，D3D11实际绘制、截图并正常返回0；参数使用隔离配置，不写用户配置。Release截图266521字节、Debug257981字节，仅确认启动及绘制，不是性能基准。
- 新Release目录下四组C/C++静态/动态ABI通过；新目录的默认暂存提示、缺失目录创建、欢迎加载状态、欢迎生命周期、直接历史五项在软件/D3D11各通过，无QML警告。

本机证据：artifacts/build-profile-{configure,build,release-launch,debug-launch,abi}.txt、build-manual-{configure,build}.txt、run-profile-{configure,build}.txt，以及build-profile-workflow-{software,d3d11}.txt；两张build-profile-*-preview.png可复核实际绘制。未运行全量测试或跨平台，未声称自动操作了CLion的构建/运行按钮。Debug仍链接既有Rust release核心，ABI1.75/state45/VBR1.0不变。
