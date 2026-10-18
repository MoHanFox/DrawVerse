# 原生纹理资源生命周期验收

2026-10-14，ABI 保持 1.41。PreparedCatalog 已将纹理源纳入包导入交接、发布采用及闲置修订释放；启用和关闭均保留资源身份，同源笔尖与纹理共享覆盖 Arc。在途消费者持有不可变纹理，释放后从 state11 配方和持久 PNG 重新准备。

通过最小回归：`cargo test --manifest-path core/Cargo.toml -p paint-ffi --lib brush_preparation::tests --locked`，9 项通过。新增两项覆盖：纯纹理/共用笔尖、启用/关闭交接及不误采用；当前配置缺失资源引用拒绝且不替换有效目录、在途保持、旧纹理源与周期 mip 释放、重建 JSON 及实际像素一致、真实撤销重做。既有图片、来源基线、ABR、准确准备和修订淘汰回归通过。

`cargo clippy --manifest-path core/Cargo.toml -p paint-ffi --all-targets --locked -- -D warnings`、cargo fmt 与 git diff --check 通过。

未更改公开 ABI、Qt 或公开包兼容边界。Qt 尚不能保存 texture 对象，package 暂拒绝仍有效；设置界面及纹理包用户入口继续下一阶段。未运行全量 tools/check.py、Qt 构建或实机性能验收；完整笔刷系统尚未完成。
