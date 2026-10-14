# 第二笔尖尺寸保存与交换

2026-10-14，采用state15/VBR1.13/behavior12。DualParameters新增可选sizing对象：model=relative-size-v1、mode=independent/relative、diameterOffset=[-511,511]px、widthRule=follow-pressure/fixed-width。对象存在时全部字段必需，未知/null/类型错误与舍入前越界拒绝；独立模式和关闭双笔刷的非默认选项同样保存。默认对象省略，旧字节和版本不升级。

VBR预设必须声明dual-relative-size能力及drawverse.brush/behavior12，manifest minor13。与平滑/图片/动态组合按最高版本选择；原ABR转换器只建立旧尺寸模型，原记录不改写。state15与新Schema完整保存实际模型，不仅保存界面值。

当前Qt/ABI入口尚未表达尺寸新对象，原生state准备、资源包接入及库包写入暂拒新对象，避免成功后丢参数。下一阶段原生捕获/Qt接通后解除。无C ABI变化。

验收：默认旧字节及版本，关闭/独立/相对两规则持久化，state/VBR恢复后两笔真实像素及历史一致，最高版本与能力/格式低版本拒绝，严格字段和舍入前边界，原生拒绝不能发布或写文件。仅跑对应层回归及构建。
