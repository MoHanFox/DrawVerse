# Qt / 原生修订桥验收

2026-10-13：ABI 1.28、旧参数状态 v1/v2 原子迁移、独立元数据编译池、准确不可变准备记录、Rust Begin等待、Qt共享库连接已接通。Qt参数是草稿，Rust拥有验证/编译后的定义；真实画布通过 prepared-preset入口执行。后台预览使用隔离原生库；后台保存采用Rust规范化有效参数，错误不覆盖文件。UI预设上限与内核统一1024，状态上限2MiB。

相关验证：

- paint-ffi/paint-io全测试通过。新增状态迁移/可执行像素/非法和未来字段拒绝/图片不被丢弃；准备任务 panic 变成明确错误、记录裁剪后旧Arc继续有效；异步起笔引用准确修订，在新提交和库销毁后仍保持椭圆像素。旧接口/目录/文件测试继续通过。
- paint-core/paint-ffi/paint-io全目标 Clippy -D warnings通过。
- Release Qt构建、生成头字节检查通过。8项相关原生 Qt测试通过（含初始化/清理10 pass），新增即时设置后起笔、笔内编辑不改当前笔、下一笔新方向、坏UUID迁移保留原INI并显示错误/恢复合法库；VBR/共享文档/关闭写入者/光标/旧预设/实际预览测试继续通过，无QML警告。
- C11/C++20静态/动态4项 ABI smoke通过，新增canonical-state/prepare/prepared-state真实调用。

日志 `artifacts/brush-bridge-*`。未运行完整 tools/check.py，不能据此声明全仓库全项验收。该桥当前迁移程序预设；图片持久资源、VBR图片、ABR、动态/平滑/湿混仍是完整目标的未完成部分。
