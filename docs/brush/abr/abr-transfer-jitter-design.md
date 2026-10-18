# ABR 传递通道随机抖动

2026-10-18。ABR `prVr`、`opVr` 的 `jitter` 百分比映射到新的 `TransferJitter` 流量和整笔不透明度通道；`szVr.jitter` 仍属于形状动态，不重复计入传递。`TransferJitter` 结构保留尺寸通道供原生配置使用。随机源使用 DrawVerse 文档的笔触序号、逻辑印章序号、子印章序号和通道 lane；不读取或伪造 Photoshop 随机种子。每笔随机序列确定、可复现，撤销重做恢复已生成像素而不回退运行时序号。

随机系数范围是 `1-jitter..1`，与现有形状抖动一致。尺寸因子在压力/设备几何和起收笔之前作用；流量乘入每印章沉积；不透明度乘入整笔上限，因此低透明度反复重叠仍遵守同一整笔上限。主笔尖和配对第二笔尖共享当前印章的尺寸因子，第二笔尖流量使用同一随机样本。

`TransferJitter` 是 `BrushDynamics` 的不可变字段，默认三通道为 0。ABR converter 版本13把三个 `jitter` 写入 converter 基线；handoff 版本12携带 `transferJitter`，Qt 接受并保存 state37/VBR1.44/behavior29/`transfer-jitter`。旧 state/VBR 不含该对象时保持旧字节和旧版本，含对象而降级明确拒绝。

限制：随机分布与 Photoshop 的序列未逐像素核对；本阶段不加入 ABR 种子字段，也不宣称所有设备/组合完全等价。
