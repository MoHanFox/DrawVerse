# ABR设备形状转换验收

2026-10-16，Windows优先，跨平台后置。对应[设计](../../../brush/abr/abr-device-conversion-design.md)。converter7 / 来源record1 / 交接JSON6 / VBR1.21；使用现有TipDynamics、state7/behavior4与Qt形状动态页面，无C ABI变化。

角度方向、笔旋转、倾斜角度和倾斜圆度已从ABR映射，独立叠加既有压力、随机、颜色与散布。设备缺失使用基础几何回退；未知/初始方向、圆度笔压、渐隐和笔姿/工具覆盖继续保存并报告未执行。原生角度与余弦圆度不是PS设备公式等效证明，映射及最低圆度换算/裁剪明确近似。通道Mnm/fStp只保留，不声称已解释设备最小值或渐隐。

关闭设备配置没有被转换成活动控制：TipDynamics无关闭位，关闭设备字段在原始来源和未执行报告保留，既有完整关闭随机配置仍保持。旧六代入口、基准、报告与原记录字节不重算。v7来源必须含tipDynamics，其他四类动态独立可选，压力DTO剥离设备字段，纯设备不强加形状或压力对象。VBR按所有来源最高版本需求21导出，20及以下夹带新来源拒绝。Qt使用既有设备参数、完整重置基准与持久化路径。

最小相关验证：

- paint-io相关56项通过：abr_conversion28、abr_import5、abr_real_corpus1、brush_source8、vbr_source14（artifacts/abr-device-io.log及追加边界后的abr-device-conversion-final.log）。程序与非方形孔洞图片、三种角度设备及缺失能力、压力/随机/散布/颜色组合真实两笔、固定几何差异、逐像素原生对照和undo/redo；独立不支持通道不抹除已支持通道，最低圆度缺省/0/25/100%及真实几何下限。关闭/未知/笔姿/工具覆盖保持v6行为和报告，非法类型/单位/舍入前范围拒绝；七代混合来源原记录字节及最高minor/降级/真实设备笔触像素一致。
- 公开FFI ABR读取/准备/来源8项通过（artifacts/abr-device-ffi.log），新增方向/旋转/倾斜的纯设备及组合配置准确准备，纯设备无多余DTO；既有真实148预设整包继续通过。
- paint-io / paint-ffi all-targets Clippy -D warnings通过（artifacts/abr-device-clippy.log）；最后追加的边界用例单独检查见abr-device-clippy-final.log。
- Draft202012Validator通过新Schema及七代合法来源、可选四模块分别和全部省略、未来版本/降级/缺失设备/非法圆度和控制类型（artifacts/abr-device-schema.log）。旧Schema和样例未改写。
- Release、生成头字节检查通过（artifacts/abr-device-release.log），C/C++ static/shared四项通过（artifacts/abr-device-abi.log）。
- Windows Qt6.8.3 / windows平台 / software Quick后端，3项功能含初始化清理5 passed、0 failed（artifacts/abr-device-qt.txt）。新设备用例关闭及三个角度模式，完整参数/编辑重置、低压力两色真实绘画与alpha上限/undo/redo、旧交接原子拒绝、删除ABR后新目录VBR逐像素一致和偏好重开；既有形状与颜色导入回归通过。

未运行无关全量测试、外部可选诊断或其他操作系统验收；受控设备输入不代表实体Windows Ink硬件认证。完整ABR/16位、其他设备与高级动态/PS视觉核对、湿混双色与橡皮、收尖/非分隔、VBR分组等仍待后续，完整目标保持进行中。
