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

## 2026-10-21：控制框不合并附带蒙版

核心新增attached_masks_never_define_transform_handles通过：普通/常驻、根层/组、链接/固定共8组合，全遮挡且范围更大的蒙版不改变原内容控制点；取消保留像素/历史，显式蒙版独立编辑仍用自身范围。共享变换预览整组9、公开transform7、原生deferred_transform_packets_and_latest_geometry_commit1通过。

真实Qt客户端新增attachedMaskDoesNotChangeTransformHandles在软件与D3D11各通过：开启/取消原图变换，添加大白色遮挡蒙版后再次开启，16个控制点相同且无错误。四ABI依次、严格Clippy、格式与Release通过。原运行EXE锁定，新程序为build/qt/bin/Release/drawverse-next.exe；ABI1.74保持。未跑全量/跨平台/实体持续手感。

## 2026-10-21：4K 局部选区常驻变换（T1）

进入普通Normal根层的选区变换时，一次分割原始选中像素与静态余像素；原位另用原图合成资产，避免羽化/alpha边界出现切缝。原始图层瓦片和选区留在会话，拖动只发布CPU网格，Qt GPU显示，确认才按原六种取样生成最终像素与选区。软件保持逐像素路径。图层属性opacity/fill低于1、自身蒙版、组内、多目标、特殊混合继续回退，像素本身半透明及羽化已支持。

- `cargo test -p paint-core --lib transform::preview::tests`：12项通过。新增3项覆盖透明/不透明、偏移、羽化、普通/Warp、全部六种最终取样与CPU逐像素等价；原始瓦片/移动源/原位源身份、局部undo/redo、延迟启用、取消/永久历史；受限捕获回退与真实暂存失败的新会话回滚。
- `cargo test -p paint-core --test transform`：7项通过。旧折叠用例的“选区必须回退”断言已过时，改为同时验证CPU实时路径和常驻确认路径；两者保留折叠像素alpha/目标选区覆盖相同、永久undo恢复原像素与选区的断言。
- `cargo test -p paint-ffi --test async_session deferred_transform_packets`：2项通过，含选区身份切换、1000次绝对更新、陈旧包/容量不足拒绝、最新几何确认、局部历史/取消及真实像素。
- 新Qt `selected4kTransformKeepsResidentTexturesAndRestoresHistory` 普通/Warp两行，在软件与真实D3D11各通过。4096×2160透明目标层上，羽化多边形只移动红色半透明笔触，外部蓝色笔触不变；确认前实际窗口显示移动结果/原位置空洞，连续12次更新保留sourceId和三张图像cacheKey，硬件不生成CPU帧；确认/撤销/重做/取消及选区恢复通过，无QML警告。
- 共用Qt普通栈/无关子树/链接蒙版/完整组/连续多选5行、折叠1、取消关闭状态1、蒙版不改控制框1，加新4K两行，共软件/D3D11各10行通过（含初始化清理显示12 passed）。日志 `artifacts/selected-transform-shared-{software,d3d11}.txt`。修复截图夹具后D3D11 Warp独立连续复跑3次通过，日志 `selected-transform-final-warp-{1,2,3}.txt`。

首次硬件测试捕获了截图夹具访问异常，调试栈位于 `QQuickItemGrabResult::render`：新夹具轮询图像非空，尚未收到ready就释放截图对象。改为等待Qt的ready信号后读取，未改生产渲染或压低颜色断言。另一次夹具误把文档历史深度与会话局部深度比较，现分别记录；保留确认只增一条文档历史。没有把两次失败作为产品已通过的证据。

同机Release固定4096×2160隐式白底、约256px多边形套索、强Warp六次输入×3遍：

| 范围 | 原CPU像素预览 | 常驻几何预览 |
| --- | ---: | ---: |
| 核心begin（不含Qt资产） | 17.278ms | 24.054ms |
| 六次更新+网格发布中位数 | 334.561ms | 1.556ms |
| 最终commit | 0.319ms | 61.887ms |
| 最终像素摘要 | `42e02ac6a1396125` | `42e02ac6a1396125` |

