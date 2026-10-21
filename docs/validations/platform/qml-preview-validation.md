# Qt Creator QML 预览验收（2026-10-21）

设计：[QML 预览](../../platform/qml-preview-design.md)。使用：[ui/qml/README.md](../../../ui/qml/README.md)。Windows 11、Qt 6.8.3 MSVC x64；Qt Creator 工作区字段对照本机 20.0.2 安装附带的 project.json schema。

## 构建与配置

- `cmake --build --preset qml-preview` 首次构建/部署成功。后续联合构建 `drawverse_qml_preview_runtime qt_workflow_tests drawverse_runtime` 成功，更新标准 `build/bin/Release`，构建期间既有生成头 `--check` 通过。日志：`artifacts/qml-preview-build.txt`、`qml-preview-final-build.txt`、`qml-preview-atomic-build.txt`。
- JSON 解析、schema 已定义字段/必填项、五个可执行文件及工作目录检查通过；qml-preview/build/run 三个构建预设存在。未覆盖个人 IDE 配置；未使用自动 GUI 操作点击 Qt Creator，执行路径由实际预览进程回归覆盖。
- 四组 C/C++ 静态/共享 ABI 通过：`artifacts/qml-preview-abi.txt`。无 Rust/ABI/state/VBR 变化。

## 实际执行

`qt_workflow_tests` 新增四项，使用真实窗口/后台和临时 QML 文件：

1. 同一个磁盘入口分别写两版源码，两次运行读取各自标记，注册的画布/颜色轮/选区/文档类型可实例化；偏好和工作区均位于临时会话目录，两次路径不同，退出后目录删除。不存在的入口返回2。
2. 从磁盘加载控件台并轮流显示六种真实组件，每项能抓取非空窗口画面，无 QML 引擎警告。
3. 监听模式启动后，以 QSaveFile 原子替换源码，读取到第二版标记并正常关闭，覆盖 Qt Creator 的安全保存方式。
4. 加载真实 Main.qml，每次创建两个文档；原子保存成错误源码后显示错误入口，随后修正并保存可重新加载。三次临时会话全部清理，正常退出，没有 ReferenceError/TypeError 或残余创建诊断。

另运行既有编辑结束焦点八组合（主/浮文档、变换/液化、确定/取消）与欢迎页生命周期。软件及 D3D11 后端各15行通过（含 init/cleanup），均无失败或跳过；结果记录在 `artifacts/qml-preview-final-software.txt`、`artifacts/qml-preview-final-d3d11.txt`。

## 调查与修复记录

初版控件台 ScrollView 的 availableWidth/Height 与 contentWidth/Height 相互依赖产生两条绑定循环；改用显式 Flickable 尺寸后，六种组件均无上述警告。软件最终记录15行通过（含 init/cleanup），无失败/跳过。

SDK qmlpreview 原型能启动宿主，但源码刷新用例未收到第二版标记；未作为最终运行入口。失败探测遗留的自有进程已按实际 PID/完整可执行路径/测试参数核对并清理，后续预览直接使用本进程监听及会话重建，不存在外部工具父子进程链。

本验收不声称全测试、跨平台、Qt Designer 可视化拖拽、Qt Creator 原生 QML Preview 或保留文档的热替换通过。未将300ms事件合并时间当作实际显示延迟；首次 QML/资产加载性能待办保持。
