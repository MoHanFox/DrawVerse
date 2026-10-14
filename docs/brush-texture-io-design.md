# 纹理资源状态与VBR

state11 / VBR1.9 / behavior8 / texture-cover。纹理是预设级资源绑定对象texture（与tipResource同层，Parameters仍仅为无外部资源的基础参数）；state行同名对象。字段必需：model=periodic-coverage-v1、enabled、space=document/tip、scale/angle/depth/invert/offsetX/Y、resource（hash或null）。数值先f64范围验证，对象null/未知字段/模型/空间拒绝；启用必须有资源，关闭保留设置和资源。默认省略，不升级旧包。

复用完整SHA PNG覆盖资源，现有tip-mask清单角色在1.9也可由texture引用，同源只保存一份PNG。decode按hash共享TextureResource，预算包含唯一源Mask与新增周期mip，并在构建新增mip前检查；实际定义只有bundle完成绑定后才编译。当前及来源基线PNG并集保持，原始ABR来源记录不新增纹理基线字段。

本阶段Rust IO完成后，ABI仍1.41、Qt仅state10/behavior7；库package暂拒绝texture对象，读required=0且caller/资源不变，写不改目标。原生Qt入口后续接入，不宣称界面已可用。

验收：两坐标/全设置/关闭资源，state11和VBR1.9真实笔触恢复；共享资源身份与mip预算、丢失/额外/错hash/旧版本/模型/范围拒绝、失败原子性；同源笔尖+纹理与来源PNG/原ABR并集保持；旧状态/包不升级。完整系统、其他纹理组合与ABR转换继续后续。
