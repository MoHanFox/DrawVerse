# 纹理混合模式验收

日期：2026-10-17，Windows 优先。

核心 `TextureMode` 已实现 `normal`、`multiply`、`darken`、`lighten`、`subtract`、`difference`、`screen`、`colorBurn` 八种覆盖组合，并支持亮度/对比度调整。`normal` 与旧版纹理覆盖保持一致；旧 state/VBR 缺少字段时由 IO 和 Qt 规范化，`colorBurn` 需要 VBR1.25，非零亮度/对比度需要 state21/VBR1.26，带纹理来源快照需要 VBR1.27。

已验证：

- 核心模式数值组合单元测试通过。
- 纹理在文档空间和笔尖空间的现有像素路径继续使用纹理资源、LOD、深度、反相和历史逻辑。
- 纹理 state/VBR 交换测试通过，包含真实资源绑定和历史像素恢复。
- Qt 纹理导入、设置、预览、绘画、导出/导入回归通过；纹理设置页新增模式下拉。
- Release Qt 构建通过。

范围说明：这些模式作用于笔尖覆盖率与纹理采样值，不直接替代最终画布颜色混合模式。ABR `Txtr` 参数与 `patt` 图案现已接入这八种子集，见 [ABR 纹理验收](abr-texture-validation.md)；其余未定义模式仍保留来源并报告未执行。
