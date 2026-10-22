# 图层移动实时预览验收

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


## 2026-10-22：首次移动静态资产按显示密度与草稿预算准备

原移动首次常驻包沿用完整CPU合成/LOD，且三个切片均固定按最长边2048准备。新增移动专用资产策略：复用共享稀疏坐标采样，按两个显示视口中较高的物理像素密度确定尺寸，但不超共享 `MOVE_PREVIEW_EDGE=512` 移动草稿预算与原分辨率；没有可见像素的切片编码为1×1透明纹理。高DPI也不扩大草稿工作量。源身份缓存、三纹理ABI协议和四顶点不变。液化/变换的代理仍走原完整合成；移动完成仍提交原层坐标，后续完整视口、保存、历史均沿用原路径。

4K完整填充、真实黑色链接蒙版、组内子层、隐式白底的同机Release基准，三次分别新建移动会话并构建静态资产；请求模拟1024px整图显示需求：

| 项目 | 原2048策略 | 最终512草稿策略 |
| --- | ---: | ---: |
| 三次资产构建 | 635.956 / 597.958 / 611.917ms | 34.714 / 34.631 / 35.281ms |
| 中位数 | 611.917ms | 34.714ms |
| 单包字节 | 28,385,304 | 1,185,820 |

中位数减少约94.3%。这里只测CPU资产构建及编码，不包含源文档创建、输入排队、Qt解码/纹理上传或屏幕显示，不是实体输入时延或冷盘测试。收益包括减少代理分辨率、稀疏采样和空切片压缩，不是同尺寸同LOD的纯算术加速。点采样草稿可能有锯齿/细线近似，sRGB查表允许1色阶误差；只影响移动预览，松手恢复完整显示，不限制文档尺寸和最终像素。

最初仅按显示密度、仍保留2048上限的中间方案，资产中位数128.976ms、包4,741,424字节，但窗口首个变化抓图D3D11仍305ms、2倍DPI564ms，故最终统一到既有CPU回退512草稿标准。以上窗口计时包含轮询/抓图，没有与原2048版本做窗口前后对照，不是实体单输入时延。中间记录保留于 `move-start-density-{assets,d3d11,d3d11-dpi2}.txt`。

原生新增资产对照通过：链接有效蒙版、Normal组、半透明、负坐标画布外源、原显示密度及1/4密度，包中采样中心与完整文档合成颜色差≤1，空切片透明，取消后历史保持；同时确认普通变换/液化编码尺寸保持。

共享渲染24项、移动核心10项、Actor屏障及变换预热各1、原生移动2/液化1通过。严格Clippy（core/render/ffi全部目标）、fmt、生成头和四套ABI均通过。

真实Windows窗口软件与D3D11各13个工作流（含init/cleanup各15 passed），另D3D11 2倍DPI首拖1项（含init/cleanup 3 passed）通过，无失败或跳过。新增4K种子在BEGIN尚未确认、常驻包尚未到达时立即发送第一下鼠标移动；检查原位清空、链接蒙版随动、正确位置、512px GPU源、空上层1×1、上传一次、后续同源/图片复用、最终完整视口和单历史/undo。共享用例保留十类复合移动及两类有效固定/祖先蒙版4K回退、确认/取消/历史。

自动序列从按下到第一个变化抓图：软件121ms（最长GUI心跳13ms）、D3D11 160ms（11ms）、D3D11 2倍DPI216ms（11ms）。含轮询/抓图与首次材质/纹理准备；没有设置时间断言，不能解释成单次输入延迟、FPS或实体设备手感。2倍DPI仍有额外呈现开销，未称首次移动瞬时完成。冷页IO、首次驱动材质以及任意深组的持续时延仍待实际剖析。

完整Release构建通过，`build/bin/Release/drawverse.exe` 与DLL于2026-10-22 13:20更新；ABI1.79/state45/VBR保持，无新增GPU计算算子。

日志：`artifacts/move-start-{before,assets-final,render,core,actor,actor-transform,native,native-liquify,clippy,fmt,build,abi,software,d3d11,d3d11-dpi2}.txt`。
