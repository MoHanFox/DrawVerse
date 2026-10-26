# 图层移动实时预览验收

## 2026-10-26：CPU计算最新位移、GPU直接显示

移动已有三纹理/四顶点常驻渲染，本轮去掉持续拖动显示对16ms提交和后台回包的依赖。核心确认BEGIN实际来源后，首个零位移包成为不可变基线，CPU按最新绝对输入更新四个UV；GPU复用同源纹理。后台仍验证并发布实际偏移、revision、最终单条历史。旧回包重用最新显示位移，不累加误差；场景图以独立presentationDelta失效，不伪造文档revision。取消立即显示零位移，快按快放不等待资源；无实际常驻来源则放行CPU回退。ABI1.80/state45/VBR保持。

新增 `residentMoveUsesLatestInputBeforeBackendRoundTrip`：同一调用栈、不运行事件循环，连续正/负/零/最终位移立即更新显示包，同时revision、提交sequence、source ID、图片cacheKey保持；真实窗口红点移动/原位清除、上传计数不增加、取消不增历史、快速释放采用最终位移、undo/redo目标像素检查通过。软件明确不产生常驻包。原组/链接蒙版/连续多选、复杂固定/祖先蒙版、4K未预热/预热首拖及共享变换/液化回归保持。

Windows 11 / Qt6.8.3 / MSVC Release验证：

| 验证 | 结果 | 本地日志 |
| --- | --- | --- |
| 完整Qt变换/输入套件，D3D11 | 84 passed，0 failed | `artifacts/move-direct-final-d3d11.txt` |
| 完整Qt变换/输入套件，software | 84 passed，0 failed | `artifacts/move-direct-final-software.txt` |
| 移动鼠标/平板、Ctrl拾取、位置锁专项，两后端分别运行 | 各5 passed（含初始化/清理），0 failed | `artifacts/move-direct-ui-rhi.txt`、`artifacts/move-direct-ui-software.txt` |
| D3D11，QT_SCALE_FACTOR=2（本机实际DPR3），即时显示/4K首拖/抓图关联反例 | 5 passed（含初始化/清理），0 failed | `artifacts/move-direct-final-dpi2.txt` |
| C11/C++20静态/共享ABI | 4/4 | `ctest --test-dir build -C Release -R '^abi_' --output-on-failure` |
| 完整Release构建、生成头一致检查 | 通过，应用部署到 `build/bin/Release/drawverse.exe` | `artifacts/move-direct-build-final.log` |

初次新增测试在种子笔触完成前记录历史基线，已改为等待该笔触单历史回执。初次软件完整套件为83通过/1失败，独立导出显示移动后白/红像素均正确、无意外输入，但所有抓图均缺ready记录：软件渲染先填充image，测试过早满足非空条件并断开排队ready信号。`PendingMoveEvidence::capture`现同时等待真实ready回执和非空图像，保持5秒界限与全部原有像素/身份条件。独立复验8次抓图均有ready，3次关联成功、0丢弃；证据在 `artifacts/move-direct-sw-ready-evidence`。随后完整软件84/84及高DPI硬件复验通过。没有放宽断言或修改产品等待。

未运行全量 `tools/check.py` 或跨平台测试；本轮未改Rust/ABI。构建仅有Qt SDK缺少translations/catalogs.json的既有部署警告。实体鼠标/数位板持续手感、首次冷页准备及复杂栈CPU回退的进一步GPU合成仍待；窗口抓图计时不作为端到端输入延迟或FPS结论。

## 2026-10-21：组内子层常驻

新增两个祖先级别的Normal/PassThrough组内单层（含自身链接蒙版）源切分；真实组与瓦片不改。核心移动10项通过，扩展的完整逐像素对照覆盖六种来源、多次位移、半透明/负坐标/不动的同组兄弟、取消与单历史/undo/redo。新增祖先opacity/fill/蒙版/特殊混合及隔离背景依赖五类回退验证。共享液化3项、变换8项、原生延迟包1项回归通过。

软件Qt五行与真实D3D11五行通过，新增Normal/PassThrough两级组内目标；硬件要求gpuDisplay为真，32次鼠标移动后松手前像素跟随、sourceId与纹理cacheKey不变、四顶点、最终单历史与undo均通过。四ABI顺序、严格Clippy、格式、生成头与完整Release通过，输出 `build/qt/bin/Release`，ABI1.74/state44不变。

祖先半透明、祖先蒙版、固定自身蒙版、非连续多目标及背景相关混合仍保留CPU回退；本项未将96px窗口功能验证当作4K实体持续时延测试。

2026-10-19，Windows 11 / Qt 6.8.3 / MSVC 2022，分支 `feat/wgpu-support`，ABI 1.63。

设计与代码入口见[移动设计](../../tools/layer-move-preview-design.md)，GPU 范围见[加速规则](../../platform/gpu-acceleration-design.md)。

## 核心与 ABI

