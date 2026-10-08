# 蒙版、拖放和默认背景验收（2026-10-08）

本轮完成图层/组的可编辑像素蒙版、拖入/移出组与行间排序、组名称行和真实缩进，以及默认不透明白色“背景”和中文图层编号。Rust 不依赖 Qt；Qt 只经 PaintCoreClient / C ABI 1.6 提交异步操作；插件仍最后完成。

## 操作

- 选中图层或组，点“蒙版”创建白色蒙版；选择其下方的缩进行，黑色隐藏、白色恢复、灰色部分显示，压感调整半径和流量。色板 #808080 对应约 50% 覆盖率。橡皮擦恢复显示；眼睛按钮禁用蒙版；删除仅移除蒙版，原内容保留。
- 顶部不透明度/混合/填充/锁继续控制所属图层，蒙版密度独立控制。所有变化可撤销/重做。位置/完全锁继承，组移动和删除包含蒙版。像素图层缩略图保留原始内容，蒙版缩略图显示黑白灰。
- 拖到组中间归组；拖到行上下沿排序；拖到列表底部空白区域移出到根列表底部。插入线和组轮廓显示落点，悬停折叠组自动展开，靠近边缘自动滚动。整组拖动包含子树，停靠与独立窗口共用操作。
- 组行只显示名称、眼睛和展开箭头；子层缩进 16 逻辑像素/级，蒙版再深一级。新建默认白色“背景”，后续默认“图层一、图层二……”；组和蒙版不占编号，当前文档删除后不回退编号。新建对话框可显式选择透明背景。全部隐藏及透明区域仍显示白灰棋盘格。

## 数据与文件

蒙版缺页为完全显示，alpha 存隐藏量，复用稀疏瓦片、分页和逐位无损历史。背景为有界隐式白色矩形，只对改动分配瓦片；1000000×1000000 初始背景零文档瓦片。全透明擦除瓦片仍是有效白底覆盖记录，重开和撤销不会意外恢复白色。透明像素锁可绘制隐式白底，保持擦除孔洞。

ORA 扩展 v2 保留可编辑节点、蒙版、背景及位置：drawverse/stack.xml 与原始 data/*.png 保存树，标准 stack.xml 引用 mergedimage.png 供其他阅读器查看合成结果。清单缺失、重复 ID、循环和尺寸冲突明确失败。普通无蒙版/隐式背景的 ORA 继续输出层栈。PNG/JPEG/WebP 导出合成，JPEG 按白色衬底去除透明。ORA 仍为 8 位 sRGB 图层，并非 32F 无损工程格式。

## 验证结果

`python tools/check.py` 完整通过：157 项 Rust 单元/集成测试，fmt、clippy -D warnings、文档测试、Release 构建和生成头检查；C11/C++20 × static/shared 四组链接运行；21 项 Qt 功能测试（加初始化/清理共 23 PASS）；两倍 DPI 的 6 项界面功能测试（共 8 PASS）。随后最新原生构建和全部六组 CTest 通过；没有跳过失败项。Python 插件未建立，未生成虚假 pytest。

新增回归覆盖黑白灰/压感/橡皮擦/密度与禁用、组遮罩、父锁、背景擦除/透明锁、冷页/磁盘删除历史、蒙版缓存及多 LOD、超大背景、ORA 可编辑回读和损坏清单、扁平导出、ABI 新命令与 DTO、浮窗实际 Qt 拖放事件及过时代际拒绝。跨语言 DTO 原大小不变；PaintLayerDrop=24 字节，生成头由 cbindgen 更新。

Windows 原生 Qt Quick/DPR 1.5 截图已经检查：[画布](../artifacts/masks-canvas.png)、[浮动图层面板](../artifacts/masks-canvas.png.layers.png)。新功能正常显示，主程序仍用 Rust CPU 渲染、Qt Quick 显示，不能称为 Rust GPU 加速。

## 性能与边界

最新原生 600 个输入样本：输出 1529×1019，帧更新间隔 p50/p95=16/17ms，GUI tick p95=9ms，追赶 24ms；全进程 CPU 500.000ms / 总墙钟 6101.3ms（包含启动和关闭）。第一次测量 578.125ms。此前透明背景场景为 359.375ms；本轮默认白色背景不同，不能宣称 CPU 已降低或逐场景零回退。交互更新 p95 保持 17ms。

1800×1200、120 帧核心基准包含输入、快照及完整 CPU 出帧：普通透明场景 p95=7.552ms；白底 + 两层隔离组 + 正在绘制蒙版 p95=9.771ms。复杂场景仍需优化以增加 120Hz 余量，不能宣称所有场景达到 8ms。缓存继续使用 Weak 身份，不把冷页全部驻留。

视口超过一百万源瓦片的隐式背景使用有界中心采样预览，极端缩小可能漏掉细节；常规画布保持精确 LOD 平均。每个节点最多一个像素蒙版，最多 16 级深度；暂未实现剪贴/矢量蒙版、蒙版永久应用、多选组合分组、穿透组。带蒙版的组直接解组会明确拒绝，需先删除蒙版；不会静默丢失效果。HDR/ICC/PSD/一般变换与插件没有在本轮伪称完成。

## 构建、启动与测试

在配置好 Rust、MSVC / Windows SDK、CMake、Ninja 的终端运行（其他平台使用对应原生编译器和 Qt SDK）：

```powershell
cmake -S . -B build/qt -G Ninja -DCMAKE_BUILD_TYPE=Release -DDRAWVERSE_BUILD_UI=ON -DCMAKE_PREFIX_PATH="<Qt SDK>"
cmake --build build/qt --config Release
& .\build\qt\bin\Release\drawverse.exe
python tools/check.py
ctest --test-dir build/qt -C Release -R "^(abi_|qt_ui_)" --output-on-failure
python tools/measure_painting.py build/qt/bin/Release/drawverse.exe artifacts/masks-ui-performance.json
cargo run --manifest-path core/Cargo.toml -p paint-render --example painting_bench --release --locked -- --white --mask --groups
```

验收初期，旧窗口运行导致标准路径的程序无法覆盖，因此先在 build/qt-masks 独立构建，没有中断旧进程。旧窗口随后自然退出，标准路径也已成功更新。现在直接启动 **build/qt/bin/Release/drawverse.exe** 即为新版；build/qt-masks 也保留相同源码的验收构建。Qt DLL/QML/platform 插件已随构建部署，最终原生截图和性能测量均使用标准路径程序。

原始记录：[完整检查](../artifacts/masks-full-check.log)、[最新原生构建](../artifacts/masks-native-build.log)、[标准路径构建](../artifacts/masks-standard-build.log)、[原生测试](../artifacts/masks-native-tests.log)、[原生截图测试](../artifacts/masks-native-preview.txt)、[UI 性能](../artifacts/masks-ui-performance.json)、[普通核心性能](../artifacts/masks-flat-performance.log)、[蒙版核心性能](../artifacts/masks-mask-performance.log)。本机 Windows 实测；macOS/Linux CI 尚待原定后续模块验收。
