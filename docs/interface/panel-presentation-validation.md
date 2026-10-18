# 面板状态与紧凑信息栏验收（2026-10-10）

设计见 [panel-presentation-design.md](panel-presentation-design.md)。仅 Qt/QML 修改，Rust、C ABI 1.8.0 不变；布局 v6 增加可选 panelViews，老布局可恢复。

图标首次点击打开、再次点击收回对应面板，其他面板仍打开；新打开的面板置顶，空白和失焦仍不收回。宽高与相对来源列的导航偏移按面板 ID 独立保存，侧边滑动或调整大小后收回/打开继承，布局保存/恢复同样继承。新屏幕与工作区边界会约束恢复位置。Qt 回归验证正常恢复、无可选字段的旧布局、非法宽高/极端偏移/未知面板不会污染布局，以及重置默认布局清除状态。分数 DPI 坐标统一取整，避免收回/打开偏移一个像素。

控件、面板与分隔描边统一 #1B1C1F，选中/焦点由填充与文字表达；拖放与几何选区的语义提示仍保留。工具标签顶部圆角 3px。停靠顶部拖动栏/标签栏背景 #05080A，浮动时背景 75% 不透明；原生工具窗口与面板根不再垫不透明背景，正文仍 #1C1E21 不透明。回归读取属性并检查实际像素：Qt 软件渲染 grabWindow 返回 RGB32，顶部半透明色预乘后为 (4,6,7)，其他保留 alpha 的后端按 (5,8,10,191) 验证；正文像素不变。检查了 Windows 实窗截图 panel-presentation-native.main/color/brush.png，诊断图片不提交。

底栏实时显示工具、共享画笔预设、画布尺寸和非蒙版图层数量，右侧缩小/百分比/放大/适配。验证橡皮擦选笔刷保留橡皮擦、移动/椭圆工具名称、图层新增、多文档切换及全部关闭的占位/禁用；中心放大/缩小文档点不漂移，范围和非法缩放输入有界，适配有效，视口操作不增加绘画撤销。工具条的适合窗口入口移除，菜单/F 保留；文件任务取消与状态提示仍可达。

Windows 11 / Qt 6.8.3 / MSVC 原生实窗四个行为槽通过，含生命周期共 6 项，3349ms：panelPresentationRemembersFlyoutsAndPaintsActualHeaderAlpha、compactStatusReflectsDocumentAndZoomControlsPreserveCenter、iconRailsSwitchSlideDismissAndDragPanels、floatingIconRailsKeepBackgroundSidePanelsAndReturnState。最后一项包含 Win32 SendInput 实际边框调整宽高与展开/收起列归位。连续重建窗口的测试筛选可见窗口并等待曝光，避免抓到延迟销毁的旧窗；实际跨窗拖动坐标提示不代表失败。日志 artifacts/panel-presentation-native-final.txt。

完整 python tools/check.py 验证包含 Python 工具/真实 Git 检出、Rust fmt/clippy/工作区与文档测试、release、cbindgen 字节检查、C11/C++20 static/shared ABI、Qt 普通及两倍 DPI。Qt 普通含生命周期 53 项、高 DPI 30 项，6 项 CTest 通过，50.06 秒，无失败/跳过。日志 artifacts/panel-presentation-final-check.log 与 build/verify-ui/qt-ui-tests.txt / qt-ui-highdpi.txt。已重新构建 build/qt/bin/Release/drawverse.exe 并运行 deploy_qt，部署日志 artifacts/panel-presentation-deploy.log。文本 LF/no-BOM、git diff --check；构建输出不提交。macOS/Linux 原生透明与窗口操作仍需相应实机验证。
