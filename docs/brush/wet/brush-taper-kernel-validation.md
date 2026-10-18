# 起收笔曲线与笔尾调度验收

2026-10-16，Windows。设计见 [起收笔内核](brush-taper-kernel-design.md)。本次只验证独立Rust数值和命令调度；Document像素回算、单笔历史、设置保存、C ABI和Qt入口尚未实现，不能据此宣布画布起收笔功能完成。

实际命令（使用现有本地工具链包装器执行）：

```text
cargo fmt --manifest-path core/Cargo.toml --all
cargo test --manifest-path core/Cargo.toml -p paint-core --test brush_taper --locked
cargo clippy --manifest-path core/Cargo.toml -p paint-core --all-targets --locked -- -D warnings
```

最终结果：10项测试通过，Clippy通过。日志：`artifacts/brush-taper-kernel-tests.log`、`artifacts/brush-taper-kernel-clippy.log`（本机验收输出，不入库）。测试源为`core/crates/paint-core/tests/brush_taper.rs`。

- 尺寸/透明度独立曲线、最低值、起收笔重叠取min、线性与PCHIP、非单调平滑曲线保持范围；关闭值保留、零长度无作用、零最低值单点透明。
- 非法长度/端点/NaN/越界距离、零和最大容量；输入完整坐标、压力、倾斜、旋转、切向压力及单调时间/id验证。
- 400个交替主/第二流记录，含相同弧长记录及完整设备通道；逐条、13条与整批调度最终命令一致，每次预览与完整轨迹数值参考一致，冻结前缀的全局起笔距离不重置。这是调度命令比较，不是像素或PS视觉等效验证。
- 2000条长轨迹、容量4，仅保留4条活动笔尾；满容量后前进可正常冻结前缀，空输入前进会更新笔尾，相同终点空输入不生成重绘，结束只处理剩余尾部且不添加新终点。
- 同点密集尾部容量失败不改变状态；65536条上限成功，上限+1拒绝。已冻结记录仍参与id/时间次序约束。
- 错误批次、取消未采用、过期与异源计划不改变状态；同一不可变基础允许分支，提交不修改另一分支；结束后拒绝追加/再次结束。

无已有绘画路径、依赖、ABI或文件格式修改，因此未重跑Qt、IO或跨平台验收，也未替换开发版可执行文件。后续必须把同一调度器接到真实瓦片/覆盖场/湿混载体检查点，验证有限笔尾像素重绘、分页/选区/蒙版/锁/双流/随机以及失败整笔回滚，再接参数持久化和界面。保留用户确认的平滑拖绳距离，结束不补到原始鼠标位置。
