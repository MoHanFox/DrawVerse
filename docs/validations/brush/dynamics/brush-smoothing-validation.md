# 时间平滑内核验收

2026-10-14，设计见 [brush-smoothing-design.md](../../../brush/dynamics/brush-smoothing-design.md)，无ABI变化。不可变BrushSmoothing捕获到统一程序/笔触，4ms网格上的位置速度自适应滤波、独立压力滤波、像素拖绳稳定器和有界终点释放已接Rust Document。主/第二沿同一过滤路径执行；关闭或启用零通道保持直接路径。逆时/时间溢出/生成数量与累计印章预算预检在更新状态前执行；材料失败按原事务恢复。

相关最小验证：

```text
cargo test --manifest-path core/Cargo.toml -p paint-core --lib brush_smoothing --locked
cargo test --manifest-path core/Cargo.toml -p paint-core --test brush_program --test brush_mask --test brush_dual --test brush_dual_path --locked
cargo test --manifest-path core/Cargo.toml -p paint-io --test brush_smoothing_boundary --test brush_dual_io --test brush_dual_exchange --locked
cargo clippy --manifest-path core/Cargo.toml -p paint-core -p paint-io -p paint-ffi --all-targets --locked -- -D warnings
```

7/24/7项分别通过；固定时间轨迹线性拆分相同、独立压力/速度/稳定器与设备通道、等时直接回退、逆时/极长时间/预算原子拒绝、真实画笔/擦除/涂抹与显式过滤参考逐像素一致、选区/蒙版/图层偏移/分页/取消及真实单笔历史已覆盖。旧圆笔和图片/双笔尖相关回归通过。cargo fmt、clippy、git diff --check通过，Qt Release构建及生成头检查通过（本阶段没有新控件）。

旧state与来源基线对任何非默认平滑（包括关闭但保留不同值）明确拒绝，防止静默丢参数；旧导入默认配置兼容。本阶段未接状态/VBR/原生Qt入口，不能宣称用户已能调平滑；起收笔曲线及有界笔尾回算仍待后续。没有全量tools/check.py、真实数位板手感/跨平台/总体性能验收，完整系统持续推进。用户新增普通画笔混合模式为双色扩展前置，下一实现顺序按该要求调整。
