# DrawVerse GPU 加速：规则、用法与代码入口

日期：2026-10-20；当前分支 `master`（GPU 功能已合并）。本文是当前加速范围与接入规范的入口，实际验证范围见各验收文档。导航与光标的本轮实现状态在 [验收](../validations/interface/canvas-navigation-adaptive-cursor-validation.md) 中记录。

## 两条 GPU 路径

2026-10-21套索液化扩展（ABI1.77）：兼容普通根层允许带选区使用常驻网格预览，CPU按行采样选区、Elastic更新场，Qt GPU复用原图/上下层/目标坐标遮罩四张静态纹理。拖动不逐次物化颜色，确认才完整重采样；没有新增wgpu算子。软件及复杂图层组合保留像素路径，2048代理和遮罩允许预览近似。代码为paint-core的liquify_preview/liquify_operators、paint-ffi的liquify_api kind2、Qt的LiquifyPreviewNode和liquify.frag，详见[设计](../tools/liquify-resident-preview-design.md)及[验收](../validations/tools/liquify-resident-preview-validation.md)。

DrawVerse 使用 wgpu 做 Rust 算子计算，使用 Qt Quick 场景图/RHI 做界面显示。两者职责、设备和资源生命周期独立；选择 wgpu 设备不等于切换 Qt 显示设备。CPU 保持文档、历史、输入和最终像素真值。

### 算子计算

`core/crates/paint-ops` 定义参数与 CPU 参考实现；`core/crates/paint-gpu` 提供 wgpu 执行器与 WGSL。文档通过算子执行器调用，不依赖 Qt。优先自动选择可用硬件，设置中的性能页可指定设备或 CPU；不支持、设备失败或超出预算时使用 CPU 路径，状态需可见。

算子必须明确输入、输出、坐标/颜色空间、边界采样规则和资源预算。CPU/GPU 使用同一业务参数和调度接口，GPU shader 不承担历史或工具状态。浮点计算按容差对照，历史与取消仍由文档事务保证。

### 显示

`ui/src/canvas/CanvasItem.cpp` 持有 GUI 线程快照，`updatePaintNode()` 在场景图同步阶段更新节点。纹理由场景图拥有，源身份不变时复用，不能把 QSGTexture/Qt 对象传进 Rust 或工作线程。移动/缩放只改显示几何，不能每帧提交全图像素作导航。

显示能力以窗口 `rendererInterface()->graphicsApi()` 为准，窗口绑定/场景图初始化时更新；全局 `QQuickWindow::graphicsApi()` 只是图形 API 设置，不能据此判断软件场景图是否支持 ShaderEffect 或自定义材质。

自定义材质使用 `ui/shaders` 的 GLSL，经 `ui/CMakeLists.txt` 中 `qt_add_shaders` 烘焙为 `.qsb`，由 Qt 生成各后端 shader。上传 QSGTexture 时在 `updateSampledImage()` 调用 `commitTextureOperations()`。资源只能在所属渲染生命周期创建/销毁；软件显示应有明确回退。

## 已完成范围

| 功能 | GPU 路径 | CPU/未覆盖范围 |
| --- | --- | --- |
| 弹性液化算子 | wgpu 场更新与重采样算子，自动设备与失败回退 | CPU 参考实现、文档历史、取消与精确提交；不是所有工具都 GPU 化 |
| 液化常驻预览 | Qt 静态原图/上下层纹理 + 有界网格，拖动只更新网格 | 普通兼容栅格栈使用此路径；选区、复杂组合、软件/CPU 模式保留像素预览 |
| 图层实时移动 | Qt 共用原图/上下层常驻纹理 + 2×2 位移网格，拖动无全图回读 | 核心偏移/历史与提交保持 CPU；无关组/蒙版可留在静态上下栈，完整根组/链接蒙版与连续根多选可复用静态源；组内普通目标可穿过100%不透明度/填充且无蒙版的可消去Normal/PassThrough祖先；未链接蒙版、非连续多选、背景相关混合、Dissolve、置入图片和软件显示仍回退 |
| 自由变换/控制点变形 | CPU正向有界细分 + Qt原图常驻纹理/视口离屏合成，更新不物化全图 | 普通栅格/置入、完整根组/链接蒙版及连续根多选；组内完整普通目标复用移动的可消去祖先分割；普通Normal根层选区可将选中部分和余像素静态捕获；祖先蒙版/低opacity、选区目标自身蒙版/图层opacity或fill非1、非连续目标、复杂栈与软件仍回退；最终取样与历史在CPU |
| 画布缩放/平移 | Qt 复用纹理与显示几何，平滑缩放/惯性/周边预取 | 动画与速度计算、普通栅格视口合成仍为 CPU |
| 笔尖自适应轮廓 | Qt fragment shader 按最终背景亮度选择黑/白，局部 GPU 离屏纹理 | 软件后端黑白轮廓；不改变实际绘画像素 |
| 选区蚂蚁线 | Qt 常驻文档遮罩 + fragment shader，动画与导航只改参数 | CPU 选区发现/布尔几何；宽高超过 4096 或软件后端走向量显示 |
| 整笔描边（主体印章复刻） | Qt 显示核心生成的像素结果 | 两尺度复刻/四通道max/差集/纹理及最终着色均CPU；没有wgpu描边shader |
| 普通画笔、湿混、涂抹、油漆桶、选区发现、通用图层合成 | 当前不宣称已有 wgpu 计算加速 | 仍使用既有 CPU 路径；Qt 显示其结果不等于算法 GPU 化 |

