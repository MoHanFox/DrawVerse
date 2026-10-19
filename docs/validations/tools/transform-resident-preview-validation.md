# 自由变换常驻纹理预览验收

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
