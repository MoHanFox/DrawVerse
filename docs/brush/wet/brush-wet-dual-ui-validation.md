# 湿混双色 Qt 组合入口验收

2026-10-16，Windows/Qt6.8.3。对应[设计](brush-wet-dual-ui-design.md)。依赖ABI1.52，不增加ABI或格式版本。

Qt完整镜像state19及关闭策略；两种启用顺序自动原子加入独立载色标记，关闭后保留。复制、resetState、原生规范化采用、重启、behavior16/capability和VBR1.23读写全部走新明确入口。旧未标记活动组合/降级/未知模型仍拒绝，不丢字段。手动操作捕获准确预设、尺寸和前景/背景，两角色状态检查与原子reload在后台执行；预览使用独立工具，不消耗实际载色。设置页说明两角色共享湿混比例，按钮在双色活动时显示“载入双颜色”，保留通用控件与1秒提示。

最小Windows验证：

- 新模型用例 `wetDualModelPreservesClosedPolicyBaselineAndStrictState19` 通过：双向启用、关闭值保留、保存副本/独立重置基准、恢复/导入及标记类型/降级/未标记活动组合/错误behavior拒绝，失败保持当前目录。
- 新真实QML用例 `wetDualControlsLoadBothCapturedColorsPreviewAndHistory` 在环境倍率1和2通过（环境缩放，不宣称真实硬件DPR2）。真实点击启用/载色/clean，载色请求后立刻改两色仍绘出已捕获红色主笔和蓝色溢出区域；预览所有可见像素白色、alpha保留、换色不重新请求、画布历史不受预览影响；在途修改第二尺寸保持原捕获，第二笔保留两色，undo/redo完整帧一致，clean后无新增笔画。QML无警告。
- 新交换用例 `wetDualImageVbrFreshStoreAndReopenMatchCapturedPixels` 最终通过：主图片/第二图片/纹理共用资源，真实VBR导出到全新目录导入；起笔后改两色和关闭湿混仍保持捕获，整个连续轨迹的完整RGBA帧与导入后/重开后绘画一致。关闭非默认值也经实际VBR文件导入保持，导入基准重置/偏好保存重开保持；按核心f32实际语义比较传输数值，不把Qt初始f64小数与原生规范化结果逐位混淆。
- 旧6项相关Qt用例通过：wetModelPreservesClosedPolicyResetBaselineAndExchange（更新旧组合拒绝断言为显式策略）、wetSettingsPageManualInkAndPreviewStaySeparateFromCanvas、wetPolicyPersistsSharesAcrossDocumentsAndImportsFromVbr、wetSmudgeUsesOwnToolStrengthAndFingerPolicyWithUndo、allAutomaticBrushPreviewsStayWhiteWithExactAlphaAndColorIndependentCache、taperControlsCaptureWhitePreviewPixelsVbrAndReopen。
- Release和cbindgen生成头一致性通过，C/C++ static/shared四项ABI通过。仅Qt层改动，未重复原生核心全量套件或跨平台验收。

首轮记录及限制不能省略：第一轮新图片交换用例失败（其余8项通过），单次64px密集图片双色湿混超过核心8M估算输入预算，诊断明确resource limit并整笔回滚。未提高预算或修改绘画算法；最终用例以连续8px输入覆盖相同完整轨迹。该结果不证明大跨度输入已支持，**有序分块调度仍待实现**。随后测试发现原生f32规范化与Qt草稿f64小数的比较差异，修正测试参考到实际核心精度后，真实文件/完整像素/重开验证通过。初始失败和诊断日志保留，不能将结果描述为首次全绿。

此前湿混参数输入首次偶发失败本轮旧用例通过，仍未定位根因，不宣称修复。双色湿混目前仅用于画笔；双色涂抹/橡皮继续明确拒绝。

本机日志：`artifacts/brush-wet-dual-ui-release.log`、`...-qt.txt`（首轮）、`...-image-diagnostic.txt`、`...-image-trace.txt`、`...-image-final.txt`、`...-qt-scale2.txt`、`...-abi.log`，及对应构建日志。均不入库。完整系统剩余范围见[剩余功能](../../planning/brush-system-remaining.md)，用户要求本阶段完成后等待下一次指示。
