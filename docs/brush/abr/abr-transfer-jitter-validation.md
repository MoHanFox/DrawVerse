# ABR 传递通道随机抖动验收

2026-10-18，Windows。

- `paint-core --test brush_transfer_jitter` 通过：三通道有界、同笔触/序号确定性、不同笔触变化、开启与关闭像素不同，整笔不透明度上限保持。
- `paint-io --test abr_import` 通过：ABR `prVr`/`opVr` 的 50%/25% 映射到 flow/opacity，`szVr.jitter` 保持形状动态通道且不重复进入传递；raw/RLE 16位笔尖与纹理回归同时通过。
- `paint-io --test brush_state` 5项、`vbr` 4项、`vbr_image` 12项通过；旧 state/VBR 无对象时保持旧行为。
- Windows Qt 最小 `transferJitterStateRoundtripKeepsThreeChannels` 将 state37 三通道恢复为 `.6/.5/.25`；传递页提供大小/流量/不透明度三个抖动滑条，QML 加载与既有组/纹理/湿混/双色移除回归同时通过。
- Release 构建与 C/C++ ABI 链接通过。

未运行全量 `tools/check.py`，未验证 Photoshop 随机序列、实体设备、跨平台或 GPU。