示例 `cargo run -p paint-core --release --example transform_performance -- 4096 <pixel|mesh> warp selection`，日志 `artifacts/selection-transform-{cpu,resident}.txt`。旧路径已在更新时物化，所以commit较短；新路径把取样留到确认。两者最终19瓦片、摘要相同。计时不含Actor/Qt调度、代理生成/上传、GPU显示或实体输入，不能当端到端时延。

四ABI消费者顺序通过，生成头检查、格式、core/ffi全目标严格Clippy与Windows Release通过；ABI1.75、state45/VBR保持。用户正在运行旧drawverse.exe，未终止该进程，新程序为 `build/qt/bin/Release/drawverse-next.exe`。两套首次资产仍各可能构建/上传；2048代理与有限网格为预览近似，最终不采样代理。未跑全量check、跨平台、实体连续拖动及GPU持续占用。

## 2026-10-21：组内目标与移动共享常驻源（T2 第一项）

移动的私有祖先消去抽为 `resident_hierarchy_source(ids, originals)`，变换传不可变会话原层，移动传空覆盖。仅显示快照提升祖先直属子树；覆盖原层后保留提升过的parent，真实层结构不变。支持可消去的Normal/PassThrough多级组内完整普通目标及其链接蒙版；祖先需可见、opacity/fill均1、无蒙版/剪贴，Normal祖先子树无特殊混合/剪贴。选区、固定/祖先蒙版、非连续目标及背景依赖继续回退，T2不标全部完成。

- 核心 `transform::preview::tests` 整组13项通过。复合用例扩为5种目标×普通/Warp，新增两级Normal/PassThrough祖先、静态兄弟、半透明目标、带offset灰度蒙版；初始分割合成、8次更新source/瓦片/真实parent及属性保持、局部历史、确认与原CPU逐像素等价、单历史/永久undo/redo/取消。新增延迟启用用例逐像素核对会话原始源，低预算确认失败保留会话/源/历史，取消恢复；8种不可消去祖先/固定蒙版条件拒绝常驻。
- 公开变换7、移动复合3及液化共用预览3通过；`async_session deferred_transform_packets` 原生3通过，新增两级组内目标覆盖1000次更新、包身份/陈旧包/容量拒绝、最新几何确认及显式启用/取消。core/ffi全目标严格Clippy及格式通过。
- Qt `transformResidentPreviewFollowsAndCompositesTransparentStack` 软件/D3D11各7行通过（新增Normal/PassThrough组内两行），窗口实际像素核对目标100次更新后已移动、静态兄弟保持、source/image cacheKey复用、局部撤销/Warp变化、确认/永久历史/取消，无QML警告。日志 `artifacts/nested-transform-d3d11.txt`、`nested-transform-shared-software.txt`。
- 共用移动7行（含组内两种和固定/祖先蒙版回退）及4K选区变换2行，在软件/D3D11通过；硬件折叠与蒙版控制框通过。硬件快捷键焦点在混合批次等待窗口激活失败（尚未进入编辑步骤），独立启动同一用例通过，未改产品或删断言；日志 `nested-transform-shared-d3d11.txt`、`nested-transform-focus-d3d11.txt`，不能将原混合批次写为零失败。

Release固定2000×2000白色内容、全幅0.6灰度链接蒙版、两级Normal祖先，强Warp六次输入×3遍：

| 范围 | 原CPU像素预览 | 常驻几何预览 |
| --- | ---: | ---: |
| 核心begin（不含Qt资产） | 20.202ms | 22.309ms |
| 六次更新+网格发布中位数 | 18335.863ms | 5.087ms |
| 最终commit | 355.272ms | 3231.376ms |
| 最终合成像素摘要 | `6a20870d922fcdcb` | `6a20870d922fcdcb` |

