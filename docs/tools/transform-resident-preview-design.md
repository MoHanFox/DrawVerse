# 自由变换与网格变形的常驻预览

2026-10-19。目标是解决拖动时画面滞后：交互只计算几何，确认才重采样原始像素；CPU与GPU职责分开，软件及复杂组合保留原有正确路径。

参考 PhotoCraft `d95fdf3181c01ad41e95456091a394b105a9afb2` 的 `ui-egui/src/transform_tool.rs`、`transform_tex.rs`、`warp_preview.rs`：会话保留源纹理，控制点改变有界细分曲面，最终算法独立执行。DrawVerse 保留自己的4×4插值Catmull曲面与投影变换，不把液化Elastic位移场冒充控制点变形。

改造前 `Document::update_transform` 每次遍历目标像素；warp逐像素Newton求逆，代价随输出面积增长。Actor虽已合并相邻绝对更新，执行中的整图操作仍阻塞后续输入。

## 实施与边界

- 显式deferred开始入口；兼容单一普通Normal栅格/置入图片栈捕获原图、上下层，拖动只更改控制点及CPU正向细分网格。复用液化/移动的静态资产协议，不回读GPU预览。
- CPU使用现有 `TransformGeometry::map`，网格包含控制点分界，按曲面跨度有界细分。warp与液化仅共享呈现架构，算法仍独立。最终确认调用既有全精度CPU取样，保持六种插值、原图、单历史、撤销/取消和资源失败语义。
- Qt以正向网格绘制原图到有界视口纹理，再在线性颜色空间与静态上下层纹理合成。透明像素、变形外区域和裁剪都要正确，不能只在变形三角形内显示上下层，也不能重复混合透明背景。静态纹理不随控制点更新上传。
- 更新只合并相邻绝对几何；checkpoint/undo/redo/commit/cancel必须保留顺序，确认采用最后一次已提交几何。预览保留至最终像素帧接管。
- 有选区、组/多选、蒙版/裁剪/特殊混合和软件显示保留像素路径。本轮不将通用图层合成器迁移至GPU，不启用wgpu计算来处理少量控制点。

## 验收条件

核心比较deferred与既有路径的最终像素/历史；检查拖动原始瓦片身份与源资产不变、控制点真实映射、取消/局部历史/画布外恢复/置入模型。原生覆盖异步包、容量和最终几何；Qt检查确认前真实画面、透明上下层合成、软件回退、撤销与取消。记录固定Release计算耗时，不以自动化等待耗时代替鼠标手感。完成Windows Release、最小回归及四ABI检查。

## 代码与调用

- `core/crates/paint-core/src/document/transform.rs`：继续定义几何、原图、局部历史和最终逆向取样；新增状态只在deferred更新时绕过像素生成，确认临时退出deferred后调用同一个旧取样器，失败保留会话可调整或取消。
- `transform_preview.rs`：`begin_transform_layers_deferred(ids)` 与 `enable_transform_deferred()`。后者接入置入图片自动打开的会话。`transform_preview()` 通过现有 `TransformGeometry::map(u,v)` 发布正向位置和源UV；包含每个1/3控制节点，轴向至多145点，共21025顶点。跨度细分受预算约束，不随原图像素数量线性增加。
- `tools/liquify_preview.rs::resident_source_layer`：与液化/移动共用静态源切分；置入图片采用保留的原始模型像素，不使用上一次变换生成的缓存作为下一次原图。三工具共用 `ResidentPreview`，正向/逆向语义显式区分。
- ABI1.64：新 `paint_session_begin_transform_layers_deferred`；已有会话通过 `paint_session_transform(...,PAINT_TRANSFORM_ENABLE_RESIDENT,null,...)` 请求接入。`focus` bit0仍为隔离源，bit1表示正向网格。旧入口默认像素路径，旧液化/移动包不改语义。
- `PaintCoreClient.cpp`：按实际窗口GPU显示能力选择入口，置入会话延迟到文件结果交付后接入；Actor继续合并相邻绝对更新，checkpoint和关闭不可越过；0/1视口在deferred期间停止CPU重采样。GUI保留末次预览直至最终帧接管。
- `LiquifyPreviewNode.cpp`：原图正向网格只绘制源，未使用的上下层采样器绑定1×1透明纹理；实际静态上下层由 `CanvasItem::previewPart` 共用图像节点提供。图像身份不变时不重新上传或主动重绘这些静态离屏层。
- `CanvasPane.qml` / `transform-composite.frag`：原图网格捕获至Qt layer，纹理尺寸每轴至多2048；静态上下层同坐标捕获，最终fragment在线性预乘空间合成后编码sRGB。画布外片元裁剪。Qt拥有纹理与生命周期，没有跨Rust共享GPU对象或GPU读回。

在硬件Qt窗口中照常 Ctrl+T 或置入图片，兼容目标自动使用该预览；不需要启用wgpu计算。性能设置选择CPU只控制wgpu算子，变换控制点本来就在CPU上，Qt硬件显示继续工作。软件场景图、有选区或复杂栈自动保留原路径。

PhotoCraft采用Bezier曲面与预览纹理/mip缓存；本实现保留DrawVerse插值Catmull节点、既有六种最终插值与Newton逆向最终取样，未移植其曲面/折叠顺序或整个GPU合成器。预览是有界细分、RGBA8与2048静态代理，极端透视、放大细节和自交折叠可能与最终精确像素有差别；不声称与Photoshop或PhotoCraft逐像素等价。首次代理准备与最终重采样仍有后台开销。

实测见[验收](../validations/tools/transform-resident-preview-validation.md)。
