# 原生第二笔尖资源生命周期验收

2026-10-14，无ABI变化。PreparedCatalog 已将第二图片加入导入交接、发布/变更采用以及闲置准备记录释放。当前、在途目录持有真实资源；没有消费者的旧记录只保留state12配方和资源目录，重查从PNG验证身份后重编译。关闭配置同样拥有资源，不被忽略。

`cargo test --manifest-path core/Cargo.toml -p paint-ffi --lib brush_preparation::tests --locked` 11项通过，其中新增2项：

- 16组（配对/独立路径×启用/关闭×第二独占/共用主笔尖/共用纹理/三者同源）导入只暂存一个同源覆盖；默认目录不能误采用；发布和mutate均完成正确交接，图片Arc身份共享，当前state12准确保留。
- 第二独占的4组配置在错误资源身份准备失败后保持有效当前目录；切换后捕获消费者仍能绘出原像素，消费者释放后旧源覆盖/mip真正释放，按原配方重新准备产生相同JSON、实际笔触及撤销重做结果。

既有纹理、主图片、来源凭据、ABR采用、准备失败/淘汰回归通过。`cargo clippy --manifest-path core/Cargo.toml -p paint-ffi --all-targets --locked -- -D warnings`、cargo fmt 与 git diff --check通过。

ABI仍1.42、Qt仍state11；公开库package对dualBrush的临时拒绝保留。下一阶段接完整Qt保存/设置、第二图片导入目标与资源导出后解除，不宣称用户已能操作双笔刷。未运行全量tools/check.py、Qt构建、跨平台或实机验收，开发版未更新；完整笔刷系统继续后续。
