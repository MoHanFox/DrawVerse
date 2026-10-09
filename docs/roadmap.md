# 模块顺序与验收

每轮先补设计，再实现、运行测试并记录结果；已交付第 1–5 阶段，QML 已接 Rust 任务与 CPU 视口渲染；插件层最后完成。

| 顺序 | 模块 | 状态 | 验收门槛 |
|---|---|---|---|
| 1 | 架构 / C ABI 草案 / UI 工作区设计 | 已设计 | 分层、所有权、线程、错误与版本有明确契约 |
| 2 | paint-core 最小内核 | 已实现并通过本机验证 | 文档、图层、压感圆笔刷、稀疏瓦片、撤销/重做/预算回归 |
| 3 | paint-ffi + cbindgen | 已实现并通过本机验证 | ABI 1.6.0 向后兼容、staticlib/cdylib、生成头漂移检查、36 项 FFI 回归、C/C++ 四组含异步链接运行 |
| 4 | paint-task + paint-render CPU | 已实现并通过 Windows 本机验证 | 有界优先级、串行写/快照、取消/panic、CPU 合成/稀疏 LOD、异步帧与背压、大画布/视口/导航/撤销回归 |
| 5 | Qt Quick / QML 绘画 UI / 工作区 | 已实现并通过 Windows 本机验证 | 后台 PaintCoreClient、画布、图层/颜色/笔刷/历史/导航、标签组拖放/浮动、自定义面板、布局保存、Qt Test |
| 6 | 平台数位板接入 | Qt 通道已接入，硬件待验证 | 接收 QTabletEvent、压力/倾斜/旋转/切向/橡皮擦/按钮、坐标与两倍 DPI 测试；Wintab 选择及真实硬件矩阵仍待实施 |
| 7 | paint-io 基础格式 | 已实现并通过 Windows 本机验证 | PNG/JPEG/WebP/ORA 回环、16 位导入、EXIF、截断/路径/XML/归档预算、原子保存、ABI 1.2 后台读写与 Qt 文件流程 |
| 7a | paint-storage 配置与清扫 | 已实现并通过 Windows 本机验证 | ABI 1.3、QML 性能设置、重启生效、冷页/历史共享所选目录、空闲检查、真实进程租约与异常清扫回归 |
| 7b | PS 风格图层控制 | 已实现并通过 Windows 本机验证 | 27 种混合、透明/位置/完全锁、opacity/fill、非破坏性移动、可见行缩略图、历史、ORA 扩展、ABI 1.4、Qt 浮窗/DPI 与性能 |
| 7c | 图层组 / 透明画布背景 | 已实现并通过 Windows 本机验证 | 隔离嵌套、分组/解组/重新归组、锁继承、移动/子树删除、结构/磁盘历史、ORA、ABI 1.5、QML 浮窗/折叠、高 DPI 棋盘格 |
| 7d | 蒙版 / PS 风格拖放 / 默认背景 | 已实现并通过 Windows 本机验证 | 图层/组黑白灰蒙版、压感、显隐/撤销/删除、ORA 可编辑回读、隐式白色背景、中文默认命名、停靠/浮窗拖放、ABI 1.6 |
| 8a | 几何选区 | 已实现并通过 Windows 本机验证 | ABI 1.8、矩形/椭圆、组合/反选、绘画约束、历史与 ORA v4、Qt 浮窗/DPI |
| 8 | 深化文档模型 | 部分完成，继续推进 | 几何选区见8a、一般变换；蒙版见 7d；组/三锁/混合/整数移动及相应历史和保存见 7b/7c |
| 9 | paint-color / GPU / PSD | 待实施 | 色彩金图、16/32F、ICC、多屏、GPU/CPU 一致性与失败回退、PSD 支持表 |
| 10 | 跨平台 CI / 测试 / 打包 | 待实施 | Windows MSVC、macOS、Linux，Qt Test，签名/打包方案，CPU-only 可启动 |
| 11 | paint-plugin / Python SDK / 沙箱 | 最后 | pytest、示例滤镜、权限拒绝、超时、崩溃隔离、平台沙箱、声明式自定义面板 |

后续新增 crate 在真正实现时才加入 workspace，不提前加入空壳。当前有 Rust、C/C++ ABI、Qt Test，Python 插件还未实施。统一 check 默认包括 UI，需要 Qt SDK；明确只测核心时用 --core-only。最新文件/工作区及 UI 验证见 io-validation.md；任务/渲染阶段记录见 task-render-validation.md。CI 可逐步增加，最终打包在第 10 阶段验收；插件完成后再扩展 CI。

