# 画笔库 / 当前画笔设置验收（2026-10-09）

画笔面板按列表选择实际圆笔预设；画笔设置只编辑当前选择，已删除重复预设网格及橡皮擦卡片。停靠、浮窗、自定义画笔参数面板共用 BrushLibrary。大小为直径 1–512px，间距为直径 2.5–50%，不透明度 0–100%；核心仍接收半径、半径间距比例与透明度。修改影响下一笔，当前笔触保留开始时的参数快照。

默认四个圆笔预设，修改按预设保留；“+”可另存为自定义画笔，允许删除自定义预设。颜色、橡皮擦与移动工具保持独立状态。预览使用独立 Rust 异步 session、真实压感和透明度/间距，后台生成透明 240×60 图；半径为展示缩放，不承诺预览尺寸等于文档像素。主画布绘画时延后预览，按 token 拒绝过期结果，失败后不自动循环重试，双击设置页预览可重试。后台配置保存有失败提示。--preview / --benchmark 使用临时偏好配置，防止改变用户画笔参数。

## 通过项

- python tools/check.py：165 项 Rust、生成头检查、四组 C11/C++20 ABI、28 项 Qt 行为及 11 项两倍 DPI 测试完整通过（Qt 日志含初始化/清理为 30/13），六个 CTest 目标全部通过。
- 新增预设独立参数、重命名/复制/删除、持久化、坏 JSON/非有限数、过期预览拒绝及失败停止重试测试。
- 实际跨窗选择、设置页键入大小、名称同步、透明度/间距预览像素变化、半透明笔触真实绘画、退出重开恢复参数通过。预览期间主文档 revision、撤销深度及 modified 不变。
- Windows 原生截图已目视检查：artifacts/brush-final.library.png、brush-final.settings.png；由 Qt 渲染，未使用 PS 截图作为笔刷资源。
- 完整测试结束后单独运行 600 点原生绘画基准：DPR 1.5、输出 1054×703、303 次更新，更新间隔 p50 16ms / p95 17ms，GUI tick p95 9ms，进程累计 CPU 281.25ms，墙钟 6354.7ms，无错误。前模块同分辨率 CPU 234.375ms、p95 17ms；累计 CPU 包含启动、配置及退出且有运行波动，不作逐笔延迟承诺。

日志：artifacts/brush-library-check.log、brush-library-native.txt、brush-library-performance.json。构建/运行/测试命令沿用 README。

本轮 Rust/C ABI 无修改，使用现有 ABI 1.7 及圆笔引擎。纹理、散布、角度/圆度、硬度、混色笔刷和 ABR 尚未实现，因此没有显示对应假控件；后续需核心笔刷模块和格式支持。macOS/Linux 与真实硬件待跨平台验收。Python 插件未开始，不创建虚假 pytest。

## 橡皮擦与画笔共享预设修复（2026-10-10）

画笔库预设点击删除强制 `PaintClient.eraser=false`，选择预设只更新共享 BrushLibrary。橡皮擦/画笔切换仍由工具条决定，预设、大小、透明度与间距共用。

brushPresetClicksKeepEraserAndShareActualStrokeSettings：在透明文档实际画线，启用橡皮擦并通过 QML 点击另一个预设，确认工具仍为橡皮擦；修改共享参数后实际擦除使中心 alpha 降低，再点击画笔工具确认预设/参数保持，实际绘画使 alpha 提升。普通、两倍 DPI 与 Windows 原生后端通过。完整 tools/check.py 全部通过（普通含生命周期 49 项、两倍 DPI 27 项），本轮日志和原生验收见 [menu-blur-layer-validation.md](menu-blur-layer-validation.md)。Rust/C ABI 无修改，继续使用 ABI 1.8.0。
