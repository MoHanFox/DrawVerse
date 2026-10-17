# 纹理混合模式

纹理模式作用于笔尖覆盖率与纹理采样值的组合，不直接改写最终画布颜色。`normal` 保持旧版周期覆盖行为；其余模式使用同一线性覆盖值，便于与笔尖、流量、选区、擦除和湿混共用已有执行路径。

支持模式：`normal`、`multiply`、`darken`、`lighten`、`subtract`、`difference`、`screen`、`colorBurn`、`linearBurn`、`colorDodge`、`linearDodge`、`overlay`、`softLight`、`hardLight`、`exclusion`、`hardMix`、`vividLight`、`linearLight`、`pinLight`、`divide`。旧 state/VBR 纹理缺少 mode 时规范化为 `normal`。`colorBurn` 要求 VBR1.25；前八种新模式要求 state30/VBR1.36，最后四种要求 state31/VBR1.37。ABR `BlnM` 的 `linearBurn`、`CDdg`、`linearDodge`、`Ovrl`、`SftL`、`HrdL`、`Xclu`、`hardMix`、`vividLight`、`linearLight`、`pinLight`、`blendDivide` 已映射。

`brightness`/`contrast` 在同一采样值上先做亮度平移、再做围绕 0.5 的线性对比度调整，然后应用反相和深度。默认 0 保持旧字节；非默认值要求 state21/VBR1.26。

`minimumDepth`、`depthJitter` 和 `depthPressure` 在印章级计算有效深度：笔压从最低深度到配置深度插值，深度抖动使用印章坐标派生的确定性随机量并以最低深度封底。默认 0/false 保持旧字节，非默认值要求 state21/VBR1.27。

模式组合在核心执行期完成，纹理采样仍保留周期坐标、LOD、深度、反相和文档/笔尖空间。模式不会把纹理当作最终画布混合模式，也不会改变纹理关闭时的参数保存规则。

新增公式使用覆盖率标量：线性加深 `base+texture-1`、颜色减淡 `base/(1-texture)`、线性减淡 `base+texture`、叠加/强光/柔光按 Photoshop/W3C 标量公式、排除 `base+texture-2*base*texture`、实色混合 `base+texture>=1 ? 1 : 0`、鲜明光按纹理半区切换颜色加深/减淡、线性光 `base+2*texture-1`、点光按半区取 min/max、划分 `base/texture`。这些是覆盖率近似，不等于最终 RGB 图层混合；透明、纸纹和深度仍由原采样路径处理。
