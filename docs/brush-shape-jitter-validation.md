# 形状抖动内核验收

2026-10-14。设计见 [形状抖动](brush-shape-jitter-design.md)。不可变参数/独立随机域、accepted笔触与逻辑/子印章序号、压力设备后几何、程序/图片共用覆盖、预算保持名义外界已实施。ABI仍1.39；本阶段没有Qt参数入口，非默认新配置不能通过旧IO保存。

实际检查：

- `cargo test --manifest-path core/Cargo.toml -p paint-core --test brush_shape_jitter --locked`：9项通过。分段一致、不同种子与下一笔新增覆盖、关闭/零抖动的旧像素及散布位置保持；三个通道各自改变真实覆盖、压力/设备组合；随机旋转孔洞图片的中心孔洞与半径外角点、非方形图片完整边界；选区擦除/蒙版透明度限制、精确撤销重做、涂抹真实红色运输和取消；同位置子印章各自产生不同覆盖、预算失败重试与参考序列一致；光标不受随机样本影响。
- `cargo test ... -p paint-core --lib brush_shape_jitter --locked`：2项通过。非法浮点和范围拒绝；1000逻辑印章×16子印章的大小/圆度下限与角度范围、独立通道、子印章及笔触序号变化、零抖动不启用随机机制。
- 核心相关回归：`--test brush_scatter` 7、`--test brush_device` 8、`--test brush_mask` 5、`--test brush_opacity` 7，全部通过。
- IO：`--test brush_shape_jitter_io` 1项通过，启用及关闭的非默认配置均明确拒绝state/pressure旧对象及错误VBR作用域，原目录保持；`--test brush_scatter_io` 4、`--test brush_pressure` 6通过。
- `cargo check --manifest-path core/Cargo.toml -p paint-ffi --all-targets --locked`、三包all-targets clippy（`-D warnings`）和相关Rust格式化通过。最后新增测试后再次执行核心tests clippy。

本轮未跑完整 `tools/check.py`、Qt构建及实体设备/跨平台/专项性能。已运行的Qt程序仍为上一阶段散布版本，本阶段不部署仅有内核的用户入口。参数单位和随机语义明确，但未宣称PS真实样本兼容；原生状态/VBR/Qt与ABR形状参数映射继续后续。随机颜色、纹理、双笔刷、平滑收尖、湿混及完整格式冻结仍未完成。
