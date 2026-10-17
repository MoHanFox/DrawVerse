# 大笔刷密集笔触性能验收

2026-10-18，Windows，对应[设计](brush-large-dense-performance-design.md)。本阶段增加完全覆盖像素跳过，覆盖普通画笔、硬橡皮、纹理、图片笔尖和灰度双笔刷；无ABI/state/VBR变化。

基准命令：`cargo run --manifest-path core/Cargo.toml -p paint-core --example brush_large_dense_performance --release --locked`。2048×2048画布，硬圆半径128，1%直径间距，240个8px输入，普通混合/整笔opacity1，3次中位数，含真实起笔、绘画、End及单历史，不含Qt视口。优化前中位数约1,333,746µs，优化后约365,547µs（另一次优化后运行约335,477µs）；最终RGBA摘要均为`2501345514932`，三次运行一致。该结果只代表此CPU固定输入，不宣称所有笔刷或实体设备达到同等倍率。

- `paint-core --test brush_opaque_repeat` 3项通过：同位置重复不透明画笔、橡皮和全不透明纹理印章均与单个印章像素/历史一致。
- `brush_texture` 7项、`brush_opacity` 7项、`document` 28项、`paging` 3项、`brush_dual` 9项及速度/切向/距离动态测试通过，覆盖纹理孔洞、图片、低不透明度、压力、蒙版、选区、分页、失败回滚和复杂笔刷路径。
- `cargo fmt --all -- --check`和严格Clippy通过；没有运行全量`tools/check.py`。
- 未改变随机、湿混、彩色双笔刷、涂抹、混合模式、起收笔、设备输入或GPU；纹理/图片和灰度双笔刷只跳过已达到完全覆盖的像素；实体数位板/Qt呈现延迟仍由用户验收。

证据：`artifacts/brush-large-dense-after.log`；优化前输出见本阶段命令记录。工作区不新增ABI/格式版本，跨平台未验收。
