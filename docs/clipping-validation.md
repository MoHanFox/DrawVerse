# 并排蒙版、剪贴与预览修复验收（2026-10-09）

本轮完成用户指定的并排像素蒙版、Alt/Option 层间剪贴，以及填满画布却露出棋盘格的缩略图问题。设计见 [clipping-design.md](clipping-design.md)。默认新建仍为不透明白色“背景”，后续新层按“图层一、图层二……”命名；插件最后完成。

## 使用

- 图层内容预览右侧显示链条和像素蒙版缩略图，不再为蒙版另占一行。点击不同缩略图切换内容/蒙版绘画目标，白色框标记当前目标。Shift 点击蒙版启用或停用；菜单可删除蒙版，原始内容不烘焙、不丢失。组保留名称行及其蒙版，子图层继续按深度缩进。
- 按住 Alt（macOS Option）点击两层之间，创建或释放上层的剪贴蒙版。分界线两侧各有有效点击区域。箭头缩进标出被剪贴层，基底名称带下划线。支持多张连续上层共用基底；可通过菜单或 Ctrl+Alt+G（macOS Command+Option+G）切换。
- 剪贴依据同级下层的不透明区域；基底自己的像素蒙版也参与范围计算。隐藏或完全透明基底关闭剪贴链。半透明边缘保持原基底 alpha，不因连续叠层变得更不透明。基底不透明度/填充在链完成后应用一次，上层保留其自身不透明度、填充和混合模式。
- 拖动后按当前同级相邻层重新解析基底，不跨组引用；移到没有下层基底的位置仍保留可释放的剪贴标志，此时不可见。直接创建缺少基底的剪贴会明确报错，不改变历史。
- 停靠和独立面板使用相同操作。蒙版工具改为紧凑单行，图层面板保留至少 360 逻辑像素高度；选中行自动滚入可视区，避免工具栏把列表挤到无法操作。

## 预览修复

确认了两处原因：棋盘格原先填满整个缩略图容器，包括 2px 留边和保持宽高比产生的留白；LOD 方块在右/下画布边缘把画布外零 alpha 计入平均。现在棋盘格仅在实际拟合图像矩形内显示，边框/留白使用面板底色；边缘平均使用有效文档像素面积。常规完整瓦片保持原有快速路径，画布外视口仍透明，图像内部真实透明区域仍显示棋盘格。

使用 641×479 奇数尺寸白色背景验证 64×48、5×4、1×1 等 LOD：每个输出像素均不透明，Qt 异步缩略图的全部像素也为纯白；随后擦除边缘，确认真实透明度仍保留。独立密集参考采样器同步修正边缘面积定义，缓存、冷页、裁剪视口、文档替换等原有测试全部保留并通过，没有去掉失败项或放宽原有逐位比较。

原生 Qt Quick/DPR 1.5 截图已目视检查：[完整白色画布及其缩略图](../artifacts/clipping-native.png.opaque.png)、[并排蒙版与剪贴图层](../artifacts/clipping-native.png.layers.png)、[剪贴画布](../artifacts/clipping-native.png)。剪贴演示使用旧 ABI 的显式透明文档测试入口，因此基底名为 Layer 1；正常启动和默认新建为中文“背景”。

## 接口与保存

Rust 内核不依赖 Qt。ABI 1.7 添加能力位 2048、PaintLayerClipping（16 字节，基底 ID 偏移 8）及独立查询/编辑函数；所有旧 DTO 布局保留，头文件由 cbindgen 生成。查询绑定 publication，编辑提交 FIFO 后台任务并执行锁、层级和原子历史检查。结构和删除磁盘历史保存剪贴标志，内部临时格式为 DVH5。

ORA 扩展 v3 保存 dv:clipped，继续读取旧 v1/v2，保留可编辑像素、蒙版与剪贴链；标准栈提供合成预览，其他阅读器可看到最终效果。非法标志或把蒙版节点设为剪贴会拒绝。PNG/JPEG/WebP 导出合成效果，JPEG 使用白色衬底。ORA 仍为 8 位 sRGB，并非浮点无损工程格式。