- `cargo test -p paint-core --lib layer_move --locked`：3 项通过。绝对位移预览、单条历史、取消/撤销/重做、组/蒙版、位置锁、非法最终位移回滚、画布外真实红色像素往返、瓦片身份保持；置入图片保留原始模型并回退。
- `cargo test -p paint-core --lib deferred_drag --locked`：1 项通过。共享源切分调整后的液化原图/来源身份与最终像素回归。
- `cargo test -p paint-ffi --test async_session layer_move_resident --locked`：1 项通过。1000 次绝对更新、最终提交、容量不足不部分写、过期网格拒绝、同源资产复用、CPU 预览真实像素和取消。
- `cargo test -p paint-ffi --test async_session deferred_liquify_packets --locked`：1 项通过。保留 ABI 1.62 网格/资产协议。
- `cargo test -p paint-ffi --lib resident_preview_keeps_render_floor --locked`：1 项通过。移动/液化连续预览允许 CPU 帧完成，完成/取消提升渲染屏障。修复既有输入分页测试夹具缺少 `preview_assets` 初始化，未更改分页运行逻辑。
- 核心与 FFI lib Clippy `-D warnings` 通过；FFI 使用 `--emit=metadata` 避免当前 Rust/MSVC 链接工具问题。生成头严格检查通过；C11/C++20 静态/共享四 ABI 检查通过，含 64 位能力掩码和共享 DTO 布局。

## 界面与构建

- Windows / D3D11 RHI：`layerMoveFollowsBeforeReleaseAndCancelsWithSingleHistory`、`liquifyResidentMeshRendersAndCommitsWithoutCpuFrames`、`adaptiveCursorContrastsBothSidesAndTracksLiquifyPreview`、`cursorLegacyBrightnessIsIgnoredByAdaptiveDisplay` 四项功能通过，含初始化/清理共6 passed、0 failed。记录：`artifacts/gpu-move-qt-hardware-final.txt`。
- Windows / software：新移动用例、`moveToolMouseKeyboardAndPositionLock`、`firstLayerSearchClickAndCtrlCanvasPickMove`、轮廓旧亮度回归、`liquifyMouseAndPenCanStartOutsideClippedCanvas` 五项功能通过，含初始化/清理共7 passed、0 failed。记录：`artifacts/gpu-move-qt-software-final.txt`。
- 新移动用例在松手前验证真实画面移动，硬件确认四顶点常驻包、软件确认新像素帧；同时覆盖鼠标/合成平板、快速空白 Ctrl 拾取、最终位移、Esc、窗口停用和特殊混合的 CPU 回退。
- 早期软件用例发现全局 `QQuickWindow::graphicsApi()` 与实际软件场景图不一致：错误进入常驻路径后主像素帧停止更新，软件不能显示自定义材质。现改读窗口 `rendererInterface()->graphicsApi()`，`gpuDisplay` 在窗口绑定/场景图初始化通知 QML，液化和光标共用正确回退。最终通过的是原生 Windows 软件后端，未依赖 offscreen 绕过该问题。
- 完整 Windows Release 构建、Rust 格式检查与生成头字节检查通过。产物：`build/verify-ui/bin/Release-wgpu-move/drawverse.exe`，许可与 Qt 运行依赖同目录。

测试后端变量仅作用于各测试进程：硬件 `QT_QPA_PLATFORM=windows QT_QUICK_BACKEND=rhi QSG_RHI_BACKEND=d3d11`；软件 `QT_QPA_PLATFORM=windows QT_QUICK_BACKEND=software`。液化旧用例的72–73ms异步取帧包含50ms轮询，不用作移动延迟或FPS指标。

## 边界

常驻路径限普通兼容 Normal 栅格栈，静态纹理最长边2048，移动网格固定四个顶点；组/蒙版/裁剪/特殊混合/置入图片与软件路径保留 CPU 像素预览。首次静态资产构建仍有后台开销，不宣称任意大图零起始延迟，也不将此轮功能检查当作 FPS 或实体设备性能结论。

未运行全量 `tools/check.py`、跨平台和实体鼠标/平板持续手感验收；平板覆盖为合成 Qt 事件。测试进程 TEMP/TMP 使用仓库忽略的暂存目录，未改变全局设置。既有 Release 程序仍运行，未终止用户进程。


## 2026-10-20：完整子树与连续多目标常驻源

