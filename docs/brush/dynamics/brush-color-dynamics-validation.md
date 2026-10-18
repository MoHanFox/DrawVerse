# 颜色动态内核验收

2026-10-14。设计见 [颜色动态](brush-color-dynamics-design.md)。不可变参数、两色与HSV随机通道、整笔/逐实际印章频率、真实颜色写入与分页贡献账本已实施；旧关闭路径保持。背景色为本笔工具字段，核心调用者可指定；ABI仍1.40，旧原生请求暂用白色背景，尚未接用户背景色与界面。状态/VBR明确拒绝非默认新模块，避免丢字段。

实际验证：

- `paint-core --test brush_color_dynamics`：9项通过。真实多色覆盖（不是alpha差异冒充颜色），25%整笔上限；分段一致、精确撤销重做；整笔固定与逐印章、三个HSV通道改变像素、关闭/零配置旧像素；原地不累积、更高压力新增贡献、低压不变浅、取消；图片孔洞及选区擦除/蒙版、手指载色和真实搬运；1瓦片驻留分页恢复、部分前景alpha不突破对应上限、透明锁、预算拒绝重试与参考像素一致。
- `paint-core --lib brush_color_dynamics`：1项通过。参数非法浮点/范围拒绝，1000采样的预乘范围和前景alpha保持，整笔固定序列及下一笔变化，透明前景/背景与两色混合边界。
- 核心相关回归：`brush_opacity` 7、`brush_shape_jitter` 9、`brush_scatter` 7、`brush_mask` 5、`smudge` 3项，全部通过。
- IO：`brush_color_io` 1项通过，启用/关闭的新配置不能被旧state/pressure序列化丢弃且原目录保持；`brush_shape_exchange` 5、`brush_pressure` 6项通过。
- `cargo check --manifest-path core/Cargo.toml -p paint-ffi --all-targets --locked`、三包all-targets clippy（`-D warnings`）及相关Rust格式化通过。新增Brush背景字段是Rust内部模型，旧C ABI请求默认白色，无DTO或头文件布局变更。

未跑完整 `tools/check.py`，未做Qt构建、实体设备/跨平台/专项性能。保持前景alpha，透明背景回退前景；HSV在线性RGB执行，未宣称PS感知HSV兼容，后续ABR转换需报告语义差异。保存/VBR/原生工具背景/Qt参数、ABR动态映射仍待接入；纹理/双笔刷/平滑收尖/湿混和完整格式冻结继续后续。本阶段不部署尚无入口的用户功能，不代表完整系统验收。
