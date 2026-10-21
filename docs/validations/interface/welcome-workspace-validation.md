# 无文档欢迎工作区验收

日期：2026-10-21。对应[设计](../../../interface/welcome-workspace-design.md)。

## 已通过

- 新 Qt 工作流 `welcomeProgressUsesExplicitReadiness`：实际阶段文本、错误文本与完成后按钮切换。
- 新 Qt 工作流 `welcomeWorkspaceDocumentLifecycle`：正常启动零文档；没有工具详情、编辑工作区及浮动面板；等待当前笔刷 preparation 成功；新建/打开按钮实际鼠标点击打开各自对话框；新建的真实客户端绑定首 CanvasPane；第二文档；关闭最后文档；折叠栏弹出窗收起；浮动布局保持；再次打开 PNG；打开失败回欢迎页；关闭全部服务。
- 这两项及既有目录等价/跨文档偏好两项在 Windows 软件后端和真实 D3D11 各通过4项（含初始化清理6项）。D3D11 由 rendererInterface 断言确认。最新按钮为126×34胶囊形，D3D11截图已检查；软件前一版截图也检查，最新软件流程重跑通过。
- 软件变换回归2项：完成变换/液化后的快捷键焦点；空变换/取消状态。
- 首轮软件扩展 UI 回归中，偏好持久化、文档状态/缩放、停靠边缘、无效存储修正4项通过。
- 四个 C/C++ 静态/动态 ABI 生命周期程序顺序通过，生成头检查通过。完整 Release 及 Qt 工作流、UI、变换目标构建通过，输出 `build/qt/bin/Release/drawverse.exe`。最后按钮调整后重新构建 drawverse 与工作流通过。

## 未通过及范围

首轮扩展 UI 的另4项失败：`floatingCanvasAndToolsDockAcrossNativeWindows` 在模拟点击后 undoDepth 为0而非1；`panelTabsRetractWithoutExpandingRailsAndKeepSharedPresentation` 收到 LayerLock Connections 和 LayersPanel.mask 的警告；`editMenuOpensKeyboardShortcutList` 期望26项而实际35项；`brushLibrarySettingsAndEnginePreviewsShareSelectionAcrossWindows` 尺寸断言实际3而期望20。未完成同提交基线比对，不能声称这些是既有失败或全量回归通过。面板项将在后续合并任务中复核。首轮泛化的 flyout 隐藏监听已收窄为显式无文档状态，欢迎页专项再次通过。

欢迎页生命周期退出时 Qt 另报告有2个异步对象仍在创建；专项流程断言通过，incubationController计数已归零，但该退出警告仍未消除。

没有运行全量 check/跨平台/实体设备或首次加载时延基准。欢迎页显示真实阶段，不等同于消除同步偏好恢复的全部阻塞。Rust算法、ABI1.74、state44、VBR保持。

本地证据：`artifacts/welcome-workflow-{software,d3d11}.txt`、`welcome-transform-software.txt`、`welcome-ui-software.txt`、`welcome-d3d11.png`；构建记录 `history-build.txt` / `interaction-build.txt`，不入版本库。

## 2026-10-21：移除通用文件成功通知

Main原来把每次文件成功回执写为“文件操作完成”，跨文档切换后仍保存在窗口fileNotice，并被欢迎区当错误显示。成功现在清空通知；失败通知和保存后继续关闭/并发修改保护仍保留。

新增 `closingCanvasKeepsWelcomeFreeOfSuccessNotices` 白底/透明两行，软件及D3D11各通过：真实新建后直接关闭无通知；另建画布实际保存ORA、检查成功回执/文件存在，再关闭，欢迎区仍无成功文字；真实打开不存在文件仍显示失败。日志 `artifacts/welcome-notice-{software,d3d11}.txt`。Release及Qt目标构建通过，ABI/state不变。D3D11透明行退出仍报告1个异步创建对象，属于U5未解决范围，没有屏蔽或称作零诊断；功能断言及QML警告收集通过。


## 2026-10-21：U5 关闭孵化与窗口透明状态

原有欢迎生命周期的软件与 D3D11 回归均通过，关闭后显式检查所有窗口的 panelContentLoader 均不激活，并检查孵化控制器归零。对原始残留计数、DelegateModel::cancel 与交换链 alpha 诊断启用 failOnWarning，未使用 ignoreMessage 或屏蔽日志。图层列表等外层异步创建完成后再填充，卸载正在 Loading 的内容前完成已开始的 deferred 控件；浮窗固定透明 surface，背景仍由矩形绘制。此前仅取消 Loader 的候选出现 DelegateModel::cancel，仍有两个遗留计数，已改正。

候选三软件：欢迎生命周期及拖动合并三行通过；D3D11 追加白底/透明保存关闭两行共六行通过，零上述诊断。证据 artifacts/lifecycle-candidate3-{software,d3d11}.txt。最终显式隐藏 Loader 断言、六行软件和 D3D11 均通过，日志 artifacts/batch-workflow-{software,d3d11}.txt；完整 Release 构建通过，输出 build/qt/bin/Release/drawverse.exe。ABI/state/VBR 不变；未跑全量或跨平台。


## 2026-10-21：U1 浮动画布真实绘画与 GPU 跨窗迁移

原0/1失败实际位于重做：测试使用Ctrl+Y，当前界面及快捷键表为Ctrl+Shift+Z，鼠标笔触和撤销本已完成。修正为当前快捷键并检查重做后红色像素；缩放等待实际1.15终值，平移先验证12/8再以静止中键停止惯性，继续验证跨窗保留状态，不加固定sleep。

继续跑D3D11发现真实产品错误：切换浮窗标签时park先修改parent、后隐藏，自适应轮廓ShaderEffectSource尚绑定旧窗口，出现Cannot use same item/不同窗口source提示，画布停在主窗口。CanvasPane现在先隐藏解绑，再改变parent并重新显示。完整原测试软件及D3D11均通过，含真实鼠标绘画/撤销/重做、缩放与惯性停止、多文档标签/保存重开/手写笔/跨窗停靠及关闭保护。日志 artifacts/canvas-window-{software,d3d11}.txt；原两个GPU窗口归属诊断消失。测试有两条故意将捕获鼠标坐标送到窗外的QtTest提示，未屏蔽，不属于QML/GPU警告。

首开80层面板软件与D3D11各通过，异步属性保持，最长心跳20/27ms、实显5行；完整Release构建通过，输出标准drawverse.exe。未跑全量/跨平台/实体输入。

## U5 补充：快速收起分类的直接销毁

附加共享面板回归发现，快速双击图层标签收起分类仍在退出留下两个创建计数，功能断言通过不代表生命周期通过。PanelContent 销毁时补齐 Loading 完成和 Loader 卸载，覆盖未先触发 requested 的父级直接销毁。原 panelTabsRetractWithoutExpandingRailsAndKeepSharedPresentation 增加残留/cancel/alpha 的 failOnWarning；候选软件与D3D11无上述诊断，D3D11共享笔刷设置窗口也通过。最终构建、原用例与欢迎/拖动共享回归记录于 remaining-fixes 验收末节。
