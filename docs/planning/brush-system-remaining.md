# 笔刷系统剩余功能与用途

2026-10-18，双笔刷覆盖模式阶段更新。已完成统一定义/资源目录、程序与图片笔尖、压力曲线/独立流量和整笔不透明度、形状/散布/颜色/纹理、主/第二笔尖散布幅度与数量的设备及渐隐控制、双笔刷8种灰度覆盖模式与ABR计算双笔刷、可选复合轮廓/半透明基础图片光标、平滑/起收笔、二维湿混/单色涂抹、ABR基础/设备来源（bVTy=3/6/8）/渐隐（bVTy=1）、VBR1.41动态来源矩阵/纹理模式/速度/切向压力/距离/倾斜/旋转/方向交换、PS分类设置及普通不透明大笔刷/硬橡皮跳过；完成范围以各阶段验收为准，不能据此宣称整体已达Procreate或完全兼容PS。用户已在Windows实机确认普通大笔刷/小间隔性能通过：约130px、1%间距时DrawVerse与Photoshop均卡顿，纯CPU的DrawVerse更流畅；湿混性能也已暂且通过用户验收，剩余性能项只指实体输入专项。

2026-10-18 模式收窄：双色绘制用户入口已删除，旧 state/VBR/ABR 中的 `dualBrush.color` 和 `wetMix.dualColor` 在 Qt 载入时剥离并按单色处理。纹理仅保留正片叠底、减去、变暗、叠加、颜色减淡、颜色加深、线性加深、实色混合、线性高度和高度；双重笔刷仅保留正片叠底、变暗、叠加、颜色减淡、颜色加深、线性加深、实色混合和线性高度。其他旧模式载入时回落为 `multiply`。见[设计](../brush/ui/brush-mode-cleanup-design.md)及[验收](../brush/ui/brush-mode-cleanup-validation.md)。

2026-10-18 补充：ABR 16位 raw/RLE 笔尖按大端全范围量化接入；ABR `szVr`/`prVr`/`opVr` 的 `jitter` 已接入 DrawVerse 确定性随机源和 state37/VBR1.44；非完全覆盖图片笔尖增加边缘/比例快速路径与固定基准；画笔库拖动增加防列表手势抢占。见[16位调查](../brush/abr/abr-16bit-investigation.md)、[传递抖动](../brush/abr/abr-transfer-jitter-design.md)、[图片笔尖性能](../brush/performance/brush-image-tip-performance-design.md)。

| 剩余项 | 功能与实际用途 | 当前边界/证据 |
|---|---|---|
| 实体输入专项性能 | 实测真实数位板高频输入、高DPI、偶发输入错误和模块组合；软边图片笔尖已有独立固定基准与边缘快速路径 | Document/Actor已接通原始输入续页、End/Cancel顺序和安全发布，普通不透明画笔、100%硬橡皮、纹理、图片笔尖和灰度双笔刷已加入完全覆盖像素跳过，固定基准中位数约1.33s降至0.37s且像素摘要一致；软边图片笔尖固定基准约3.34s（程序笔尖约2.37s），见brush-image-tip-performance-validation.md。普通大笔刷小间隔和湿混已由用户实机确认；实体输入仍由用户自行验收。 |
| ABR高级兼容与16位样本 | 更完整还原PS笔刷中的纹理、双笔刷、其他设备/渐隐控制；读取更多高位深笔尖来源 | 基础、压力/散布/颜色/形状/设备来源（`bVTy=3/6/8`）/渐隐（`bVTy=1`）/主第二笔尖散布幅度与数量设备控制（`bVTy=1/2/3/6/8`）/computed+sampled双笔刷（含scatter/count jitter、随机Flip和固定flipX/flipY）/双笔刷8种灰度覆盖模式/有界8/16位纹理图案/十种纹理覆盖模式（含高度、线性高度）/亮度对比度/最低深度与深度动态/纹理来源快照与恢复已接；ABR 16位 raw/RLE 笔尖已按大端全范围量化接入；`szVr`/`prVr`/`opVr` 随机抖动已接 state37/VBR1.44/behavior29。Hue/Saturation/Color/Luminosity仍未执行；见abr-dual-brush-validation.md、brush-dual-coverage-modes-validation.md、brush-fade-validation.md、brush-scatter-device-validation.md、brush-texture-dissolve-validation.md、abr-device-source-validation.md、abr-texture-validation.md及abr-transfer-jitter-validation.md |
| 扩展动态控制 | 用切向压力等驱动更多尺寸/流量/透明度通道 | 速度、切向压力、笔触距离、倾斜幅度、笔旋转和笔触方向动态已分别接入state24-29、VBR1.30-1.35及Qt曲线；任意来源选择矩阵已接入state32/VBR1.38，ABR `bVTy=3/6/8`设备来源及`bVTy=1`渐隐（state34/VBR1.40/converter10/handoff9）已接，基础压力、方向/旋转/倾斜、距离起收笔、时间喷枪及ABR传递随机抖动已接。其他ABR控制仍未接 |
| VBR分组与笔刷包组织 | 导入导出保留文件夹、分组和顺序，方便管理大笔刷包 | Rust组织树/目录、state20/VBR1.24及整树副本映射、资源发布/目标目录原子合并、ABI1.54准备及1.55完整包读写交接已接；Qt state20 镜像、树显示、复制/删除组织保持、重启恢复、上下文菜单、拖放排序、完整范围导出及分组包副本树导入已接，见brush-groups-import-validation.md。旧入口有组明确拒绝。 |
| Windows整体验收与输入问题 | 实测数位板手感、设备/高DPI、密集参数和模块组合，定位偶发输入错误 | 用户自行测试，无需agent测试 |
| 后续wgpu加速 | 通过GPU执行绘画/混色等重像素工作，复用核心参数与调度语义 | 当前CPU为参考；GPU仍是后续路线，尚无完整笔刷GPU后端。不是本轮已交付能力 |

