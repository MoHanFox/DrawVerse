# 复杂移动有界交互采样验收

2026-10-21，Windows，同工作区Release；[设计](../../tools/layer-move-bounded-fallback-design.md)。

## 核心与原生

- `cargo test --manifest-path core/Cargo.toml -p paint-render --locked`：23项通过，含新增2个单元/2个集成。区段工作量上限、负视口、Normal/Multiply/PassThrough组、固定/祖先蒙版、偏移、精确线性参考、sRGB误差≤1、取消/非法请求、最终像素和历史均覆盖。
- `paint-ffi`的`complex_move_coarse_frames_restore_full_resolution_on_commit_and_cancel`：通过。两个视口在固定蒙版移动时从1024降为512，松手/取消恢复1024，最终偏移及历史保持。
- 既有`layer_move_resident_packets_absolute_updates_and_final_barrier`通过；常驻源和最终发布屏障不变。
- Clippy（paint-ffi lib含依赖，warnings deny）与workspace格式检查通过。

## 窗口与构建

- `qt_transform_tests::compoundLayerMovesKeepResidentSources`七行在软件Qt与真实D3D11各通过：链接自身蒙版、带蒙版根组、连续多选、两级Normal/PassThrough组内子层、固定自身蒙版、祖先蒙版。真实鼠标32次移动、松手前显示、一次历史、确认和撤销；前五类硬件常驻源身份与四顶点保持，后两类确认使用回退。
- 完整Release构建（主程序、三类Qt测试、四套ABI）通过；cbindgen头一致。四套C11/C++20静态/动态ABI依次运行通过。
- 产物：`build/qt/bin/Release/drawverse.exe`。Qt测试使用96px画布验证功能，4K性能只依据下面的CPU基准，不混为实体4K交互验收。

## 固定性能样本

`cargo run --manifest-path core/Cargo.toml -p paint-render --release --example move_preview_bench --locked -- 3840 2160`；对应2000×2000再跑一次。完整彩色层、固定自身蒙版、70%祖先组、白底；5次绝对位移，各自中位数。旧CachedCpuRenderer与新有界渲染对同一快照比较，输出请求最长边1024。

| 文档 | 完整预览 | 有界预览 |
| --- | ---: | ---: |
| 3840×2160 | 394.346ms | 42.927ms |
| 2000×2000 | 252.732ms | 48.740ms |

仅CPU画布渲染，不含输入、排队、Qt显示或结束后的最终帧；不是实体连续输入时延。预览最长边512、最近邻及色阶近似；任意深组/混合的合成成本仍与层数有关，首次资产、冷页IO和最终完整渲染仍可能耗时。未跑全量、跨平台或实体持续手感验收。ABI1.74/state44保持，无新增GPU计算。
