# 时间平滑状态与VBR交换

2026-10-14：已有time-grid内核进入state14/VBR1.12/behavior11。新增预设级smoothing对象：model=time-grid-v1、enabled、positionMs/pressureMs（0..250ms）、speedResponse（0..1）、stabilizerPx（0..128px），以f64验证，非有限/未知/缺失/null字段拒绝；关闭非默认完整保留。默认不新增对象，旧状态/包仍保持原版本。保持主/第二共用过滤路径，不新增第二独立过滤器，不改内核算法或间距。

VBR模块位于Preset顶层，能力path-smoothing必需、profile drawverse.brush/behavior11，最高格式minor12；无图片也使用统一资源bundle IO，不把新模块写进旧程序包。原始ABR converter v1基线继续拒绝自身未表达的平滑；用户当前修改携带既有sourceResource，原记录与原字节不重写。当前ABI1.45/Qt最高state13尚未接设置，因此原生准备和公开库包入口明确暂拒新对象，防止旧Qt丢字段。下一阶段桥/Qt完成后再解除并公布能力；本轮不展示空控件或声称UI已完成。

验收：默认旧字节/版本，开关关闭值/精确参数，state14与VBR1.12两笔真实时间输入像素/历史/起笔捕获一致，图片/压力曲线/双路径/来源组合继续共用；旧版本/未知模型/比例及舍入前范围/null拒绝，原子整包和取消；Schema与版本能力一致；旧原生入口返回Unsupported且不发布或写文件。最小对应回归后提交。