绘画性能回退修复已验收：CPU 瓦片缓存/稀疏输出、逐源像素编码、QML 图层通知拆分，保留 ABI 与原画质；本机固定压感输入基准及回归见 painting-performance-validation.md。

大笔触历史限制已修复，采用内存快照 / 无损编码 / 临时磁盘文件，保留整笔撤销与资源预算，见 history-storage-design.md / history-storage-validation.md。live 文档瓦片分页已实现（见 document-paging-design.md / document-paging-validation.md）。paint-storage 的暂存盘设置、空闲空间检查和租约清扫已完成（见 storage-settings-design.md / storage-settings-validation.md），下一模块推进选区等文档模型；插件仍最后实施。

PS 风格图层控制按用户澄清完成（透明像素锁；“样式”仅为混合下拉），见 layers-design.md / layers-validation.md。图层组与透明背景见 groups-design.md / groups-validation.md；蒙版与拖放及默认背景见 masks-design.md / masks-validation.md；下一模块推进选区；fx、特殊 Fill 曲线另行设计，插件最后。


### 7e. 并排蒙版、剪贴蒙版与缩略图边缘

用户要求将像素蒙版放在内容预览右侧，添加 Alt/Option 层间点击剪贴及释放，连续上层共用下方同级基底。核心合成、结构/磁盘历史、缓存、异步 ABI 1.7 和 ORA v3 已实现；旧 ORA v1/v2 继续读取。缩略图边框/留白不再显示棋盘格，部分边缘 LOD 按有效文档像素归一化。设计与验收见 docs/clipping-design.md / docs/clipping-validation.md。

### 7f. 参考图 QML 界面与停靠

紧凑无边框主窗、HSB 色轮、Logo/矢量图标、参考图与双列预设、标签排序/合并/上下拆分、折叠图标临时面板、关闭重开、稳定原生浮窗、工具条拖动浮动归位和布局 v4 已验证。见 reference-ui-design.md / reference-ui-validation.md。下一步按用户补充完成画笔库与当前画笔设置的共享状态与真实预览，然后继续选区、变换、色彩/GPU、PSD、自有格式和 CI；插件最后。每个模块完整验证后独立 commit 并推送 origin/master。

### 7g. 画笔库与当前笔刷设置

共享活动预设、按画笔独立参数、后台保存、自定义预设与真实异步笔触预览已完成。设置页无第二个笔尖网格；大小/间距按直径显示并正确转换核心单位。详见 brush-library-design.md / brush-library-validation.md。下一模块推进矩形/椭圆选区、选区组合与反选、限制绘画及历史；之后一般变换和核心高级笔刷。

### 7h. 画布与工具条的自由停靠

画布、工具条和工具标签组共用主窗及原生浮窗的二维分割树，四边停靠、标签合并/排序和空分支收缩；移走最后一个面板不留固定列。画布为唯一可移换父项的实例，保留异步绘画、缩放/平移、选区与压感输入；工具条可在任意面板两侧放置。布局 v5 保存比例和窗口几何，v1–v4 迁移保留既有内容。设计与验收见 flexible-docking-design.md / flexible-docking-validation.md。仅 UI 层修改，Python 插件仍最后。


### 7i. 多文档工作区与面板列

多个独立文档通过已有 PaintCoreClient / C ABI session 实施，主区与浮窗标签保留各自绘画/历史/文件任务和视口。画布/工具互拒，上边缘归位，#17191C 留空文档区；工具列顶部唯一导航栏、整体图标折叠、1px 分隔线与实际浮窗拖动。布局 v6 迁移旧工具布局；关闭/退出处理全部未保存文档。设计与验收见 document-workspace-design.md / document-workspace-validation.md；Rust/ABI 未改，Python 最后。

### 8a. 几何选区

矩形/椭圆、组合/反选、文档坐标绘画限制、统一历史、ABI 1.8、ORA v4、Qt 鼠标/数位笔/浮动工具条/高 DPI 已验证。无选区不创建轮廓绘制项，笔刷走专用快路径；600点同环境基准保持原有CPU与交互更新水平。见 selection-design.md / selection-validation.md。后续推进一般变换、核心高级笔刷、色彩/GPU、PSD、自有格式及跨平台构建；插件最后。