## 验证

`python tools/check.py` 完整通过：165 项 Rust 测试、fmt、clippy -D warnings、文档测试、Release、生成头一致性；C11/C++20 × static/shared 四组 ABI 链接运行；22 项 Qt 功能测试（初始化/清理计入共 24 PASS），两倍 DPI 的 7 项功能（共 9 PASS）。随后在常用 build/qt 路径对新增边界两侧 Alt 点击运行全部六组 CTest，通过；最后原生 Windows 场景再次通过，没有跳过失败。Python SDK 未建立，不创建虚假 pytest。

新增覆盖半透明/多层剪贴、混合/不透明度、蒙版/组/位置/隐藏/完全锁、孤立链、撤销与磁盘删除历史，缓存与多 LOD、奇数尺寸预览边缘、ORA v2 兼容/v3 回读/非法元数据/三格式导出、ABI 复制/错误码/一致发布，以及真实 Qt 鼠标 Alt/Shift 操作、浮窗与缩略图目标切换。

## 性能与限制

相同默认白底原生 600 点输入，输出 1529×1019、DPR 1.5，两次独立样本都保留：

| 指标 | 第一轮 | 第二轮 |
| --- | ---: | ---: |
| 帧间隔 p50 / p95 (ms) | 16 / 19 | 16 / 17 |
| GUI tick p95 (ms) | 10 | 9 |
| 追赶 (ms) | 32 | 40 |
| 全进程 CPU (ms) | 546.875 | 218.750 |
| 全进程墙钟 (ms) | 6207.6 | 6260.8 |

CPU 包括启动和退出，样本存在波动，不能把第二轮较低数字当作稳定降幅保证。核心 120 帧、1800×1200 基准包含输入/快照/完整 CPU 出帧：普通透明场景 p95=5.459ms；白底、两层隔离组、正在绘制剪贴层 p95=8.234ms。复杂场景仍略高于 8ms，120Hz 尚未普遍达标。Rust 仍使用 CPU 渲染，Qt Quick 负责显示。

剪贴依据当前层序，重新排序或移组会改变基底；没有 Photoshop 全部高级混合开关。矢量蒙版、蒙版永久应用、多选分组、穿透组、ICC/HDR/PSD/Rust GPU 和插件仍为后续模块。极端缩小的有界中心采样预览沿用既有限制，不宣称所有缩放精度相同。Windows 已实测，macOS/Linux 构建和硬件 CI 仍待后续验收。

## 构建、启动、测试

先激活 Rust、MSVC/Windows SDK、CMake/Ninja 和 Qt SDK（其他平台使用对应原生编译器）：

```powershell
cmake -S . -B build/qt -G Ninja -DCMAKE_BUILD_TYPE=Release -DDRAWVERSE_BUILD_UI=ON -DCMAKE_PREFIX_PATH="<Qt SDK>"
cmake --build build/qt --config Release
& .\build\qt\bin\Release\drawverse.exe
python tools/check.py
ctest --test-dir build/qt -C Release -R "^(abi_|qt_ui_)" --output-on-failure
python tools/measure_painting.py build/qt/bin/Release/drawverse.exe artifacts/clipping-ui-performance.json
cargo run --manifest-path core/Cargo.toml -p paint-render --example painting_bench --release --locked -- --white --groups --clip
```

常用 **build/qt/bin/Release/drawverse.exe** 已更新和部署，DLL、QML 及平台插件随构建部署。

记录：[完整检查](../artifacts/clipping-full-check.log)、[标准路径构建](../artifacts/clipping-native-build.log)、[原生全部测试](../artifacts/clipping-native-tests.log)、[原生截图测试](../artifacts/clipping-native-preview.txt)、[UI 第一轮](../artifacts/clipping-ui-performance-first.json)、[UI 第二轮](../artifacts/clipping-ui-performance.json)、[普通核心基准](../artifacts/clipping-flat-performance.log)、[剪贴核心基准](../artifacts/clipping-chain-performance.log)。
