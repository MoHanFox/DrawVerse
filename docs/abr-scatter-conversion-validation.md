# ABR散布转换验收

2026-10-15，按用户确认以Windows为当前验收平台，跨平台验收后置。converter4 / 来源record1 / ABR交接JSON3 / VBR1.18；复用既有state8/behavior5、Qt散布页与统一内核，无C ABI变更。

## 实际行为

useScatter、bothAxes、Cnt 、scatterDynamics.jitter与countDynamics.jitter进入真实ScatterDynamics。数量支持描述符Integer及整数值Double；实际Photoshop探针保存Double(4.0)，小数/非有限/越界拒绝。幅度0..1000%与数量抖动0..100%舍入前校验，native count1..16。完整关闭非默认配置保留，关闭默认保持旧版本/报告，不按非默认无关字段推断旧来源版本。

使用原生seed0及文档笔触序号，非零随机执行标Approximate；PS位移/数量分布和随机序列未作逐像素认证。bVTy设备/渐隐调制不执行，完整原始字段与报告保留。开启/关闭配置、压力/程序/图片共用真实内核，不引入C++采样算法。

v4来源必须有scatter基准，可组合独立pressure DTO；v1/v2/v3不允许夹带scatter。独立v3转换入口保持旧报告；旧记录字节不重算。pressure DTO序列化先剥离scatter且按版本门控，避免新字段污染旧手工基准或把其他动态放进压力对象。VBR四代混合按最高需求18，17以下降级拒绝；不含v4来源的既有包需求不变。交接JSON3使旧Qt明确拒绝，当前Qt1/2/3复制完整配置与重置基准。

## 最小相关验证

- paint-io 42项相关测试通过：abr_conversion最终17、abr_import5、真实语料1、brush_source8、vbr_source11。日志artifacts/abr-scatter-core.log与abr-scatter-conversion-final.log。含单/双轴、数量/抖动/完整关闭、真实程序/图片及压力组合的两笔像素与历史、实际散布范围、整数值Double/非法小数和范围、旧三代转换、关闭默认报告不升级、四代记录字节/最高minor与降级拒绝。
- 显式外部诊断1项通过（artifacts/abr-scatter-external.log）：ag-psd原样本真实压力+散布已执行；参考PNG只在隔离scatter的基础笔尖路径核对alpha，实际散布路径另核对半压力alpha上限/保守范围及来源。该资产仍不分发，不声称PS完整笔触等效。
- 公开FFI ABR读取/准确准备/来源5项通过，包括新v4及真实148预设整包（artifacts/abr-scatter-ffi.log）。
- paint-io / paint-ffi all-targets Clippy -D warnings通过（artifacts/abr-scatter-clippy.log）。
- Draft202012Validator新Schema通过4个合法/4个非法来源，Rust也读取发布的新v4/降级/未来版本样例；旧Schema与原样例未重写。
- Release及生成头字节检查通过（artifacts/abr-scatter-build.log），C/C++ static/shared四项通过（artifacts/abr-scatter-abi.log）。
- Windows Qt6.8.3原生windows平台、software Quick后端，3个功能用例通过，含初始化/清理5 passed、0 failed（artifacts/abr-scatter-qt.txt）。新用例各跑开启/关闭配置，核对完整参数、旧交接版本拒绝且库不变、编辑后重置、真实低压力密集绘画/undo/redo、带来源VBR新目录逐像素一致和偏好重开；原压力及真实148预设整包保持通过。

未运行无关全量或其他操作系统验收。当前Windows阶段通过；完整ABR设备散布/数量控制、其他高级动态/16位/旧ABR参数、湿混双色/橡皮、收尖/非分隔等继续后续，完整笔刷系统目标尚未完成。
