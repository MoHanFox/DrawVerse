# 非完全覆盖图片笔尖性能验收

2026-10-18，Windows CPU。对应[设计](../../../brush/performance/brush-image-tip-performance-design.md)。

- `cargo run --manifest-path core/Cargo.toml -p paint-core --example brush_image_tip_performance --release --locked` 三次摘要一致，RGBA摘要 `2366978002773`，中位数约3.34秒。
- 同机程序笔尖对照约2.37秒，说明剩余主要成本在软边图片覆盖采样，而非历史/调度。
- 同一输入下图片笔尖的真实像素、孔洞、选区/蒙版和历史回归沿用既有 `brush_texture`、`brush_dual` 与 `paint-io` 用例。
- Windows Release、ABI 和严格 Clippy 通过。

图片笔尖仍慢于程序笔尖；未做 Photoshop 逐像素对照、设备延迟、跨平台或 GPU 验收。
