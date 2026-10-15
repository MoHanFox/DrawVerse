# ABR 笔压动态阶段验收

2026-10-15，转换器v3 / 来源record1 / ABR交接JSON2 / VBR1.17；既有state5/6与behavior2/3执行，无C ABI变化。

已实现：useTipDynamics/szVr、usePaintDynamics/prVr/opVr笔压与三个最低比例，独立尺寸/流量开关与opacity压感；minimumDiameter优先且冲突Mnm明确未执行；开启模块严格单位/类型/舍入前范围。非零抖动仅保留并报告，笔压分量独立执行；渐隐/未知控制、工具覆盖和保存姿态不误映射。字面点号键不能因报告路径相似而被跳过。

来源：新执行笔压才用converter3，基准含完整压力DTO；v1/v2入口/记录/报告保持，不重算旧数据。v3必须有压力基准，v1/v2拒绝压力基准；其他动态仍拒绝。ABR交接2防旧Qt静默丢最低比例和opacity；Qt接受1/2、复制完整对象到既有state。VBR新旧三代来源混合按最高需求17，16及以下降级拒绝，原记录与原ABR字节保持；无来源导出仍按既有执行能力选择版本。

验证记录：

- `artifacts/abr-pressure-core.log`：paint-io abr_conversion 13、abr_import 4、brush_source 8、vbr_source 10，共35项通过。涵盖独立/组合与开关/未知控制/工具覆盖/非零抖动/错误单位及边界、旧转换对照、真实尺寸/alpha压感与四档压力、两笔程序/图片来源基准及单步历史、三代来源/最高minor/降级/字节和像素往返。
- `artifacts/abr-pressure-ffi.log`：公开ABR读取/缓冲保护/整包持久化/准备准确状态与来源基准3项，原生来源/包9项，共12项通过。
- `artifacts/abr-pressure-clippy.log`：paint-io / paint-ffi all-targets、`-D warnings`通过。
- Draft202012Validator检查新Schema，3个合法/3个非法来源通过；Rust也读取发布样例并验证v3/旧版本降级与未知转换器拒绝。既有1.16 Schema及样例未改写。
- `artifacts/abr-pressure-build.log`：Release构建及生成头字节检查通过；`artifacts/abr-pressure-abi.log`：C/C++ static/shared四项通过。
- `artifacts/abr-pressure-qt.txt`：4个相关Qt函数通过（含初始化/清理6 passed，0 failed）。新用例真实ABR导入三通道/最低比例、交接降级拒绝且库不变、编辑后重置、低压力密集绘画alpha上限/undo/redo、全新库VBR导入后逐像素一致及偏好重开；既有opacity/整包来源/改图后来源交换回归通过。

最小相关检查，未运行无关全量验证。字段参考ag-psd的ABR描述符；人工生成独立wire fixture不代表Photoshop授权资产或PS逐像素等价认证。实际PS工具覆盖、完整随机/渐隐/其他动态、16位采样、旧参数ABR、湿混双色/橡皮、收尖和非分隔等继续后续；整个笔刷系统目标尚未完成。
