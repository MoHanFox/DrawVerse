# 参考图 QML 与停靠验收（2026-10-09）

## 普通高斯菜单与紧凑行距（2026-10-09）

菜单栏底色恢复 #202226，子菜单恢复 #1c1e21，底色不透明度 75%。去掉高光和亮边；菜单栏上方两角圆角、下方直角，最大化时无圆角；子菜单四角圆角。菜单项随文字高度布局，上下各 3px，总文字行间距 6px，保留字号与 1.1 倍缩放。

FrostedSurface 对打开前的 Qt 所属窗口显示帧执行有界、可分离高斯模糊；缓存最长边 512px，菜单关闭即释放，没有持续抓帧。Qt 6.8+ 显式使用 Popup.Item，旧版本保留同窗弹出路径，主窗和浮窗共用材质。窗口拖动、系统按钮及原有菜单动作保留。

python tools/check.py 完整通过（artifacts/gaussian-menu-final-check.log）；随后修正 Windows 自动化测试需先激活工具浮窗的前置条件，最终六组 CTest 全通过（artifacts/gaussian-menu-final-ctest.log）。34 项 Qt 测试、14 项两倍 DPI 测试通过，不计初始化/清理；包含高斯核扩散、常色保持、缓存上限、75% 底色、上下角形状、6px 行距、菜单缓存释放与键盘回归。两项既有像素断言临时断开 CanvasItem，让手动 128px 读帧独占视口槽，消除与实时 DPR 请求的竞争，不修改应用绘画行为。

实际 Windows 菜单交互单测通过（artifacts/gaussian-menu-final-windows.txt），工作区菜单截图 artifacts/gaussian-menu-final.png 已检查。build/qt 常用启动目录已同步构建；Rust、ABI、文档像素与布局格式未修改。

## UI 整体放大 1.1 倍（2026-10-09）

应用与 Qt Test 在创建 QGuiApplication 前共用 UiScale.h，将 Qt 全局缩放乘以 1.1，保留系统 DPI、外部倍率和现有 QML 逻辑布局。新增真实 QML 回归覆盖主窗、颜色浮窗、工具浮窗的 DPR、截图物理尺寸和布局保存恢复。普通 offscreen 为 1.1，两倍 DPI 为 2.2，原有鼠标/数位板、选区、图层及跨窗口拖放全部通过。

python tools/check.py 完整通过：31 项 Qt 测试、13 项两倍 DPI 测试（不计初始化/清理），Rust、严格生成头与四组 C/C++ static/shared ABI 检查均通过，六个 CTest 目标无失败。日志：artifacts/ui-scale-check.log。Windows 主窗截图由 2220×1170 增至 2442×1287，宽高均为原来的 1.1 倍；artifacts/ui-scale-before.png / ui-scale-after.png 已视觉检查，文字、控件和排布正常。build/qt 常用启动目录同步构建；Rust/ABI 与布局文件格式未修改。

## 原参考图与停靠验收

以用户 HTML、预览-PS化面板.png、编辑页.svg 和最新停靠示意为视觉依据。设计见 reference-ui-design.md。单面板平直标题，多面板上沿圆角标签；选中标签与内容同色，非选中更暗。Logo 保持原图，图标为 Qt Quick Shapes 自绘矢量。主窗无白色系统标题栏；系统移动、缩放、最大化/还原仍由 Qt 原生窗口接口处理。

新启动使用参考图布局：右侧历史/图层，颜色独立浮窗、画笔与画笔设置折叠两行。工作区菜单可切换参考图或双列布局；新版保存的自定义布局保留，旧版迁移保留自定义面板并清理笔记。颜色支持色相环、三角选择、HSB 数值和双击色块 HEX 输入。历史列表点击按真实深度逐步异步撤销/重做，当前仍用“操作 N”，不虚构操作名称。

原生 QDrag 跨窗口合并、标签排序、上下拆分、整组移动、关闭/重开、图标列临时展开及浮窗尺寸恢复已覆盖。浮窗按 ID 保持实例，不随无关面板结构变化重建。工具条顶部握柄可拖出，左侧边缘接收归位；双击握柄浮动/归位、窗口菜单归位和关闭归位均可用。Ctrl/Command 暂停停靠，Esc 取消；布局 v4 保存工具条位置。

## 验证

- `python tools/check.py` 完整通过：165 项 Rust 测试、cbindgen 头漂移检查、C11/C++20 static/shared 四组 ABI、26 项 Qt 测试及 10 项两倍 DPI 测试（Qt 日志含初始化/清理为 28/12）。六个 CTest 目标全部通过。
- Windows 原生 Qt 定向测试通过：参考图预设、工具条实际浮动/跨窗 DropEvent、切换工具、布局恢复、标签颜色、历史点击以及最大化/还原，无 QML 警告。原有蒙版、剪贴、拖放、鼠标/数位板、保存流程完整回归。
- `artifacts/reference-final.composite.png` 为实际主窗与两个实际原生浮窗按屏幕坐标合成的预览；独立窗口截图为 `.color.png` / `.brush.png`。并非设计稿或完整桌面截图。已目视检查比例、HSB/色轮、直角内容与圆角标签、图层可见性图标。
- `python tools/measure_painting.py build/qt/bin/Release/drawverse.exe artifacts/reference-ui-performance.json`：600 点，DPR 1.5，301 次画面更新，更新间隔 p50 16ms / p95 17ms，GUI tick p95 9ms，进程累计 CPU 234.375ms，墙钟 6506.9ms，无错误。输出 1054×703，与旧布局 1529×1019 不同，因此不作严格 CPU 百分比对比。缓存和异步绘画路径未修改。

构建/运行使用 README 中命令。Windows 启动 `build/qt/bin/Release/drawverse.exe`；统一检查输出在 `artifacts/reference-ui-final-check.log`，Qt 详情在 `build/verify-ui/qt-ui-tests.txt` / `qt-ui-highdpi.txt`。

## 当前边界

Windows 本机验证完成，macOS/Linux 尚待 CI 与硬件验证。停靠是左右列纵向组、独立浮窗和浮动标签组，尚不支持任意二维停靠树或多个浮窗联动移动。仅圆形压感笔刷已接核心；用户补充的“画笔库选择 → 对应画笔设置”将作为下一独立模块完成，当前不声称拥有截图中的复杂笔刷。未改 Rust/C ABI，不创建虚假 pytest。