`cargo run -p paint-core --release --example transform_performance -- 2000 <pixel|mesh> warp nested-normal`；日志 `artifacts/nested-transform-{cpu,resident}.txt`。新路径21025顶点，原完整图层/蒙版取样转到确认。数字不含Actor/Qt调度、初次代理生成/上传、GPU显示和实体输入；不承诺8K/16K或持续时延，最终3.2秒仍是后台完整提交工作。

最终Windows Release及Qt/四ABI目标构建通过，生成头一致，四ABI消费者顺序通过；新二进制两条组内用例D3D11再次通过，日志 `nested-transform-final-d3d11.txt`。原EXE已无占用，标准产物更新为 `build/qt/bin/Release/drawverse.exe`，构建日志 `nested-transform-release-build.txt`。ABI1.75、state45/VBR不变，无新增wgpu算子。未跑全量check、跨平台、实体持续手感/占用，首次资产、固定/祖先蒙版与非连续目标仍待。


## 组内局部选区常驻预览（2026-10-21）

移动、完整变换与选区分割共用私有祖先上下文，移动/剩余/初始源保持该上下文 parent，真实组织不改。核心预览14项通过，其中两种祖先×六种最终插值×affine/Warp及alpha/羽化/offset确认与CPU完全相同，覆盖原始瓦片身份、局部/永久历史、延迟启用、取消和不安全祖先回退。公开变换7、共享复合移动3、液化预览3、严格Clippy与格式/生成头通过。

真实Qt 4096×2160局部套索：原affine/Warp及新增两级Normal affine/Warp、两级PassThrough Warp，共软件/D3D11各5行通过。确认前检查移动红色区域、静止蓝色余像素和背景，硬件连续更新保持sourceId/三图cacheKey/CPU帧修订，确认/撤销/重做恢复选区和像素。日志 artifacts/selection-nested-{software,d3d11}.txt。

Release固定4096×2160约256px套索、两级Normal祖先：六次Warp中位345.593→1.528ms，begin13.625→20.864ms，commit0.283→59.841ms，最终摘要两者42e02ac6a1396125。入口 transform_performance 4096 cpu/mesh warp selection-nested-normal；日志 artifacts/selection-nested-{cpu,mesh}-perf.txt。此计时仅核心与网格发布，不含Actor、首次代理/上传或窗口显示，确认原采样转为后台一次执行。

完整Release及四ABI构建和四消费者顺序通过，标准build/qt/bin/Release/drawverse.exe更新。固定/祖先蒙版、图层低opacity/fill、非连续及背景相关混合继续CPU回退；未跑全量/跨平台/实体持续时延。

## 2026-10-21 编辑结束快捷键恢复

用户报告结束液化/变换仍需点击画布。新增editingCompletionRestoresShortcuts在实际选项控件持有焦点、另一原生窗口激活时结束会话，旧代码transform-cancel-main稳定失败于宿主未激活。修复保留待恢复标记，等busy/窗口/可见状态满足后完成焦点恢复。

最终用例实际修改变换几何或液化像素后，通过选项确定/取消，主窗口/浮动文档共8行；结束后不调用focus辅助、不点击画布，直接E/B/M均成功。软件及D3D11各8行与欢迎页文档生命周期1行通过，零QML警告。日志artifacts/edit-focus-before.txt及edit-focus-final-{software,d3d11}.txt。完整标准Release构建通过，build/bin/Release/drawverse.exe更新；ABI1.75/state45保持。本项不宣称其他性能问题已完成。

## 2026-10-21 首拖CPU代理准备提前至进入工具

新增Actor回归确认：带羽化套索的BEGIN完成前，真实拖动sourceId的资产已缓存，和初始精确图sourceId不同；首次UPDATE直接返回同一Arc字节身份，三次初始/变形往返均复用两套，CANCEL清空。该回归1项、核心变换预览14项、paint-core/paint-ffi所有目标严格Clippy及fmt通过。

