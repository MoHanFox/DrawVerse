# 颜色动态IO验收

2026-10-14。state10/VBR1.8/behavior7/color-dynamics接Rust IO，ABI仍1.40，Qt尚未接。设计见 [保存交换](brush-color-io-design.md)。模型严格字符串linear-hsv-v1，范围在f64检查后转f32，完整seed及关闭参数保持。工具前景/背景不进入预设，测试显式供应相同两色。

实际检查：

- `paint-io --test brush_color_exchange`：5项通过。完整压力/设备/散布/形状/颜色组合state10及VBR1.8还原后连续两笔逐像素一致，完整种子、关闭后配置及真实像素保持；单独颜色配置和整笔/逐印章频率正确还原；旧默认状态原字节保持。模型未知/数字/对象、字段/null/类型/范围/略越界、版本/能力不匹配拒绝。Schema模型/seed/来源基线角色有回归断言，未运行第三方JSON Schema验证器。
- `paint-io --test brush_color_io`：1项通过。启用和关闭配置可保存state10，错误pressure作用域仍拒绝并保持原目录。
- IO相关回归：`brush_shape_exchange` 5、`brush_pressure` 6、`vbr_source` 6、`vbr_image` 4、`vbr` 4通过。来源图片测试新增实际颜色动态和两色工具输入，非空真实像素还原、当前资源/来源身份与完整原ABR字节保持；converter1基线拒绝新对象，旧资源包仍用最低版本。
- `paint-ffi --test brush_source`：7项通过。ABI1.40 package暂拒绝新对象时required=0且caller字节、已有目标保持；既有压力/设备/散布/形状/来源路径不变。检查在import_bundle持久化之前。
- `cargo check --manifest-path core/Cargo.toml -p paint-ffi --all-targets --locked`、三包all-targets clippy（`-D warnings`）和相关Rust格式化通过。最后模型字符串收紧后重跑颜色exchange及IO all-targets clippy通过。

未跑完整 `tools/check.py`，未做Qt构建、实体设备/跨平台/专项性能。本阶段不部署新界面。原生背景传入和Qt入口后续接入，完成后解除临时包边界。运行时随机笔触序号不入格式，不承诺重开接续原序列；线性HSV不宣称PS感知HSV兼容。ABR动态映射、纹理/双笔刷/平滑/湿混与完整格式冻结仍未完成。
