# 几何选区验收（2026-10-09）

本模块为 ABI 1.8.0。矩形/椭圆、添加/减去/相交、全选/取消/反选已接入 Rust 历史与 Qt 输入；None 与启用但空覆盖不同。选区按文档坐标约束笔刷、擦除及像素蒙版，图层移动后仍正确。选区本身不裁剪显示/导出。OpenRaster 扩展 v4 保存有界步骤并读取 v1–v3；PNG/JPEG/WebP 导出仍为完整画布，ORA 仍使用既有8位像素编码。

M 为矩形，Shift+M 切换椭圆；拖动时 Shift 添加、Alt/Option 减去、Shift+Alt/Option 相交，松手提交一次。Esc 取消当前拖动；Ctrl/Command+A 全选、D 取消、Shift+I 反选。选择菜单及浮动工具条均可用。静态黑白虚线随缩放/平移更新，未使用持续全画布动画；无选区时 Loader 不创建 QQuickPaintedItem，笔刷通过 const 专用路径消除逐像素几何判断与额外乘法。

## 验证

完整 python tools/check.py 已通过（artifacts/selection-final-check.log）：171项 Rust、生成头漂移、四组 C11/C++20 静态/动态 ABI、30项 Qt 行为和12项两倍 DPI（含初始化/清理为32/14），六个 CTest 目标无失败。

新回归覆盖几何抗锯齿/布尔组合、百万画布不分配像素瓦片、64步骤限额与失败保护、无变化历史、磁盘历史回环/截断、绘画/擦除/移动后坐标/蒙版、FFI非法参数/过期publication/FIFO/活动笔触拒绝不取消、ORA保存与坏选择拒绝、导出不裁剪、实际鼠标和数位笔选区、组合键、Esc、缩放、工具浮窗、保存回读和轮廓未启用时对象不存在。

Windows 原生截图 artifacts/selection-native.png 已目视检查透明棋盘、受限笔触和椭圆轮廓；最终按需创建版本原生测试也通过（artifacts/selection-final-native.txt），无 QML 警告。截图来自 Qt 实际程序。

## 性能调查

完整测试与编译之外按顺序运行600点绘画，DPR1.5、输出1054×703。初次累计进程 CPU453.125ms，重复500ms；画面p95为17–18ms。独立检出并构建上一个提交96eb256，在同环境重测 CPU265.625ms，确认需要优化。Qt线程观察显示增加主要位于界面/场景图线程，Rust绘画/合成没有相应增加。

改为轮廓按需创建并保留笔刷专用快路径后：artifacts/selection-lazy-performance.json 中 CPU265.625ms、墙钟6355.5ms、301次更新、p50 16ms/p95 17ms、GUI tick p95 9ms、追赶16ms，无错误。同环境旧版本为 CPU265.625ms、p95 17ms。进程 CPU包含启动/关闭和所有线程；结果是本机固定无选区工作负载，不承诺逐笔<8ms或所有选区复杂度相同。已移除性能对照临时工作树，基准报告保留在主工作区 artifacts。

## 命令与边界

构建：cmake --build build/qt --config Release。运行：build/qt/bin/Release/drawverse.exe。完整测试：python tools/check.py。性能：python tools/measure_painting.py build/qt/bin/Release/drawverse.exe artifacts/selection-performance.json。

选择步骤最多64；替换清空旧步骤，超限报错并保留旧状态。自由套索、羽化、魔棒、选择变换后续实施；边界为静态虚线。Windows本机及合成数位笔输入已验收，macOS/Linux和真实硬件仍待跨平台模块。Python插件最后，不创建虚假pytest。
