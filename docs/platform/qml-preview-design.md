# Qt Creator QML 源码预览

用户需要在 Qt Creator 编辑 `ui/qml` 并直接查看界面。正式应用通过 qrc 加载界面，且 QML 依赖 `PaintCanvas` 等 C++ 类型及 `PaintClient`、`Workspace` 上下文，因此不使用独立 qmlscene 执行 Main.qml，也不另建一套模拟业务界面。

`drawverse_qml_preview` 是按需构建的独立开发目标，复用正式 main.cpp 的样式、C++ 类型注册及后台初始化。仅此目标定义 `DRAWVERSE_QML_SOURCE_DIR`：读取磁盘 QML、关闭 QML 磁盘缓存、使用每次会话独立的临时偏好/布局，并允许 Qt Creator QML 调试服务；正式应用仍从 qrc 启动。`qml-preview` 构建预设负责构建和部署对应运行库，不加入默认应用构建，不改变 Rust、ABI、文档或笔刷格式。

`--watch` 使用 QFileSystemWatcher 监听目录及 QML/JS/qmldir/图片。300ms 单次定时器合并事件，内容 SHA-256 比较过滤非内容变动，重新注册文件和目录应对编辑器原子替换。无定时扫描；只有保存事件后读取内容。变化时退出当前事件循环，让引擎、所有文档后台、工作区及临时目录依次析构，然后用同一个 QGuiApplication 启动全新的预览会话。这样不复用旧文档的上下文指针，也不强杀进程或丢留临时目录。重载期间没有跨会话共享后台/纹理，未保存的临时绘画不保持。

加载失败后 Live 模式显示简单错误窗口，Qt Creator 应用输出保留 QML 位置诊断；继续保存可重新尝试。普通模式加载失败返回非零。不存在的入口在创建后台前返回2。正式主程序的原截图/benchmark 入口和正常设置路径保持。

组件台 `ui/qml/preview/ComponentGallery.qml` 复用欢迎页、笔尖方向盘、参数滑条、画笔列表、画笔设置和图层，使用真实后台，可改容器宽高/缩放/背景。它只从源码入口运行，不加入正式应用资源。共享工作区配置提供完整界面、临时画布及组件台的运行项，其中前两项为监听模式；不写用户的 project.json.user，不以 IDE 的默认 MinGW/MSYS Kit 重编译 MSVC 程序。

原计划直接以 SDK qmlpreview 启动开发宿主，但本机测试中未能可靠刷新根源码，故共享入口采用上述独立监听机制。开启开发调试服务不等于已验收 Qt Creator 自带 QML Preview 的完整重载行为。实际使用见 [QML 编辑说明](../../ui/qml/README.md)，证据见 [验收](../validations/platform/qml-preview-validation.md)。

限制：保存后全会话重建，不承诺保留文档或控制台调试状态；启动后台和加载 QML 的耗时仍存在。显式文件保存/导出仍写用户选择的位置。原生 C++、shader 和模块依赖变化需重编译。目录工作区的静态类型推导可能不完整，建议需要 C++ 导航时打开根 CMake 项目；不是 Qt Designer 拖拽表单支持。
