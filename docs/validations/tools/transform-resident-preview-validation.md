# 自由变换常驻纹理预览验收

## 2026-10-20：无关组和蒙版的共享常驻源

- 原判据整栈禁用组/蒙版，现按真实根层边界检查；下方完整复杂栈、上方 Normal/穿透组及蒙版保留为静态资产。目标自身蒙版、组内目标、多目标、选区和上方剪贴/特殊混合仍回退，未宣称全部复杂栈性能完成。
- Rust 新增 2 项：半透明嵌套组/图层蒙版/组蒙版三层合成与原图误差 <1e-6；变换八次 Warp 更新复用源与原瓦片、移动和液化同享快路径，最终整图逐行与 CPU 路径相等，历史/撤销/重做保持。
- 既有 deferred 4 项、layer_move 7 项通过；严格 paint-core Clippy 通过。
- `qt_transform_tests::transformResidentPreviewFollowsAndCompositesTransparentStack`：原平坦栈及新增上下组/蒙版两行，真实 D3D11 通过；覆盖 100 次更新、静态资产身份、半透明合成、最终确认和取消。
- 同两行与已修复快捷键回归在软件后端通过。Release 目标 drawverse/transform/selection/workflow 构建和生成头检查通过，输出 `build/qt/bin/Release`。
- ABI 1.71 不变。未做本项 4K 实体连续拖动性能测量、全量、跨平台或实体设备验证，不将结构复用测试当作 UI 延迟数据。

2026-10-19，Windows 11 / Qt 6.8.3 / RTX 4060 Laptop GPU，分支 `feat/wgpu-support`，ABI 1.64。设计与调用见[设计](../../tools/transform-resident-preview-design.md)，加速职责见[GPU总入口](../../platform/gpu-acceleration-design.md)。

后续强折叠一致性修复替换了本文初版的Warp Newton定位路径，见[折叠回归](transform-fold-consistency-validation.md)。本文基准保留为初版历史记录。

## 实现与参考

比较本地 PhotoCraft `d95fdf3181c01ad41e95456091a394b105a9afb2` 的 `crates/ui-egui/src/transform_tool.rs`、`transform_tex.rs`、`warp_preview.rs` 与最终取样路径。采用不可变原图、CPU前向有界细分、GPU常驻纹理显示、确认独立物化的架构；保留DrawVerse的4×4插值Catmull节点和既有最终Newton逆向取样，没有移植Bezier控制语义或最终三角栅格算法。

兼容无选区、单一Normal栅格或置入图片、平坦普通混合栈。拖动仅更改几何并发布至多145×145顶点，Qt使用常驻原图和上下层在有界视口合成；确认才执行原全精度取样器。控制点由CPU计算，Qt RHI负责显示，wgpu计算偏好不控制这一显示路径。旧ABI开始入口保持像素预览；新增入口/显式启用供支持常驻显示的客户端使用。

## 最小回归

- `cargo test -p paint-core --locked --lib deferred_transform`：3项通过。验证拖动不替换原始瓦片/源身份、控制点实际映射、局部撤销重做、完成与旧路径逐像素一致、永久历史；取消画布外变换、置入原始模型与自动会话接入、选区回退；超16Mi目标预算物化失败保留会话、原像素和历史。日志 `artifacts/transform-core-new.txt`。
- `cargo test -p paint-core --locked --test transform`：6项通过，覆盖组/多选、选区、取样、置入模型与保存/历史。日志 `artifacts/transform-core-regression.txt`。
- `cargo test -p paint-ffi --locked --test async_session transform`：2项通过。新增用例覆盖1000次绝对几何更新与最终最新位置、静态源身份、旧revision网格拒绝、容量不足不写入、局部checkpoint/完成单历史、真实像素与撤销、旧入口加显式启用。日志 `artifacts/transform-api-final.txt`。
- Actor最小4项、液化常驻包1项、移动常驻包1项通过。日志 `artifacts/transform-{actor,liquify,move}-regression.txt`。以上Rust正确性测试为debug配置，性能另用Release。
- `cargo clippy -p paint-core -p paint-ffi --lib --locked -- -D warnings --emit=metadata`、格式、cbindgen生成头检查通过。metadata参数沿用本机Rust链接诊断规避方式，不改工具链。日志 `artifacts/transform-{clippy,fmt,header-check}.txt`。
- C/C++静态/动态四ABI检查通过，日志 `artifacts/transform-abi-final.txt`。

Qt Release最小用例：

- 新 `qt_transform_tests::transformResidentPreviewFollowsAndCompositesTransparentStack` 在Windows真实D3D11和软件场景图各通过1项功能测试。96×96三层半透明图实际抓取窗口，检查100次绝对更新后红色内容实时移动、旧位置/外侧显示正确上下层、源资产身份保持、局部撤销、Warp改变画面、确认帧接管、永久撤销与取消。日志 `artifacts/transform-qt-focused-{hardware,software}.txt`。
- 新测试在注册的offscreen/software CTest配置通过，日志 `artifacts/transform-ctest.txt`。
- 真实D3D11既有3项通过：`placedImageFreeTransformRasterizeAndShortcuts`、`liquifyResidentMeshRendersAndCommitsWithoutCpuFrames`、`layerMoveFollowsBeforeReleaseAndCancelsWithSingleHistory`。日志 `artifacts/transform-qt-hardware-final.txt`。
- Windows软件既有3项通过：置入/变换/栅格化、移动、液化画布外起笔。日志 `artifacts/transform-qt-software-final.txt`。

