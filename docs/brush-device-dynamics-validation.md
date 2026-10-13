# 设备与路径笔尖动态内核验收

2026-10-14。TipDynamics不可变配置与每印章几何已进入真实核心绘画：Fixed/Direction/PenRotation/Tilt角度来源、倾斜圆度及最低比例；程序/图片笔尖共享几何，设备能力缺失使用基础预设，周期旋转走最短路径。方向首印章采用基础角度，后续使用实际线段方向。

通过的最小回归：

- paint-core `brush_device` 7项：真实程序笔尖旋转/倾斜及能力缺失、倾斜最低圆度、方向急转、共线输入拆分精确像素、非方形图片旋转完整角落、非对称图片350→10°跨零方向、擦除/蒙版/手指涂抹与选区共用、在途定义捕获及撤销重做。
- `--lib device_interpolation_tests` 1项：跨360°短路径及能力切换不从缺失通道插值出虚构倾斜/旋转。
- 现有 `brush_mask` 5、`brush_program` 3、`brush_opacity` 7、`brush_pressure` 5项通过；包含默认程序像素兼容、图片孔洞/资源快照/旋转预算、整笔不透明度及压力曲线原有执行。
- paint-io `brush_pressure` 5项通过，新增断言非默认设备几何不能被state/VBR或pressure DTO默默序列化成静态笔尖；既有v5/v6、VBR1.3/1.4继续往返。
- 核心/IO/FFI all-target clippy `-D warnings`通过。

ABI无变化。没有新增Qt控件，没有宣称用户可启用这些控制；保存格式/原生入口/设置/实时光标在后续阶段接入。未跑全量tools/check.py、Qt回归、真实数位板、跨平台或性能基准，不宣称Procreate/PS手感达标。随机、纹理、散布、双笔刷、颜色动态、路径平滑、湿混及ABR动态仍继续后续。