液化会话始终保留不可变源和弹性场，画布外只是裁剪。预览资产最长边 2048、网格最多 257×257；完成才全精度物化。详细适用边界见 [常驻预览](../tools/liquify-resident-preview-design.md) 与 [GPU 算子](../tools/liquify-gpu-operators-design.md)。

图层移动的会话、ABI 1.63、输入合并和代码入口见[实时移动设计](../tools/layer-move-preview-design.md)，检查范围见[验收](../validations/tools/layer-move-preview-validation.md)。移动没有新增 wgpu 算子：位移为 CPU 整数外观状态，GPU 只做纹理呈现。

自由变换和网格变形见[常驻预览设计](../tools/transform-resident-preview-design.md)。它复用原图资产与呈现网格，但使用独立控制点曲面，液化继续使用Elastic场。Qt通过正向源层与完整画布合成保留透明上下层；无需wgpu控制点算子，GPU显示不受计算设备选择CPU影响。Warp确认与预览共用正向网格覆盖，CPU按瓦片定位源UV；GPU源层在有序透明通道覆盖写入，折叠处不重复叠加图层透明度。

套索、魔棒和油漆桶的性能路径见[选区与填充优化](../tools/selection-complexity-performance-design.md)：CPU 共用扫描线区域算子、按行取样和瓦片写入；纯选区修改共享已有像素帧，Qt 保留相同图像的纹理身份。轮廓使用线性边界追踪和几何缓存。后续[常驻选区蚂蚁线](../tools/selection-resident-ants-design.md)增加 Qt 显示 shader：CPU 只在选择变化时栅格化文档遮罩，导航和相位复用纹理；真实 D3D11 与软件边界见[验收](../validations/tools/selection-resident-ants-validation.md)。未增加 wgpu 选色计算。

## 新功能如何接入

1. 在 `paint-ops` 定义可复用算子和 CPU 实现，明确 ROI、输入输出与预算。
2. 在 `paint-gpu` 增加对应 shader 和执行分派，复用设备与缓冲区，避免每次操作初始化设备或全图读回。
3. 从文档工具调用统一执行器，处理 GPU 失败并保留 CPU 行为；不得让后端选择改变历史、撤销、持久化或裁剪外恢复语义。
4. 如果输出只用于预览，优先发布有界网格/场和不可变源身份，让 Qt 复用纹理。需扩展 C ABI 时先定义版本/能力与容量协议，再生成头文件。
5. 用一个小 CPU/GPU 对照与对应 Qt 交互回归验证真实路径，记录硬件、输入大小和未覆盖边界。GPU 占用率不能单独代表延迟或效率。

## 代码示例

### 复用同一个算子

下面使用实际类型构造一个 3×3 弹性场。执行器应在文档/会话生命周期内创建并复用，不能为每个鼠标事件重新创建。`Elastic` 的返回向量存储在 RG 通道，`Remap`/`RemapBlocks` 返回颜色或约定的恒等标记；上层按算子契约采用，不能一律当最终 RGBA。