测试取帧曾遇到两项夹具问题：`canvas.grabToImage()`只捕获源项目，遗漏Qt layer.effect的最终合成，改为抓取真实窗口；软件窗口图像的devicePixelRatio元数据与实际尺寸不一致，裁剪按图像宽/窗口逻辑宽计算比例。原始帧与实际窗口均用于核对，修正后两后端通过；没有为通过测试修改产品颜色或软件渲染行为。自动化包含取帧与轮询，不能用整项测试时间推断输入到显示延迟。

## 固定性能

Release示例 `cargo run -p paint-core --release --locked --example transform_performance -- <size> <pixel|mesh> <warp|affine>`。同机白色正方形原图，六次绝对更新，Warp额外移动节点5/10，三遍取中位数。包含CPU更新和发布几何生成，不含初次源/代理准备、Actor/Qt调度、GPU显示、最终物化或历史。

| 场景 | 原逐像素路径 | 新CPU几何路径 | 顶点数 |
| --- | ---: | ---: | ---: |
| 512×512 Warp，六输入 | 1189.722ms | 1.832ms | 7744 |
| 512×512 仿射平移，六输入 | 449.099ms | 0.352ms | 7744 |
| 2000×2000 Warp，六输入 | 未测 | 5.708ms | 21025 |
| 2000×2000 仿射平移，六输入 | 未测 | 1.212ms | 21025 |

日志 `artifacts/transform-pixel-before.txt`、`transform-mesh-after.txt`、`transform-affine-before.txt`、`transform-affine-after.txt`。后两份早期调用的第三参数写作 `projective`，示例将所有非warp值解析为普通四边形路径，实际几何是仿射平移，不能据此声称透视基准。数据仅说明固定路径移除了交互逐像素计算，不代表端到端帧率、任意复杂图片或实体连续拖动已通过。

## 构建与限制

完整Windows Release构建通过，产物 `build/verify-ui/bin/Release-wgpu-transform/drawverse.exe`；同目录包含最新 `paint_ffi.dll`。没有终止用户运行中的旧程序。构建日志 `artifacts/transform-ui-build.txt`。

- 有选区、组/多层目标、蒙版/剪贴/特殊混合及软件场景图继续走原逐像素预览，本轮未解决这些场景的交互成本。
- 原图代理最长边2048、视口纹理每轴上限2048，预览RGBA8/有界三角细分；极端放大、透视和自交折叠与最终精确逆向取样可能存在差异。完成使用完整源与原六种取样，不以预览像素烘焙。
- 初次代理准备、最终确认仍需后台计算；未移植PhotoCraft完整GPU合成器或mip缓存，不声称PS/PhotoCraft逐像素等价。
- C盘空间不足默认暂存余量，测试子进程TEMP/TMP定向D盘 `artifacts/resident-test-temp`，未改变全局配置或清理用户文件。
- 未运行全量 `tools/check.py`、跨平台、实体鼠标/平板持续手感或长时间GPU占用验证。


## 完整链接子树变换预览（2026-10-20）

- 捕获用户选择的根ID及原始图层/蒙版，复用移动的完整子树分割；带链接自身蒙版的根层、完整Normal/独立穿透根组和连续根多选可常驻。普通置入仍保留原始模型。直接单层入口没有捕获蒙版、固定蒙版、组内单层、非连续多选、复合置入与外部混合依赖保留回退。
- 核心新2项通过：三类目标×普通/Warp，初始分割真实合成、不同offset的世界坐标UV、8次更新静态源与原始瓦片身份、局部undo/redo、最终像素与CPU基线逐像素一致、单历史和取消；CPU已更新后才启用仍使用原始蒙版/图层，低预算提交失败保留源与会话并可取消，固定蒙版/组内/非连续/不完整目标拒绝。
- 变换预览整组8项、共享移动2项、绝对蒙版变换1项、无关组共享工具1项及长任务像素/低预算事务1项通过；原生 deferred_transform_packets_and_latest_geometry_commit 1项通过。
- 真实D3D11和软件Qt各5行通过：原平坦/无关子树，以及链接所有者蒙版、完整带蒙版组、连续多选。100次合并输入，确认前显示新位置、旧位置恢复背景；真实Warp变化、sourceId/image cacheKey保持、局部撤销、确认后撤销、取消与无QML警告。D3D11行显式要求窗口gpuDisplay为真。
- 同机Release固定白色组（图层蒙版+组蒙版）、Warp六输入×3次：512px CPU中位540.091ms→mesh2.035ms（7744顶点）；2000px CPU8625.200ms→mesh5.633ms（21025顶点）。2000px begin约9.242/10.078ms，只含核心会话捕获；不含实际代理纹理生成/上传、窗口显示或最终确认，不代表实体连续手感。
- ABI1.74、state44/VBR不变，生成头/格式、严格Clippy、四ABI消费者和Release构建通过，产物 `build/qt/bin/Release`。没有新GPU计算，CPU几何/Qt GPU显示职责保持。

复合预览先合成蒙版/层，再对2048代理采样；最终分别重采样原始层/蒙版，半透明边缘、局部Warp细分和极端放大仍允许近似差异。首次资产构建、选区、组内单层、固定蒙版和非连续目标继续待优化，未跑全量/跨平台/实体4K持续时延。
