# 在 Qt Creator 中编辑和预览

在 Qt Creator 20 中选择 **文件 → 打开工作区（Open Workspace）**，打开这个 `ui/qml` 文件夹。项目读取 `.qtcreator/project.json`，不需要把每个 QML 文件创建为项目，也不需要在这里运行 qmlscene。

运行目标选择器提供：

| 运行目标 | 用途 |
| --- | --- |
| DrawVerse - Live QML preview | 完整界面，保存 QML/图片后自动刷新 |
| DrawVerse - Live component gallery | 控件预览台，保存后自动刷新 |
| DrawVerse - QML source preview | 完整界面，手动重新运行时读取源码 |
| DrawVerse - QML canvas preview | 自动建立一个临时画布，方便查看工作区和工具栏 |
| DrawVerse - QML component gallery | 控件预览台，手动重新运行 |

选择目标后按 **Ctrl+R**。Live 模式监听源码目录，保存后约 300ms 合并文件变动，再销毁旧的 QML 引擎和后台、创建新的临时预览会话；这不是保留文档状态的热替换。预览内未保存的绘画和面板调整会清空，预览适合调界面。首次窗口和后台准备仍需要时间，300ms 不是端到端显示延迟。

控件预览台包含欢迎页、笔尖方向盘、参数滑条、画笔列表、画笔设置和图层，可调整容器宽高、显示缩放和背景。它复用真实组件及真实 C++ 类型和后台，不维护另一套模拟界面。QML 出现语法错误时，Live 模式显示错误窗口；在应用输出查看具体位置，修正并保存后自动重试。

## 首次构建与后续更新

此工作区不配置编译步骤，避免按运行按钮就编译 Rust/C++。第一次或修改 C++ 后，在有 MSVC 环境的终端中从仓库根目录执行：

```powershell
cmake --preset build
cmake --build --preset qml-preview
```

开发程序输出为 `build/bin/Release/drawverse_qml_preview.exe`，本次接入已构建。只修改 QML、JS、qmldir 或图片时无需重编译。修改 C++、shader 或增加 Qt 模块依赖后应停止预览并重新构建；shader 仍需编译成 qsb。新增 QML/图片供正式程序使用时，还要更新 `ui/CMakeLists.txt` 的资源清单。SDK 路径沿用 `.tools/qt/6.8.3/msvc2022_64`。

命令行等价用法（从仓库根目录）：

```powershell
& .\build\bin\Release\drawverse_qml_preview.exe --watch
& .\build\bin\Release\drawverse_qml_preview.exe --watch --qml-entry preview/ComponentGallery.qml
& .\build\bin\Release\drawverse_qml_preview.exe --canvas
```

`--qml-root <目录>` 可以指定另一份 QML 源码，默认使用构建时的 `ui/qml` 绝对路径。`--qml-entry <文件>` 指定该目录下的 Window/ApplicationWindow 入口；普通 Item 控件应放入预览台或一个窗口包装中。Qt Creator 使用目录工作区时，命令行相对路径基于 `ui/qml` 项目目录解析，工作目录为仓库根目录。

## 设置与边界

每次启动及每次 Live 刷新都使用独立临时笔刷偏好和工作区布局，正常退出或刷新后删除；正式软件的偏好不被预览覆盖。预览中的显式保存/导出仍会写入你选中的文件。正式 `drawverse.exe` 继续读取打包资源，保持原启动和设置行为，不启用开发预览调试服务。

这是运行真实界面的源码预览，不是把现有复杂 QML 转为 Designer 拖拽表单。完整 C++ 类型导航/语义分析建议打开仓库根 `CMakeLists.txt`，使用 MSVC x64 和项目的 Qt SDK；只打开这个目录时，编辑器可能无法完整推导 `DrawVerse` 动态注册类型和上下文对象，但预览程序中这些类型已真实注册。不要用 Qt Creator 默认的 qmlscene 代替此运行目标。

`.qtcreator/project.json` 是共享运行配置，`project.json.user` 是个人 IDE 配置，不提交。移动源码目录或更新 Qt SDK 后需重新构建预览程序；如果已有工作区未显示新目标，请关闭并重新打开工作区。