```rust
use paint_ops::{Cpu, ElasticOp, Executor, Grid, Operator};

let op = Operator::Elastic(ElasticOp {
    grid: Grid {
        origin: [0, 0], width: 3, height: 3, cell: 16.0,
        vectors: vec![[0.0, 0.0]; 9],
    },
    start: [0, 0], size: [3, 3], center: [16.0, 16.0],
    delta: [2.0, 0.0], radius: 24.0, strength: 0.3,
    density: 0.5, time: 0.016, speed: 1.0, tool: 0,
    selection: vec![],
});
let cpu = Cpu.execute(&op)?;
let accelerated = paint_gpu::Accelerated::new("auto");
let result = accelerated.execute(&op)?; // GPU 失败时运行同一请求的 CPU 实现
let backend = accelerated.name();       // 实际 GPU 或 CPU 回退原因
```

实际接线：`Document::set_compute_executor(Arc<dyn Executor>)`，位于 `core/crates/paint-core/src/document/tools/liquify.rs`；`liquify_operators.rs` 负责受影响节点范围、源瓦片预热、结果与预算校验。`paint-gpu/src/lib.rs` 的 `Accelerated` 持有设备与失败状态，计算 shader 位于 `paint-gpu/src/operators.wgsl`。

### 复用显示纹理

```cpp
// GUI 线程：改变目标视图，动画期间每帧只改变坐标/比例。
canvas->zoomBy(1.25); // 中心锚定、约 140ms；连续输入累积目标倍率
canvas->actualSize(); // 精确回到 100%，同时停止动画/惯性

// PaintCoreClient 内部复用 view 1；不会覆写已有完整概览请求。
client->ensureOverview();
```

`CanvasItem::requestView()` 提交带 128 屏幕像素边距及最多 360 屏幕像素方向前瞻的 ROI。`updatePaintNode()` 用缓存键决定是否上传图像，平移/缩放仅更新 QSG 节点矩形。四块概览条带避开高清视口，透明图层不会被重复 over 合成。每个视口仍受 4096 边长/4M 像素限制；概览默认最长边 512，已有导航器的整图请求则复用其帧。

### 接入自适应轮廓

```qml
AdaptiveCursor {
    backgroundItem: canvasBackground // 最终画布显示，不包含本轮廓输出
    outlineItem: outlineSource       // 现有圆形/图片 SVG 几何，仅使用 alpha
    captureRect: cursorBounds        // 与两个 sourceItem 同一画布局部坐标系
    active: canvas.gpuDisplay && cursorVisible
}
```

`CanvasPane.qml` 计算主副笔尖旋转后的包围区域并裁到窗口；`AdaptiveCursor.qml` 用两个 `ShaderEffectSource` 生成局部背景/轮廓纹理，复用现有 Shape，不做 CPU 截图或 GPU 读回。`adaptive-cursor.frag` 计算 `Y=0.2126R+0.7152G+0.0722B`，`Y<0.5` 选白，否则选黑，邻近 alpha 构成相反色细边。捕获范围只依赖可见轮廓、窗口与当前显示内容，不依赖文档原始分辨率；巨大轮廓可能覆盖整个窗口，但不会分配整张高分辨率文档纹理。隐藏时关闭 sourceItem 与 live 更新。

软件后端使用现有 Shape 的黑白线条，颜色/亮度旧偏好仍可读取以兼容原配置，但不再决定轮廓颜色，界面已移除亮度与自定义轮廓颜色控件。绘画预设、VBR、ABI 与文档文件格式没有变化。

副笔尖只在既有工具语义允许显示时进入共用 alpha 源；当前灰度双笔刷裁到主覆盖，不启用已经退役的双色溢出轮廓。不能将共用 shader 的代码支持写成副笔尖溢出功能已经重新实现。

## 使用与验证

设置 → 性能 → GPU 加速硬件：默认自动选择，通常优先独显，其次集显；下拉框可选择明确设备或 CPU，保存后看实际后端状态和回退原因。此项控制 Rust 算子；Qt 显示按应用图形后端工作，自适应轮廓不需要新增开关。

V 移动工具或 Ctrl 拾取拖动可实时预览，松手提交一条历史，Esc 取消。

缩放支持滚轮、Ctrl 加/减；中键或空格拖动后松手产生惯性，再次按下立即停止。100% 与适合窗口直接精确定位。轮廓随背景变化，无需手调亮度。

最小测试入口为 `qt_ui_tests canvasNavigationAnimatesAnchorsAndInertiaWithoutHistory adaptiveCursorContrastsBothSidesAndTracksLiquifyPreview cursorLegacyBrightnessIsIgnoredByAdaptiveDisplay`；真实 shader 像素用 `QT_QUICK_BACKEND=rhi`、`QSG_RHI_BACKEND=d3d11`，软件回退用 `QT_QUICK_BACKEND=software`。液化沿用 `liquifyResidentMeshRendersAndCommitsWithoutCpuFrames` 与既有软件用例。后端测试环境变量只作用于测试进程，不能据此改用户的全局图形设置。

