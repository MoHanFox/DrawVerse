# 散布IO验收

2026-10-14。state8 / VBR1.6 / behavior5 / scatter-stamps 已接Rust IO；ABI仍1.38，Qt入口未接。设计见 [散布保存](../../../brush/dynamics/brush-scatter-io-design.md)。

本阶段验证：

- `paint-io --test brush_scatter_io`：4项通过。state8及VBR1.6还原散布、压力与方向/倾斜配置，带不同压力的连续两笔全图像素一致；u64最大种子保持、关闭配置保持、旧状态原字节保持。坏类型/范围/null/未知字段/种子与版本能力不匹配拒绝。
- `paint-io --test brush_pressure --test brush_state --test vbr --test vbr_image --test vbr_source`：分别6/4/4/4/6项通过。压力旧对象仍拒绝错误作用域的散布；独立state对象可保存。含图片及原ABR来源的散布包保留来源身份、基线和全部原字节；converter1基线拒绝新动态。
- `paint-ffi --test brush_source`：5项通过。旧包入口拒绝新对象时required为0、caller字节及已有目标保持；既有来源/压力/设备包路径保持。
- `cargo clippy -p paint-core -p paint-io -p paint-ffi --all-targets --locked -- -D warnings` 通过；本轮相关Rust格式化通过。新Schema字段界限有回归断言，未运行第三方JSON Schema验证器。

未跑完整 `tools/check.py`，未做Qt构建、实体数位板及跨平台验证。本模块仅IO，因此不宣称散布设置界面已可用。下一阶段接原生能力与Qt后解除临时包边界。种子入格式，运行时笔触序号不保存；工程重开不承诺接续旧运行时序列。整个笔刷系统的其他动态、纹理、湿混、完整ABR映射和验收仍见总体计划。
