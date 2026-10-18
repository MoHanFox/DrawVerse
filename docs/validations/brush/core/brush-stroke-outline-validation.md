# 整条笔触描边分阶段验收

对应[设计](../../../brush/core/brush-stroke-outline-design.md)，2026-10-19 Windows。

## 当前范围

首阶段新增独立描边配置和有界覆盖距离带；第二阶段通过显式Rust入口接入干画笔的自身／纹理描边和真实Document历史；第三阶段接入已有笔刷快照和活动预览；保存交换与原生入口见[保存验收](brush-stroke-outline-io-validation.md)及[ABI验收](brush-stroke-outline-abi-validation.md)。Qt设置仍待接，整项反馈保持未完成。

`cargo test -p paint-core --test brush_stroke_outline --locked`四项通过：540组距离带与独立穷举参考逐像素一致（五类尺寸、六类输入、三种位置、六种宽度），重叠无内接缝／完整带宽／亚像素覆盖，关闭值与宽度切换，空覆盖／50%阈值／超限和非法值。输入覆盖保持不变。

`cargo clippy -p paint-core --lib --test brush_stroke_outline --locked -- -D warnings`通过；首轮发现一处可替换为枚举迭代的循环，修正后复测。格式及diff检查通过。该阶段不改变ABI、state或VBR；未跑全量、设备、跨平台或GPU。

## 第二阶段：Document像素和历史

独立几何位图按实际主／双笔刷、纹理、散布和起收笔尺寸的50%覆盖记录，不受目标图层颜色影响，也不把整笔不透明度当作轮廓阈值。额外采样计入原有输入访问预算；最大2048个几何瓦片。画布外保留足够几何余量，内描边不把画布边缘误判为笔触边界。几何随起收笔检查点一起回退和回放。

`cargo test -p paint-core --test document_stroke_outline --locked`七项通过：三位置×两宽度逐像素参考／单历史撤销重做，同色底图／取消，起收笔普通与分页一致，文档外轮廓，真实纹理文档／局部坐标，选区／图层偏移／透明度锁，新增瓦片及8Mi包围区域失败的整笔回滚。另一个内部单元验证起收笔几何与已知最终距离的一遍参考相同。首轮暴露内描边画布外余量不足，修复并复测通过。

`cargo clippy -p paint-core --lib --test document_stroke_outline --locked -- -D warnings`通过。现有起收笔与分页相关回归单独执行并记录于反馈总验收；没有ABI、state或VBR变化。显式入口目前要求干画笔普通像素层，尚未支持的湿混／旧彩色双笔刷／蒙版和已有笔刷来源在起笔前明确拒绝，避免静默变成普通描边。

## 第三阶段：已有笔刷与活动预览

已有笔刷在起笔时保存不可变参数快照，提取整笔覆盖的闭合轮廓，按来源间距沿轮廓排布实际图片笔尖，再叠加来源散布；外／中／内通过轮廓位置与最终带宽裁剪共同实现。来源选择、源笔刷后续修改或删除不改变在途笔触，也不建立预设递归引用。

活动笔触在下一次输入前移除上一版描边预览，再按完整覆盖重绘；分页输入未完成时不发布缺少描边的中间画面，分页完成后才恢复可发布状态。结束笔触沿用同一完整覆盖重绘，因此预览、最终像素和单次撤销一致。

`cargo test -p paint-core --test document_stroke_outline --test document_stroke_outline_existing --test document_stroke_outline_preview --locked`九项通过：既有Document七项保持通过；已有笔刷快照验证图片笔尖、间距、两轴散布及孔洞/分离轮廓，普通与分页结果一致；活动预览验证旧位置无残影、分页完成前禁止发布、结束像素一致及单次撤销。瓦片和包围区域资源失败现在可能在起笔或后续活动预览即发生，测试按首个失败验证整笔回滚且不留下活动历史。

`cargo clippy -p paint-core --lib --test document_stroke_outline --test document_stroke_outline_existing --test document_stroke_outline_preview --locked -- -D warnings`及`cargo fmt --all -- --check`通过。该阶段无ABI、state或VBR变化；未跑全量、设备、跨平台或GPU。