实际4K Qt套索affine/Warp及两级Normal/PassThrough五行、原工具完成快捷键一行，软件/D3D11各6行通过，含静止余像素、纹理身份、取消及文档历史。完整标准Release/生成头、四ABI通过，输出build/bin/Release/drawverse.exe。日志artifacts/transform-preparation-{actor,core,clippy,software,d3d11,abi,build}.txt。

没有将缓存命中或核心计时称为首拖端到端延迟；Qt显卡纹理首上传、复杂CPU回退及实体持续性能仍需验收。ABI1.75/state45保持。

## 2026-10-21 进入工具预读取及上传两套纹理

ABI1.78新增kind3静态包，提供与当前源同会话的备用sourceId和三图；Actor仍在BEGIN前准备CPU源。Qt后台一次解码两套QImage，源切换仅交换共享资产。ResidentPreviewTextures以窗口/sourceId共享主网格及上下层纹理，注册表仅弱引用，节点最多保留两套；主材质首次更新将备用纹理一起提交RHI批次。上下层改为会话Loader，退出不遗留隐藏节点强引用。

- 原生三项（普通、选区、组内）及Actor一项通过。新增kind3容量探测/不足不改缓冲区、过期generation、无备用NOT_FOUND、初始备用sourceId与实际首拖ID/字节相同。
- 实际D3D11 4K套索五行（affine/Warp、两级Normal/PassThrough）：拖动前两套各提交一次上传；首拖、两轮返回原位再拖动、12次持续更新后仍各一次，移动QImage cacheKey及共享计数身份与预读源相同。画面红色移动/蓝色静止、羽化原位显示、确认/取消/永久历史保持正确。该计数是上传批次，不是GPU时间测量或端到端零延迟承诺。
- 软件4K选区五行、复合移动七行、选区液化两行均通过；D3D11完整复合变换七行、移动七行及选区液化两行通过。编辑结束主/浮窗×确定/取消八行及欢迎关闭生命周期一行在软件/D3D11各通过，零QML警告。
- 四ABI、core/ffi全目标严格Clippy、fmt/生成头、完整标准Release通过。新程序build/bin/Release/drawverse.exe；state45/behavior37/VBR1.0保持。

日志artifacts/transform-gpu-preparation-{native,actor,d3d11,final-d3d11,software,shared-d3d11,workflow-software,workflow-d3d11,abi,clippy,fmt,header-check,check-build}.txt。两套三图GPU像素最坏约96MiB，另有现有视口目标；仅活动会话有强引用，跨窗口分别创建。上传在进入后的首个真实帧提交，极早输入仍可能与其重叠；未测驱动物理冷启动、实体首拖端到端、全量、跨平台或16K整图。


## 2026-10-22 空蒙版及零不透明度蒙版共用预览

仅允许全域恒等覆盖：空蒙版（无置入/隐式底色/瓦片）或opacity=0。共享判断用于自身、固定蒙版及可消去Normal/PassThrough祖先；私有三图移除这些蒙版，真实层级、内容、链接和最终采样不变。有实际覆盖的隐藏或fill=0蒙版仍回退。

核心新增三项通过，涵盖72组合（自身/两类祖先×空/opacity=0×链接/固定×选区/无选区×变换/移动/液化），逐瓦片像素/属性/结构、确认/撤销/重做与原CPU路径完全相同；取消保持原文档。三张显示快照独立层级验证及读取，初始和拖动后均通过。旧安全拒绝样本改成实际alpha遮挡，而非用空蒙版代表复杂蒙版。共享液化预览7、变换14、移动10项通过；公开变换7/液化11、原生变换3/移动1/液化1通过。

