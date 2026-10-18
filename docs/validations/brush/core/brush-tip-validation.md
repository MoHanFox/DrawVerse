# 程序笔尖第一阶段验收

2026-10-13：已实现 Rust `BrushDefinition` / `BrushProgram`，程序笔尖硬度、圆度、角度、流量、大小/流量压感。画笔/擦除/蒙版/涂抹共用覆盖；旧入口保留默认硬圆像素。新增 ABI 1.25，生成头同步。Qt 预设状态 v2 读取 v1，失败恢复保持原状态；PS 分类面板、真实后台预览、旋转椭圆双线光标。

相关验证：

- `cargo test -p paint-core -p paint-ffi --locked`：核心全部通过；FFI 旧组测试的硬编码版本改为当前常量后，`cargo test -p paint-ffi --locked` 全部通过（24 单元、31 异步、1 回调测试）。
- `cargo clippy -p paint-core -p paint-ffi --all-targets --locked -- -D warnings` 通过。
- Release Qt 构建通过，生成头字节检查通过；四项相关 Qt 测试通过：新笔尖像素/光标/保存/真实预览、旧预设/迟到预览、画笔橡皮共用与独立大小、跨窗口设置/后台预览。无 QML 警告，原生 Windows 截图检查通过。
- C/C++ 静态/动态四项 ABI smoke 通过。

日志在忽略的 `artifacts/brush-tip-*`。这是相关模块验收，不表示本轮运行了完整 `tools/check.py`。VBR/ABR、图片笔尖、散布、纹理、稳定器、整笔不透明度、湿混及额外全局光标设置尚未完成。
