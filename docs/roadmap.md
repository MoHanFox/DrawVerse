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
| 8 | 深化文档模型 | 部分完成，继续推进 | 选区、一般变换；蒙版见 7d；组/三锁/混合/整数移动及相应历史和保存见 7b/7c |
| 9 | paint-color / GPU / PSD | 待实施 | 色彩金图、16/32F、ICC、多屏、GPU/CPU 一致性与失败回退、PSD 支持表 |
| 10 | 跨平台 CI / 测试 / 打包 | 待实施 | Windows MSVC、macOS、Linux，Qt Test，签名/打包方案，CPU-only 可启动 |
| 11 | paint-plugin / Python SDK / 沙箱 | 最后 | pytest、示例滤镜、权限拒绝、超时、崩溃隔离、平台沙箱、声明式自定义面板 |

后续新增 crate 在真正实现时才加入 workspace，不提前加入空壳。当前有 Rust、C/C++ ABI、Qt Test，Python 插件还未实施。统一 check 默认包括 UI，需要 Qt SDK；明确只测核心时用 --core-only。最新文件/工作区及 UI 验证见 io-validation.md；任务/渲染阶段记录见 task-render-validation.md。CI 可逐步增加，最终打包在第 10 阶段验收；插件完成后再扩展 CI。

绘画性能回退修复已验收：CPU 瓦片缓存/稀疏输出、逐源像素编码、QML 图层通知拆分，保留 ABI 与原画质；本机固定压感输入基准及回归见 painting-performance-validation.md。

大笔触历史限制已修复，采用内存快照 / 无损编码 / 临时磁盘文件，保留整笔撤销与资源预算，见 history-storage-design.md / history-storage-validation.md。live 文档瓦片分页已实现（见 document-paging-design.md / document-paging-validation.md）。paint-storage 的暂存盘设置、空闲空间检查和租约清扫已完成（见 storage-settings-design.md / storage-settings-validation.md），下一模块推进选区等文档模型；插件仍最后实施。

PS 风格图层控制按用户澄清完成（透明像素锁；“样式”仅为混合下拉），见 layers-design.md / layers-validation.md。图层组与透明背景见 groups-design.md / groups-validation.md；蒙版与拖放及默认背景见 masks-design.md / masks-validation.md；下一模块推进选区；fx、特殊 Fill 曲线另行设计，插件最后。


### 7e. 并排蒙版、剪贴蒙版与缩略图边缘

用户要求将像素蒙版放在内容预览右侧，添加 Alt/Option 层间点击剪贴及释放，连续上层共用下方同级基底。核心合成、结构/磁盘历史、缓存、异步 ABI 1.7 和 ORA v3 已实现；旧 ORA v1/v2 继续读取。缩略图边框/留白不再显示棋盘格，部分边缘 LOD 按有效文档像素归一化。设计与验收见 docs/clipping-design.md / docs/clipping-validation.md。