软件与D3D11各24行实际窗口工作流通过（测试总数各26含初始化/结束）：新增九行覆盖三工具各三类蒙版；4K套索Warp两套纹理进入时各上传一次，12次更新及两轮原位往返均保持源ID、图像身份与上传次数；静止余像素、确认/取消/历史保持。实际遮挡固定/祖先蒙版4K两行仍回退并正确显示/结束。液化三新增行保留所选外像素且确认/取消/会话与文档历史通过。

首次D3D11验证发现单层分割把空蒙版留在没有owner的背景快照，导致postorder parent错误；已修复为在私有快照过滤恒等覆盖，并补齐核心三图校验/读取。初次失败日志保留为artifacts/identity-mask-d3d11.txt；修复后的最终两后端日志均全部通过。没有删除安全断言或屏蔽错误。

单次1024px核心合成工作负载，旧像素路径六次Warp更新合计2422.884ms；新路径只更新CPU几何，完整颜色计算移到确认351.249ms，最终所有瓦片逐像素一致。此数字不含Actor代理图合成、Qt纹理上传、GPU呈现、事件调度或冷盘，不作为用户端拖动延迟；没有声称实际遮挡蒙版全部可常驻。

core/render/ffi全目标严格Clippy、fmt、生成头检查与四ABI消费者通过；完整Release构建部署成功，build/bin/Release/drawverse.exe与paint_ffi.dll更新。ABI1.79/state45/behavior37/VBR1.0不变，无新wgpu算子、协议或预算。

最终日志：artifacts/identity-mask-final-{liquify,transform,move,public-transform,public-liquify,native-transform,native-move,native-liquify,software,d3d11,clippy,fmt,header,abi,workload}.txt；构建artifacts/identity-mask-build-final.txt。手工计时测试identity_mask_transform_preview_workload默认忽略，需--release --lib --ignored显式运行。其余真实遮挡固定/祖先蒙版、半透明祖先、非连续目标、首次资产、冷盘与实体持续性能仍分别按T2/M1/L1/P1/H1跟踪。


## 2026-10-22 复杂变换CPU采样瓦片寻址

SourceSampler保留原64个私有像素副本和FIFO语义，将缓存值改为稳定槽索引，连续邻近采样直接访问上个槽；缺失瓦片只记坐标，仍逐点计算隐式底色。源瓦片加载/选区覆盖成功后才替换旧槽，不长期pin源页，不改变插值权重、求和顺序或选区算法。用于所有六种插值、复杂栈预览、延迟预览的最终确认及原位图变换路径；图层移动和液化算法本身未改。

新增三项通过：独立逐点像素参考覆盖负坐标/80瓦片正反往返淘汰、隐式白底同瓦片两条边界和带偏移羽化选区；源读取失败重复返回错误且保留已缓存样本，修复源后可重试；单瓦片PagePool驻留读取80页后仍可回收、不保留页pin。变换相关核心24项/公开7/原生4通过，含六插值、折叠Warp、蒙版与历史失败回滚；三个手工计时用例默认忽略，不计为普通通过。

同一1024px核心工作负载、每种两次更新，修改前后单次测量如下，四种最终全图显示结果摘要均相同：

| CPU回退样本 | 原耗时ms | 本轮ms | 减少 |
| --- | ---: | ---: | ---: |
| 固定自身蒙版 affine | 695.550 | 304.707 | 56.2% |
| 固定自身蒙版 Warp | 752.794 | 342.956 | 54.4% |
| 选区及祖先蒙版 affine | 257.874 | 150.286 | 41.7% |
| 选区及祖先蒙版 Warp | 273.087 | 167.262 | 38.8% |

上述只计update_transform，未计进入工具/Qt上传/显示/事件调度/冷盘，不是端到端延迟或统计中位数，也没有承诺复杂变换已经足够跟手。运行：core目录cargo test -p paint-core --lib --release --locked complex_mask_sampling_workload -- --ignored --nocapture。基线/新日志为artifacts/transform-sampling-{before,after}.txt。

