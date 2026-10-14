# 形状抖动IO验收

2026-10-14。state9/VBR1.7/behavior6/shape-jitter的Rust IO已实现，ABI仍1.39。设计见 [保存交换](brush-shape-jitter-io-design.md)。独立对象与压力/设备/散布组合，完整十六进制u64种子、关闭值和默认旧状态保持；范围在f64输入层验证后转换，防止略越界数值被f32舍入接受。

本阶段实际检查：

- `paint-io --test brush_shape_exchange`：5项通过。完整四模块组合/state9与VBR1.7还原后的连续两笔逐像素一致，完整seed、关闭后真实像素和参数保持；无散布/无压力的单独形状模块可执行；旧默认状态规范化原字节不变。坏字段/null/种子/类型/范围、略越界与版本/能力不匹配拒绝。Schema的数值边界/seed/冻结来源基线角色有断言；未运行第三方JSON Schema验证器。
- `paint-io --test brush_shape_jitter_io`：1项通过。state9保存启用和关闭配置；错误pressure作用域仍拒绝，VBR旧对象不能隐藏非压力模块。
- IO相关回归：`brush_device_io` 4、`brush_scatter_io` 4、`brush_state` 4、`vbr_source` 6、`vbr_image` 4、`vbr` 4，全部通过。来源图片组合增加形状抖动，真实非空覆盖逐像素还原，当前资源/来源身份与完整原ABR字节保持；converter1基线拒绝新对象，旧资源包仍用最低版本。
- `paint-ffi --test brush_source`：6项通过。当前package入口临时拒绝shapeJitter时required=0且caller字节、现有目标不变；既有压力/设备/散布/来源路径保持。临时检查在import_bundle资源持久化之前。
- `cargo check --manifest-path core/Cargo.toml -p paint-ffi --all-targets --locked`、三包all-targets clippy（`-D warnings`）和相关Rust格式化通过。

未跑完整 `tools/check.py`，未做Qt构建、硬件/跨平台/本阶段性能测试。本阶段不部署界面新功能；原生能力和Qt入口下一阶段接入后解除包限制。保存基础seed，不保存运行时accepted随机笔触序号；不承诺重开后接续旧随机序列。ABR动态映射、随机颜色、纹理/双笔刷、平滑收尖、湿混及完整格式冻结仍未完成。
