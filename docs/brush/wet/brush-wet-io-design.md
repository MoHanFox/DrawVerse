# 湿混预设状态与VBR交换

2026-10-15。state17、VBR1.15、drawverse.brush/behavior14新增预设顶层wetMix完整对象：model=linear-premul-grid-v1、enabled/allLayers/autoClean/autoLoad布尔，以及wetness/pickup/mix/load/deposit/consumption六项0..1比例。严格完整字段、未知字段/模型/null/非布尔拒绝；f64先检查有限和范围，再转f32，避免舍入掩盖越界。默认整个对象省略，保持旧字节/版本；关闭非默认数据仍完整保存，整库/整包选择最高版本。

实际前景/背景和二维载体不进入预设，起笔按已完成配置入口自动清洗/装载或保留工具状态。真实往返测试必须跨两笔复用返回载体、用同一颜色/输入得到相同像素与历史，而不是只比JSON。组合图片/纹理/散布/压力/PCHIP/时间平滑/灰度双路径不新增资源角色；原始ABR来源与旧转换基线保持分离，converter v1仍拒绝非默认湿混基线。

wetMix对象存在要求wet-mix能力、profile与行为14、manifest minor15；资源bundle IO负责执行与导出，不写进旧procedural-only容器。Schema从现有完整1.14子集扩展，能力上限17。有效湿混+有效双色仍明确拒绝，关闭组合保留。

当前ABI1.49/Qt未接湿混字段，因此原生准备及公开库包读写入口暂拒非默认湿混/含wetMix的包，必须在资源导入、库发布或写文件之前报Unsupported。下一阶段再接完整准备/Qt并解除，不能让旧Qt丢值或普通绘画代替湿混。

验收：默认旧字节，启用/关闭参数及最高版本、真实两笔与载体结果/state/VBR/历史一致、图片来源保持；严格字段与舍入前范围、旧版本/错误能力/模型/manifest降级拒绝；整包与取消原子性、Schema对应；旧native准备/包入口准确拒绝且不发布、不修改已有文件和资源。
