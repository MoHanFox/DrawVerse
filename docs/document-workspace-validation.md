# 多文档工作区与面板列验收（2026-10-09）

对应 document-workspace-design.md。Windows 11、MSVC x64、Qt 6.8.3；实现限于 C++ / QML UI，Rust 与正式 ABI 1.8.0 未修改。

## 可见结果与交互

面板之间显示 1px Theme.line，分隔条有额外的透明鼠标命中范围。垂直工具列只有第一组显示移动与图标折叠栏，内部为分隔线；小栏不再有关闭面板组按钮。各组全部折叠为标签时按最小高度连续排列，浮窗缩短且记住展开尺寸；整列可折叠为图标，点击恢复。关闭按钮默认透明，悬停红色，按下深红色；主窗右上圆角和菜单栏原生背景模糊保留。顶部选项栏右侧的性能设置和创建面板按钮移除；导航菜单已有“编辑 → 性能与暂存盘…”与“工作区 → 自定义面板…”入口，回归从对应菜单动作打开真实对话框。移除图层面板的“蒙版 / 黑 / 白”快捷行，保留蒙版密度、正常蒙版操作与颜色绘画。

工具浮窗与文档浮窗直接移动实际 QWindow，持续更新位置和当前停靠预览；自身不作为目标，Escape 恢复原树与几何。工具条可停在工作区边缘和任意工具面板左右侧。工具不能停到画布上，画布不能加入工具树。画布标签可组成独立文档浮窗，仅主文档区顶部 28px 接受归位；文档全部浮动或关闭时保持 #17191C 空白区域，工具侧栏不会填满它。

新建、打开添加独立 PaintCoreClient / session，每个画布保留自己的绘画、历史、文件任务、图层和视口。共享工具随活动文档切换，画布实例绑定固定客户端。保存或关闭确认阻止活动文档切换；退出依次处理未保存文档，等待所有客户端后台线程停止。打开失败移除失败文档；后续保存失败不会复用打开失败处理或关闭已有文档。最后一个文档关闭后可以重新新建。

## 可执行验证

Windows 原生实窗运行以下 Qt Test，用真实浮窗位置、鼠标抓取、桌面采样与原生窗口区域验证；四个测试槽及初始化/清理均通过：

- floatingCanvasAndToolsDockAcrossNativeWindows：独立绘画/撤销、ORA 保存和重开、保存/打开失败保护、切换视口、浮窗文档标签组合、压感、实际拖动/取消/顶部归位、关闭及全部线程停止。
- panelColumnsUseOneHeaderAndDragTheActualWindow：唯一顶部栏、连续折叠高度、图标列与尺寸恢复、原生浮窗跟随鼠标、即时几何与候选、排除自己、取消与放入主窗。
- menusReopenWithoutGhostsAndFloatBesidePanels：重复开关画面相同、浮窗侧面菜单、点击面板或主画布空白关闭、最大化与还原后重新激活菜单。
- nativeMenuBarBlursLiveBackdropWithoutMenuSnapshots：原生上方圆角、最大化工作区铺满、关闭按钮真实悬停红色；测试条纹背景的模糊后最大相邻颜色差小于关闭模糊时的一半。

实窗日志：artifacts/document-workspace-native.txt；画布标签截图：artifacts/document-workspace-native.png；实际菜单栏桌面截图：artifacts/document-workspace-menu.png.title.png；完整主窗截图：artifacts/document-workspace-menu.png.render.png。这些是本机构建产物，不提交。跨窗口模拟拖动的 Qt Test 坐标警告表示指针离开源窗口；QML 警告列表为空。

最终源码执行 `python tools/check.py` 完整通过：Python 真实 Git 检出/工具测试、Rust fmt/clippy/全部工作区与文档测试、Release 构建、cbindgen 严格字节检查、C11/C++20 静态/动态 ABI 及 Qt 普通/两倍 DPI 六项 CTest 全通过。Qt 普通 38 个测试槽（含初始化/清理共 40），两倍 DPI 16 个测试槽（含初始化/清理共 18）；CTest 总计 40.48 秒。日志 artifacts/document-workspace-final-check.log、build/verify-ui/qt-ui-tests.txt、build/verify-ui/qt-ui-highdpi.txt。

多文档退出任务与保存失败后恢复交互的补充实窗回归通过，日志 artifacts/document-workspace-native-close.txt。最终删除顶部按钮后，storageSettingsDialogShowsAppliedAndPendingValues 与原生背景模糊测试再次在 Windows 实窗通过（含生命周期共四项），日志 artifacts/document-workspace-native-final.txt；上述完整主窗和标题截图已重新生成并检查，顶部重复按钮消失。

`cmake --build build/qt --config Release` 构建及 Qt 部署通过，日志 artifacts/document-workspace-launch-build.log；可直接运行 build/qt/bin/Release/drawverse.exe。`git diff --check` 通过，新增与修改文本均为 LF；构建输出、运行日志与截图不提交。

## 持久化与验证边界

布局 v6 保存工具分割树、几何与图标组。回归覆盖四边停靠、1px 几何、无效树/比例/窗口/图标 ID 原子拒绝，以及旧 v1–v5 迁移；v5 原有主窗布局保持，原工具/画布混合浮窗中的工具留在浮窗，画布回到主文档区。布局只恢复工具布局，不自动重新打开文档像素或保存未保存的文档。

本次 Windows 普通 DPI、两倍 DPI 和原生桌面均验证；macOS/Linux 原生窗口、真实数位板硬件与跨平台 CI 仍待对应平台验收。没有增加 Python 插件或宣称已完成后续 GPU/色彩/PSD 工作。
