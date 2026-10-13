# 原生笔刷库 ABI 验收

2026-10-13：ABI 1.27 已接独立 PaintBrushLibrary 句柄、准确 revision 元数据/文字、参数编辑、选择、改名、副本与删除，以及捕获库预设的真实异步笔触入口。目录接口与文档无生命周期绑定。

相关验证：

- paint-ffi 全测试通过：24单元、32异步（新增开始笔触后编辑/删除/销毁库仍保留真实椭圆像素及撤销重做）、2目录 ABI（DTO布局、文本缓冲、过期读取、失败原子性、内置保护、句柄与core独立）、1文件、1回调。
- paint-core/paint-ffi 全目标 Clippy -D warnings 通过。
- cbindgen生成头字节检查和Release Qt/C/C++构建通过。C11/C++20静态/动态4项 ABI smoke 全通过，新增目录 DTO大小/偏移、创建、读取、参数编辑、过期读取、内置保护、副本与销毁真实调用。

日志 `artifacts/brush-catalog-abi-*`。没有把内核ABI验证冒充 Qt 镜像已迁移：当前 UI 仍使用既有参数桥，独立库的 UI 生命周期/持久化与资源接口为后续接入。图片包/ABR/动态/湿混仍未完成。本轮未运行完整 tools/check.py。
