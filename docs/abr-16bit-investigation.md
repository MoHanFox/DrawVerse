# ABR 16位采样调查（尚未实施）

2026-10-15，先检查实际来源后决定实现，当前仍只执行8位ABR采样。

## 字节序与量化的证据缺口

- [ag-psd abr.ts](https://github.com/Agamnentzar/ag-psd/blob/master/src/abr.ts)的16位raw分支使用readUint16后取高8位；该读取器按大端解释。16位RLE另经其PSD行解码器，不能仅从raw分支推断所有压缩变体。
- [GIMP gimpbrush-load.c](https://github.com/GNOME/gimp/blob/master/app/core/gimpbrush-load.c)的现代raw双字节采样分支却按小端、全u16范围归一到8位，并拒绝压缩16位。该差异是实际源码观察，不足以证明哪方符合所有Photoshop生产文件。
- [Krita kis_abr_brush_collection.cpp](https://invent.kde.org/graphics/krita/-/blob/master/libs/brush/kis_abr_brush_collection.cpp)按depth计算原缓冲长度，其展示路径读取单字节；不是可作为16位正确灰度参考的第三个独立依据。

现有ag-psd simple/tilt无采样，special一个8位RLE，sample-and-pattern一个8位RLE；未找到该上游测试目录的16位资产。新增可分发Myer 148个和两个Photoshop探针也均为8位RLE。相关文件的身份/许可见core/crates/paint-io/tests/fixtures/abr/NOTICE.md和abr-real-corpus-validation.md；ag-psd资产仍仅在ignored artifacts中外部诊断，不分发。

不能从仅包含00/FF、相同高低字节或图像看起来平滑推定字节序。后续需要有明确生成方法/版本的16位Photoshop导出及参考覆盖，含不对称高低字节、渐变、透明/完全覆盖、非方形/孔洞，分别核对raw与RLE及6/10、1/2旧封套。全范围/PS内部取值范围、舍入与截高字节规则也需单独验证。

已有BrushMask执行资源为8位，16位源若量化必须报告近似并保留完整原始ABR；不能把无损原文件保存等同16位绘画精度。恢复与交换仍应保留原记录/基准身份。此调查不构成16位支持完成，也不是整个任务的阻塞：分节对齐的真实兼容错误已按另一个模块修复，其他笔刷能力可继续推进。