用户已取消可交互试画，不能把它算作待补功能。自动预览固定白色/正常模式的要求继续保持。ABR导出、物理高度/光照颜料模型不属于当前首期承诺；Python插件排最后。

原双色湿混的涂抹与橡皮已由用户确认没有实际意义；双色绘制入口删除后，旧配置在载入时剥离，不列入剩余功能。

非分隔双笔刷的灰度覆盖仍以 `interleaved-stamp-v1` 保留在核心、state22、VBR1.28 和 Qt，见[设计](../brush/dual/brush-dual-stamp-design.md)及[验收](../brush/dual/brush-dual-stamp-validation.md)；双色贡献入口已删除，旧颜色配置会在载入时剥离。

时间喷枪已以 `time-interval-v1` 接入核心、state23、VBR1.29、ABI1.57 和 Qt，见[设计](../brush/wet/brush-airbrush-design.md)及[验收](../brush/wet/brush-airbrush-validation.md)；只支持干画笔和橡皮。

用户已发令继续推进；此清单记录当前剩余范围，具体实施及验收见各阶段文档。整体Windows设备手感由用户自行验收。

纹理已增加 Photoshop 风格的 `normal`、正片叠底、变暗、变亮、减去、差值、滤色、颜色加深组合模式及亮度/对比度调整，旧纹理缺省为 normal/0；ABR 的 `patt` 图案、`Txtr` 参数和上述八种模式/调整已接入实际执行，来源报告和资源往返已验。其余未定义 Photoshop 纹理模式仍属于高级 ABR 未完成范围。

半透明基础图片光标已完成Qt入口，默认关闭/25%不透明度，实际身份捕获及全局保存见brush-cursor-image-ui-validation.md；不等于随机/纹理/最终混色印章预演，也不恢复被取消的试画。

速度动态已以`speed-v1`接入核心、state24、VBR1.30、behavior19和Qt“传递”页，见[设计](../brush/dynamics/brush-speed-design.md)及[验收](../brush/dynamics/brush-speed-validation.md)。它以文档像素/秒统一驱动主/配对第二笔尖尺寸、流量和整笔不透明度上限；任意来源矩阵和ABR设备来源已接，ABR速度/渐隐字段、实体设备手感、跨平台和GPU仍属于后续范围。

切向压力动态已以`tangent-v1`接入核心、state25、VBR1.31、behavior20和Qt“传递”页，见[设计](../brush/dynamics/brush-tangent-design.md)及[验收](../brush/dynamics/brush-tangent-validation.md)。它使用设备-1..1切向压力归一化曲线；实体数位板是否输出非零切向压力由设备/驱动决定，尚未做硬件认证。

笔触距离动态已以`distance-v1`接入核心、state26、VBR1.32、behavior21和Qt“传递”页，见[设计](../brush/dynamics/brush-distance-design.md)及[验收](../brush/dynamics/brush-distance-validation.md)。它按活动笔触累计文档距离驱动三条曲线；ABR距离/渐隐来源和任意动态矩阵仍待后续。

倾斜幅度动态已以`tilt-v1`接入核心、state27、VBR1.33、behavior22和Qt“传递”页，见[设计](../brush/dynamics/brush-tilt-design.md)及[验收](../brush/dynamics/brush-tilt-validation.md)。它使用设备二维倾斜长度归一化曲线；设备无倾斜能力时保持原值。

笔旋转动态已以`rotation-v1`接入核心、state28、VBR1.34、behavior23和Qt“传递”页，见[设计](../brush/dynamics/brush-rotation-design.md)及[验收](../brush/dynamics/brush-rotation-validation.md)。它使用设备旋转角整圈归一化曲线；无旋转能力时保持原值。

笔触方向动态已以`direction-v1`接入核心、state29、VBR1.35、behavior24和Qt“传递”页，见[设计](../brush/dynamics/brush-direction-design.md)及[验收](../brush/dynamics/brush-direction-validation.md)。它使用输入段方向角整圈归一化曲线；无位移时保持原值。