构建通过 `ui/CMakeLists.txt` 要求 Qt ShaderTools，缺失时先按项目 `tools/setup-qt.ps1` 的模块安装流程准备依赖，再运行 CMake Release 构建。不得把单机 Qt/编译器绝对路径写入仓库配置，或为适应测试而改用户全局 GPU/暂存设置。

当前平台验收、构建路径与尚待实机检查的连续手感见 [导航与轮廓验收](../validations/interface/canvas-navigation-adaptive-cursor-validation.md)。

## 笔刷的 GPU 范围与复刻描边用法

截至本轮，普通画笔、图片笔尖、纹理、双笔刷、湿混、涂抹、橡皮以及整笔描边的像素计算均在 CPU。Qt 在硬件后端呈现这些像素，并为笔尖轮廓提供 GPU 背景亮度自适应；这不等于主笔刷在 GPU 上盖章。笔刷面板里的真实笔触预览也由同一 CPU 核心生成，QML 只显示返回图像。

整笔描边现在复刻主体已经确定的印章中心、压力尺寸、方向/随机形状及尾部回放，按大/小两尺度累计四通道max，最后做几何差集并着色。允许溢出只影响独立来源的主体包络裁剪，仍受画布、选区、锁透明和文档配额约束。Qt不再提供建路径/SDF算法选项，也不调度独立描边路径的间距/散布；旧设置读取后迁移为新模型。

```rust
use paint_core::{OutlinePlacement, OutlineSource, OutlineWidth, StrokeOutline};
let outline = StrokeOutline::new(
    true, OutlinePlacement::Outside, OutlineWidth::new(false, 6.0, 10.0)?,
    OutlineSource::SelfBrush, true,
)?.with_allow_overflow(false);
// 放入 BrushDefinition.outline 后，实际笔触与笔刷预览共用 CPU 执行入口。
```

实际采样见 `core/crates/paint-core/src/document/stroke/outline_replay.rs`，主体解析后的覆盖闭包复用见 `document/mod.rs`，覆盖发布、选区/图层约束与单笔历史见 `document_outline.rs`；纯算子为 `paint-ops/src/replay_outline.rs`。64×64四通道瓦片由现有尾部日志保存/恢复；一次区域/整笔边界8Mi像素限制，失败回滚整笔。详见[复刻描边设计](../brush/core/brush-stroke-replay-outline-design.md)。旧Path/SDF描边执行器、缓存和算法选择类型已删除；旧模型名仅在IO读取校验处保留，绑定核心时统一迁移。paint-ops中的通用SDF数值算子不属于描边执行器，独立保留。

GPU计算设置仍控制现有wgpu算子，选择GPU不会把描边自动转为GPU计算。将来加速须复用不可变采样与四通道覆盖身份、相同差集语义及有界范围，比较上传/回读在内的总成本，不能将CPU回调包装为GPU后端或为每次预览全图往返。

双重笔刷的已有来源纹理已通过 SecondaryTip 的可选 BrushTexture 接入（state44/behavior36、ABI1.72）：主/第二笔尖共用 CPU TextureDab 采样和资源/mip缓存，预览与实际笔触一致；此项没有新增GPU计算，Qt仍负责硬件显示。来源、覆盖组合和交换说明见[本轮清单](../planning/2026-10-20-bugfix-design.md)。


2026-10-20扩展：根层链接自身蒙版、完整根组、连续根多选的自由变换已接常驻代理，CPU只更新几何，Qt硬件显示；确认仍在CPU对原始分层/蒙版分别取样。复合预览存在预合成/代理近似，固定蒙版、组内单层与选区等保留CPU预览，见[变换设计](../tools/transform-resident-preview-design.md)。三类完成任务的真实进度和延迟显示见[长任务设计](../tools/long-task-progress-design.md)，不新增GPU算子。

2026-10-21移动扩展：无自身蒙版、完全不透明的普通祖先组可在私有显示快照中消去，使组内普通层继续使用三张常驻纹理和四顶点；固定蒙版、半透明/蒙版祖先等复杂组合使用最长边512的[有界CPU预览](../tools/layer-move-bounded-fallback-design.md)，Qt负责显示。最终显示恢复原完整路径，无新增wgpu计算请求。
