# 文件 IO / 工作区 v2 验收

2026-10-08，Windows 11 x64；Rust/Cargo 1.99.0 MSVC、MSVC 19.51、CMake 4.3.1/Ninja、Qt 6.8.3 MSVC2022。先写 io-design.md，再实现 paint-io、C ABI 1.2 文件任务及 QML 文件菜单。macOS/Linux、MSRV 和真实数位板硬件尚未在本轮验证。

## 已验证

`python tools/check.py` 完整通过 fmt、clippy（-D warnings）、80 项 Rust 单元/集成测试、doc tests、release workspace、cbindgen 头漂移检查，以及 CTest 四组 C11/C++20 static/shared ABI 和 Qt Test / 两倍 DPI。Qt 主测试包含 9 个功能场景，另有 init/cleanup；高 DPI 单独重复 QML 绘画与浮窗场景。

- 核心导入构建：名称/属性/边缘像素/坐标/预算校验，历史为空、活动层与后续 ID 正确。
- PNG/JPEG/WebP：实际写入再读取，线性预乘与文件直通 sRGB alpha 转换、JPEG 白色背景、PNG 16 位精度；JPEG EXIF 90 度方向正确改变尺寸。损坏/截断、ICC、非 sRGB PNG gamma 及动画 PNG 明确失败，标准 sRGB gamma PNG 可读取。
- OpenRaster：第一项未压缩 mimetype、stack.xml、图层 PNG、mergedimage.png、thumbnail；名称 UTF-8/特殊字符/换行、图层顺序/位置、可见性/不透明度/活动层回环。非法路径、DOCTYPE、组、未支持混合、越界非透明像素、NaN、巨大尺寸、不完整 XML、重复归档名称、伪造超限解压大小被拒绝。
- 原子保存：取消/参数失败/编码中途遇到不可表示的 XML 控制字符时保留旧文件并清理临时文件；成功替换旧目标。输入文件大小在解码前校验。
- 文件任务：一个 session 一个作业、排队取消/BUSY/ID 递增、持久错误、路径 NUL/代际校验；保存期间发生新修订不会清除 modified，过期加载不能覆盖新文档。四组原生 C/C++ 程序调用新接口保存/重开真实 ORA，并校验两个 64 字节 DTO。
- Qt：四格式后台文件流程、保存后重开、失败保留原文档、按内容识别伪装扩展名，避免误把导入图片的路径作为可编辑文档覆盖；取消和带文件任务的异步关闭；已接受但尚未发布的编辑也即时保护未保存状态，防止立即新建/关闭误丢绘画。既有压感/撤销/图层/百万画布/导航/过载回滚/QML 拖放/浮窗/高 DPI 全部通过。
- 笔记：删除创建入口、编辑方法和 QML 内容组件；v1/v2 旧配置校验后只清理 notes 定义与实例，空组删除，活动标签修正；其他色板、面板组、浮窗几何仍可重开。本机应用 QSettings 已实际迁移到 v2，并确认无 notes 项。

历史构建中有尚未重编译的零依赖旧 C++ 对象；本轮清理并重建 `build/qt` / `build/verify-ui`，正确的 MSVC 本地化 include 前缀继续生成头依赖。Qt DLL、文件对话框 QML 模块、windows/offscreen 平台插件均部署在启动目录。

## 构建、运行和测试

在已激活 MSVC/Rust/CMake/Ninja 的终端，从项目根目录运行：

```powershell
python tools/check.py
cmake -S . -B build/qt -DCMAKE_BUILD_TYPE=Release -DDRAWVERSE_BUILD_UI=ON "-DCMAKE_PREFIX_PATH=.tools/qt/6.8.3/msvc2022_64"
cmake --build build/qt --config Release
ctest --test-dir build/qt -C Release -R "^(abi_|qt_ui_)" --output-on-failure
./build/qt/bin/Release/drawverse.exe
cargo test --manifest-path core/Cargo.toml -p paint-io --locked
```

默认完整测试日志：`artifacts/io-check.log`，Qt 详细报告：`build/verify-ui/qt-ui-tests.txt` / `qt-ui-highdpi.txt`。本机通过同一输入/渲染路径的原生预览另存 `artifacts/io-workspace.png`。

## 当前范围

PNG 导入支持 8/16 位；PNG/JPEG/WebP 输出为 8 位合成图，WebP 第一版为无损编码。ORA 支持当前平面 normal 图层，保存为 8 位 sRGB，不是 RGBA32F 无损原生格式，不保存历史。ICC、自定义非 sRGB PNG 元数据、动画、组、其他混合、DPI 元数据、PSD/自有格式待后续模块。

输入 ≤256 MiB；合成图及 ORA 图层栅格累计分别 ≤16,777,216 像素、单幅边长 ≤16,384；ORA ≤256 层/1024 ZIP 项、XML ≤1 MiB、ZIP32 单磁盘、累计声明解压大小 ≤512 MiB；模型 ≤4096 瓦片。超限明确报错，无隐式降采样。取消为协作检查，单次 codec 调用不能强杀；原子替换不等于跨平台断电耐久性保证。性能目标仍需后续测量，Python 插件仍最后实施。
