# ABR基础转换验收

2026-10-13：采样/程序笔尖基础参数映射、严格单位/原生范围、真实BrushProgram编译、共享覆盖、明确100%工具缺省近似与逐叶字段报告已实现。完整原预设描述符保留引用。未知动态/纹理/工具字段未执行；非零角度方向待PS对照，true翻转/spacing-off未执行。非100%PS整笔opacity不会映射为印章opacity。

默认6项相关测试通过（转换4+身份关联2），补最终报告边界后转换4项再次通过：图片孔洞/程序实际像素与不凭空压感、共享Arc、间距单位、非100%接近100的双精度opacity仍标记未执行、具体动态子路径/部分状态/完整来源、错单位/缺字段/非有限/超范围/晚失败整包拒绝/取消，以及报告超限拒绝不隐藏字段。paint-io全目标Clippy -D warnings通过。日志 `artifacts/abr-conversion-tests.log`、`artifacts/abr-conversion-final-tests.log`。

明确执行的外部诊断1项通过，沿用 [关联阶段来源与hash](abr-association-validation.md)，资产不提交。样本尺寸112、spacing0.02、roundness1与上游结果核对，使用转换后的真实原生定义，112×105无动态基线alpha逐像素符合参考（浮点容差3e-6）；得到97项具体报告并判定部分转换，textureDepth/szVr.bVTy等未执行。日志 `artifacts/abr-conversion-external.log`。这仅验证基础覆盖，不证明PS动态、颜色/混合或整条笔触等价。

本阶段无新ABI/Qt入口，未运行全仓库检查。仍需来源/报告持久化、旧1/2转换桥、原生/Qt整包导入、动态及湿混实现、角度/翻转与PS像素核验、授权可分发真实语料。完整笔刷系统和完整ABR兼容仍未完成。
