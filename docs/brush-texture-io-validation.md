# 纹理IO验收

2026-10-14。state11/VBR1.9/behavior8/texture-cover已接Rust IO，纹理是预设级资源绑定，ABI仍1.41。设计见 [纹理保存](brush-texture-io-design.md)。同源PNG只保存一次，按hash共享周期TextureResource；源Mask与周期mip计入预算，新增mip构建前拒绝超预算。当前Qt尚未接，库包入口暂拒绝texture对象。

实际验证：

- `paint-io --test brush_texture_exchange`：3项通过。两坐标/启用关闭/旋转深度反相偏移、state11和VBR1.9恢复真实连续两笔逐像素一致；同源笔尖/纹理、两预设共享同一源Mask和周期资源，原PNG字节保持；低1字节预算拒绝、旧VBR降级拒绝。坏模型/空间/类型/范围和轻微越界、资源null/缺字段/缺文件拒绝，合法最低缩放可重新验证。新Schema模型、源基线角色和5120清单上限有断言，未运行第三方Schema验证器。
- `brush_texture_io` 1项通过，未持久身份的启用/关闭纹理仍明确拒绝保存；`brush_color_exchange` 5、`brush_state` 4、`vbr_image` 4项相关回归通过。
- `vbr_source` 6项通过。实际纹理叠加设备/散布/形状/颜色的非空图片结果还原，完整原ABR和资源/来源身份保持；1024预设各有独立当前笔尖/纹理/来源基线和原包，3072 PNG、5120清单与6145 ZIP条目可完整往返。旧minor仍限定4096清单，原角色和基线不变。
- `paint-ffi --test brush_source`：8项通过。旧库包入口暂拒绝texture时required=0且caller字节/目标文件不变；既有来源/压力/设备/散布/形状/颜色读写保持。
- `cargo check --manifest-path core/Cargo.toml -p paint-ffi --all-targets --locked`、三包all-targets clippy（`-D warnings`）、Rust格式化和diff检查通过。

未跑完整 `tools/check.py`，未做Qt构建、实体设备/跨平台及性能基准。本阶段不部署新界面；原生资源采用及Qt纹理入口下一阶段接入后解除临时包边界。其他纹理组合、ABR纹理映射、双笔刷、平滑收尖、湿混和完整格式冻结仍未完成，本阶段不是完整系统验收。
