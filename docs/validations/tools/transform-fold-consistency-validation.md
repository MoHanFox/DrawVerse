# Warp折叠预览与确认一致性验收

2026-10-19，Windows 11 / Qt 6.8.3 / RTX 4060 Laptop GPU，分支 `feat/wgpu-support`。设计更新见[常驻变换设计](../../tools/transform-resident-preview-design.md)。无ABI/文件格式变化，仍为ABI1.64。

## 问题与修复

用户两张截图显示强折叠预览和确认结果不同，确认后透明洞、断裂和换面。复现使用按截图控制点归一化的交叉行、反向单元及画布外节点，不声称还原用户原项目的全部像素。

原最终Warp用单初值Newton求逆，折叠时多根/不收敛不能确定覆盖。参考PhotoCraft `d95fdf3181c01ad41e95456091a394b105a9afb2` 的 `crates/algo/src/warp.rs`，采用前向网格/目标瓦片分桶/重心UV/后面覆盖前面。DrawVerse预览和物化共用 `transform_mesh.rs::TransformMesh` 的相同拓扑、坐标精度与三角顺序；六种颜色插值仍在原始源上执行，非Warp透视仍用精确逆矩阵。选区覆盖也使用同一Warp面。

Qt正向源层保持有序透明通道，通过 `QSGMaterialShader::updateGraphicsPipelineState` 使用One/Zero覆盖RGBA。不能直接关闭材质Blending：Qt不透明通道的深度写入会让先画的折叠面固定占据像素。也不能普通over多次叠加同一图层的半透明折叠面。上下层合成仍独立执行，液化/移动保留原材质状态。

输出预算16Mi像素与文档瓦片预算保持；新增最多524288目标桶、4194304三角-瓦片引用，超限在候选发布前报错；每次仅分配一个64×64 UV块，不存全图位移数组。拖动仍只发布有界网格，最多145×145顶点，不回到逐像素预览路径。

## 最小验证

- 核心 `cargo test -p paint-core --locked --lib document::transform`：5项通过。新强折叠用独立的暴力重心三角参考对发布网格逐像素比较，并要求至少100个多覆盖像素，检查确认/撤销/重做；另一项检查共享边、跨瓦片、负坐标、反向绕序和完全退化三角。旧deferred原图/置入/失败原子性3项保持通过。日志 `artifacts/transform-fold-core.txt`。
- `cargo test -p paint-core --locked --test transform`：7项通过。新增强折叠选区与实际内容alpha逐像素对应、完成撤销；原组/多选、插值、置入源、历史和选区6项通过。日志 `artifacts/transform-fold-regression.txt`。
- 原生 `cargo test -p paint-ffi --locked --test async_session transform`：2项通过，包含1000次更新、源身份、精确revision/容量、最新几何与历史。日志 `artifacts/transform-fold-ffi.txt`。
- 定向Clippy `cargo clippy -p paint-core -p paint-ffi --lib --locked -- -D warnings --emit=metadata` 通过；格式及生成头检查通过。日志 `artifacts/transform-fold-clippy.txt` 和构建日志。
- 四ABI和offscreen/software `qt_transform_tests` 的CTest共5项通过，软件测试含两个实际功能用例。日志 `artifacts/transform-fold-ctest.txt`。
- 真实D3D11 `qt_transform_tests` 两个功能用例通过：既有透明上下层/连续拖动/局部历史回归保持；新增96×96白底黑笔触强折叠，分别以1.0、0.5图层透明度比较确认前后全画布，永久撤销恢复。考虑1.5 DPR显示重采样，允许相邻1物理像素、RGB每通道16误差，双向无法匹配比例须小于2%；本次为302/20736（1.46%）、117/20736（0.56%）。这不代表GPU预览与最终像素严格相同；核心几何覆盖另有精确参考测试。日志 `artifacts/transform-fold-qt-hardware.txt`。

置入图片变换/栅格化和液化常驻显示的真实D3D11最小回归各1项通过，日志 `artifacts/transform-fold-qt-regression.txt`。

## 构建与边界

完整Windows Release通过，产物 `build/verify-ui/bin/Release-wgpu-transform/drawverse.exe`；`paint_ffi.dll` 已同步该目录。没有终止用户应用。并行编译最初因本机内存不足失败，设置子进程 `CARGO_BUILD_JOBS=1`、`CMAKE_BUILD_PARALLEL_LEVEL=1` 后完整构建通过；TEMP/TMP继续仅对子进程定向D盘，未改变系统设置。

同步 `QQuickWindow::grabWindow()` 在确认切换离屏层时曾停滞；仅终止本任务测试进程，测试改为根contentItem的有界异步 `grabToImage()`（包含layer.effect）后通过，未改产品渲染循环。比较前固定画布实际像素缩放，避免窗口初始布局/适配变化混入图像比较。

常驻资产仍为最长边2048的RGBA8代理，Warp是有界分片仿射曲面，最终从完整源重新插值，不保证等于连续Catmull曲面的任意精确逆。固定行序为明确的折叠遮挡规则，未移植PhotoCraft预览的位移排序。软件和复杂栈仍使用CPU像素预览，均受共享Warp定位修复。未重跑全量、跨平台、持续实体输入或长时间GPU占用验收；历史初版基准不作为本次新增性能测量。