实际软件/D3D11各22行工作流通过（各24总数含初始化/结束）：4K套索10行含新增有效固定自身蒙版及祖先蒙版回退，支持栈常驻纹理及预上传复用不变；复合栈7行、折叠Warp一行含两档透明度、整图/工具翻转历史四行。静止余像素、确认/取消和文档历史正确；GPU折叠显示保持已有近邻容差（两档分别302/117像素，均低于20736像素的2%），不称预览与最终完全一致。所有原断言保留，未屏蔽QML警告。

core/render/ffi全目标严格Clippy、fmt、生成头、四ABI消费者和完整Release构建部署通过。build/bin/Release/drawverse.exe与paint_ffi.dll更新，ABI1.79/state45/behavior37/VBR1.0保持，无新增GPU算子/缓存容量/任务上限。T2仍部分完成：有效固定/祖先蒙版继续CPU路径，后续需减少每输入颜色物化；首次资产/冷盘/实体手感另行验收。

日志：artifacts/transform-sampling-{core,transform,public,native,software,d3d11,clippy,fmt,header,abi,build}.txt。


## 2026-10-26 完成时旧画面闪回修复

共享客户端不再在关闭元数据时提前释放 resident，等待最终主视口帧后交接；若最终帧先到则立即交接。新增自由变换提交/取消以及移动对应两行，刻意禁用主视口，在最终概览帧和关闭元数据均已到达后断言预览仍在，恢复主视口后验证最终像素与历史。完整变换输入软件/D3D11各89项、实际 Ctrl 移动 UI 两后端各4项、完整Release/生成头/四ABI通过。标准 build/bin/Release/drawverse.exe 已更新，无ABI/state或最终取样变化。过程与日志见[移动验收](layer-move-preview-validation.md#2026-10-26-ctrl-拾取即时显示与结束闪烁修复)，未声称实体端到端性能验收。


## 2026-10-26 预热清理错误与静止空转

已复现用户截图 transform active，与液化Busy来自相同移动预热清理反馈循环。Qt清理幂等，核心将清缓存作为编辑期间允许的非文档操作；既有编辑互斥保持。D3D11静止400ms的31次清理/31次窗口呈现降为0/0，无实际错误；软件同样通过。按用户要求仅跑新增公开ABI一项、Qt两行各后端4项及四ABI/头/格式/Release构建，未跑全量或GPU利用率百分比。标准程序已更新，详见[共享验收记录](layer-move-preview-validation.md#2026-10-26-液化自由变换错误循环与静止重绘)。


## 2026-10-29：大图取消控制框立即退出

PaintCoreClient独立transformControlsVisible控制QML边框寿命，确认/取消成功入队立即隐藏，像素预览仍按原最终revision交接。关闭期间拒绝UPDATE/CHECKPOINT与重复结束，避免隐藏MouseArea的onCanceled提交旧点；元数据完成后恢复可开始状态，关闭失败仍活动时恢复控制。generation变化重置关闭标记。该控件只通过transformChanged通知，元数据最终更新active后再通知，避免属性读取顺序短暂复活。

新增4096×2160实际CanvasPane测试在不处理后台事件前断言QML边框已隐藏，检查旧UPDATE被拒绝、取消历史不增、再次进入边框可见。连同BEGIN旧点、非法中间态、空变换取消、Warp轮廓与折叠回归，软件/D3D11各8行通过（含init/cleanup），artifacts/cancel-direction-{software,d3d11}。GPU未确认关闭commit/cancel及原取消/快释放6行通过cancel-direction-gpu。此测试为白底大图与控制可见性，不冒充用户实体截图或任意尺寸验收。

Release构建/生成头检查通过，标准build/bin/Release更新；用户运行的TransformReview进程未关闭或替换。标准独立profile D3D11 --preview冒烟退出0并输出PNG，见cancel-direction-smoke；构建日志build/cancel-direction-runtime-build.log。ABI/state/文档不变，未跑总check。
