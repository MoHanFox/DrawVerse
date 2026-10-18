# 整条笔触描边分阶段验收

对应[设计](../../../brush/core/brush-stroke-outline-design.md)，2026-10-19 Windows。

## 当前范围

首阶段新增独立描边配置和有界覆盖距离带；尚未接Document绘画、保存交换、原生准备或Qt设置。整项反馈保持未完成。

`cargo test -p paint-core --test brush_stroke_outline --locked`四项通过：540组距离带与独立穷举参考逐像素一致（五类尺寸、六类输入、三种位置、六种宽度），重叠无内接缝／完整带宽／亚像素覆盖，关闭值与宽度切换，空覆盖／50%阈值／超限和非法值。输入覆盖保持不变。

`cargo clippy -p paint-core --lib --test brush_stroke_outline --locked -- -D warnings`通过；首轮发现一处可替换为枚举迭代的循环，修正后复测。格式及diff检查通过。该阶段不改变ABI、state或VBR；未跑全量、设备、跨平台或GPU。