- 移动单独调用 resident_move_source，完整保留根组/链接蒙版与连续多选的后序子树，统一归一化非组节点的绝对坐标；原普通源也复用 partition_resident_source，液化/变换资格保持各自边界。组和蒙版不会每次位移重复合成，纹理首次生成后拖动仍只改四顶点。
- 新核心2项及既有移动7项通过：带负坐标和半透明的自身蒙版、Normal隔离组内Multiply+剪贴、带蒙版的PassThrough组、连续双目标的完整画布CPU像素对照；源/瓦片身份、取消/单历史/undo/redo保持。未链接蒙版、依赖背景透传、非连续目标、显式蒙版和上方剪贴回退；Dissolve依赖画布坐标随机数，也回退。原测试中两个“蒙版/多目标必须回退”断言随新资格更新。
- 共享液化预览3项、变换预览6项通过（含所有插值/选区/折叠/物化失败）；原生多目标API1项通过，校验复制ID数组和最终单历史。
- Windows D3D11真实窗口新增3行通过：自身链接蒙版、带蒙版组、连续多选。真实鼠标拖动32次、松手前像素跟随/原位清除、四顶点、相同sourceId与图片cacheKey、最终单历史及undo恢复。软件新增3行和共享变换2行通过；D3D11共享变换2行也通过。
- 四ABI顺序运行、严格Clippy、格式、生成头及Release应用/QtUI/工作流/变换/四ABI目标构建通过；ABI1.73/state44/VBR1.0保持。产物build/qt/bin/Release。
- 静态源最长边2048代理和首次源构建开销保持；组内单子层、固定蒙版、非连续多目标/复杂背景相关组合、置入源等仍回退。没有新增wgpu计算，也未将96px功能用例作为4K连续时延或实体设备手感验收；未跑全量/跨平台。


## 2026-10-22：撤销512px移动纹理并改为工具/目标切换预热

提交 `e5b8f19` 把移动的 `image/below/above` 三张常驻纹理共同限制到最长边512px，导致正常画布移动时用户可见画质下降。本轮撤销该策略：三图重新按当前视口物理像素密度稀疏采样，最高2048px；没有可见像素的切片仍编码1×1透明纹理。四顶点网格、同源缓存、最终原层坐标、取消与单条历史不变。CPU复杂回退的稀疏输出也取消512px上限，改用最高2048px的请求密度，避免保留另一个可见移动降质入口。

新增ABI1.80后台移动准备入口。切换移动工具或目标图层/多选集合时，文档actor只捕获不可变源，三图采样/编码在CPU池执行；结果绑定请求ID、document generation、revision和有界目标ID集合。Qt在空闲画布上预热同源纹理，但不在正式BEGIN前显示代理。首次按下若预排已就绪，拖动只改四顶点并复用已有上传；若尚未就绪，BEGIN不等待资源，保留现有完整分辨率帧并由同视口物理密度的稀疏完整帧路径更新，不进入512px移动纹理。预排完成后旧revision结果被拒绝。

4K完整填充、真实黑色链接蒙版与组内子层的原生资产对照通过：包中采样中心与完整文档合成颜色差≤1，空切片透明，原/显示密度尺寸符合物理密度及2048px上限，取消后历史保持。`prepared_layer_move_is_density_preserving_and_bound_to_target_revision` 还验证目标/request切换、BEGIN消费同一source ID，以及提交后的revision失效；ABI布局固定 `PaintMovePreparationInfo=56` 字节。

真实Windows Release用例 `prewarmed4kResidentMoveReusesFullQualityTexturesAndRestoresFullFrame` 覆盖：先使用选择工具、切换移动后不额外点击画布；预排未完成时立即按下拖动；取消不增历史；随后等待预排及纹理上传，再次首拖时四顶点、同源cacheKey、空上层1×1、上传计数不增加；最终完整视口、单历史及undo。软件、D3D11和D3D11 2倍DPI均通过。

本轮窗口观察值如下，均包含Qt事件、轮询、抓图和首次呈现，不是单一输入处理或端到端实体延迟：

| 后端 | 预排未完成时按下到首次变化抓图 | 预排到纹理预热完成（含等待） | 预热后按下到首次变化抓图 | 最大GUI心跳间隔离散值 |
| --- | ---: | ---: | ---: | ---: |
| software | 378ms | 687ms | 318ms | 14ms |
| D3D11 | 358ms | 615ms | 161ms | 11ms |
| D3D11 2倍DPI | 647ms | 1108ms | 126ms | 11ms |

表内预热耗时是CI/窗口测试的累计等待，不是资源准备中位数；首次变化抓图时间也不是鼠标事件处理时间。预热后首拖的纹理上传计数保持不变，但画面轮询/抓图仍可能占主要部分。不得据此声称实体设备瞬时完成或冷盘已解决。

`e5b8f19` 的512px资产基准与“中间2048方案”不再作为最终质量策略，历史日志 `move-start-density-*` 仅保留作调查记录。最终验收以本轮 `move-prewarm-*` 日志为准。ABI1.80/state45/VBR保持，无新增GPU计算算子。

最终源码验证：`paint-core`移动核心10/10、`paint-render`交互4/4、`paint-ffi`库70通过+1手工忽略、移动/ABI相关`async_session` 63/63；完整`async_session`文件中另有outline与wet-dual两个VBR pressure behavior/capability用例在本次未触及的既有数据校验上单独失败，未混入本项通过计数。完整Qt变换套件软件与D3D11各69/69、D3D11 2倍DPI首拖通过；四套C11/C++20静态/共享ABI 4/4、严格Clippy、fmt与生成头一致检查通过。完整Release构建输出`build/bin/Release/drawverse.exe`、`paint_ffi.dll`及运行依赖，只有Qt SDK缺少translations/catalogs.json的既有部署警告。
