# ABR 16位采样实现与字节序证据

2026-10-15，先检查实际来源后决定实现，当前仍只执行8位ABR采样。

## 字节序与量化的证据缺口

- [ag-psd abr.ts](https://github.com/Agamnentzar/ag-psd/blob/master/src/abr.ts)的16位raw分支使用readUint16后取高8位；该读取器按大端解释。16位RLE另经其PSD行解码器，不能仅从raw分支推断所有压缩变体。
- [GIMP gimpbrush-load.c](https://github.com/GNOME/gimp/blob/master/app/core/gimpbrush-load.c)的现代raw双字节采样分支却按小端、全u16范围归一到8位，并拒绝压缩16位。该差异是实际源码观察，不足以证明哪方符合所有Photoshop生产文件。
- [Krita kis_abr_brush_collection.cpp](https://invent.kde.org/graphics/krita/-/blob/master/libs/brush/kis_abr_brush_collection.cpp)按depth计算原缓冲长度，其展示路径读取单字节；不是可作为16位正确灰度参考的第三个独立依据。

现有ag-psd simple/tilt无采样，special一个8位RLE，sample-and-pattern一个8位RLE；未找到该上游测试目录的16位资产。新增可分发Myer 148个和两个Photoshop探针也均为8位RLE。相关文件的身份/许可见core/crates/paint-io/tests/fixtures/abr/NOTICE.md和abr-real-corpus-validation.md；ag-psd资产仍仅在ignored artifacts中外部诊断，不分发。

2026-10-18 实现采用 ABR 容器的大端 u16 解释，按全范围四舍五入量化到现有 8 位 `BrushMask`（`(value * 255 + 32767) / 65535`）。raw 与逐行 RLE 均支持，RLE 行长度按字节计数；1/2 旧封套和 6/10 `samp` 分节共用同一解码器。已知 0/65535、不对称高低字节、纹理与笔尖 raw/RLE 和孔洞覆盖有独立夹具验证；Photoshop 全范围/内部取值范围及所有生产文件仍需更多真实语料核对，量化明确为近似并保留原始 ABR。

已有 BrushMask 执行资源为 8 位，16 位源量化必须报告近似并保留完整原始 ABR；不能把无损原文件保存等同 16 位绘画精度。恢复与交换保留原记录/基准身份。本阶段已完成 raw/RLE 8/16 位解码和分节/旧封套接入，但 Photoshop 全范围与生产文件一致性仍需更多真实语料。
