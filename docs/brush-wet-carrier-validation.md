# 二维湿混载色内核验收

2026-10-15，Windows/Rust，纯paint-core数值组件，无ABI或现有state/VBR变化。尚未接BrushDefinition/Document事务或Qt；不宣称应用已经可以使用湿混笔刷。

cargo test --manifest-path core/Cargo.toml -p paint-core --test brush_wet_carrier --locked：7项全部通过，日志artifacts/brush-wet-carrier-core.log。

- 两维红蓝载色保持不同位置颜色，中心为线性混合；移动后保留分布，事务克隆保持原数据，UV外/NaN透明。
- 载色、拾色、混合、沉积的预乘RGBA逐项计算，半透明取样不重复乘alpha；透明底稀释、干笔零取样与零沉积。
- 旋转90度、圆度0.5、压力尺寸变化的实际世界格点及输出颜色。
- 在实际RGBA数组与blend_pixel上完成“读取全部→写入全部→提交载体”，下一准备读取前次写入而非旧底图；这是载色组件的像素回归，不是Document历史测试。
- 距离消耗与拆分在1e-6容差内一致，驻点不消耗，清洗/重载与重置路径，consumption=1边界。
- 非有限/超范围参数、几何和取样权重/回调失败不改变载色；独立来源准备、过期准备和清洗后的旧准备拒绝。基础状态的克隆可用于事务快照，不使用全局对象身份。
- 最大64×64网格4096次取样，最小直径与512边界。源码审计：原/下一/输出三个Pixel网格加一份20字节WetSample网格，转换Arc前释放源，单准备数据载荷约272KiB峰值；不把此数据量当作实测进程内存或性能结果。

paint-core全目标clippy -D warnings和git diff --check通过，日志artifacts/brush-wet-carrier-clippy.log。未运行完整tools/check.py；没有界面变化，本轮不重新部署Qt程序。

下一阶段仍需真正接入逐印章Document采样/写入/预算、取消和历史暂存失败回滚、目标/合成/蒙版取样及选区/锁、持续载色清洗/重载调度、state/VBR/准备桥和Qt；ABR高级映射、收尖和其他全系统验收仍未完成。
