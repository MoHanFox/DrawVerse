# 双色双笔刷原生与Qt入口验收

2026-10-15，Windows/MSVC/Qt6.8.3 Release，ABI1.48。本轮为相关最小回归，未运行完整tools/check.py。

- Rust paint-ffi：native_dual_color_preparation_capture_and_package_entry通过，准确state16准备、捕获后修改仍保留原配置、关闭值及图片资源库包导入/写入/重开；日志artifacts/brush-dual-color-ui-native.log。
- Qt DPR1：dualColorClosedValuesHighestVersionResetAndPreviewTokens、dualColorControlsTwoColorPreviewCaptureExchangeAndReopen、dualSettingsImportPreviewPixelsHistoryAndExchange，3个用例全部通过。严格字段/state15拒绝、完整基准/最高版本、旧预览票据、实际两色预览、起笔交换色不影响笔触、独立VBR像素及undo/redo、保存关闭重开。
- Qt DPR2：双色入口、图片双笔刷与重置、相对尺寸入口、工具混合缓存，共4个用例通过。真实点击验证双笔刷页重置隐藏，来源与兼容页可见且能恢复完整资源/参数；历史不变。截图artifacts/brush-reset-source-{1,2}.png和brush-dual-color-ui-{1,2}.png。
- 4项静态/共享C/C++ ABI烟雾检查通过；生成头--check通过；Release构建成功；paint-ffi全目标clippy -D warnings通过。

当前模型为整笔隔离双色合成，工具模式预览仍固定正常，实际两色不写入预设。双色橡皮/涂抹明确拒绝。自动描边快捷预设、组合光标、非分隔逐印章模型、真实数位板与跨平台、完整湿混和高级ABR仍未完成。
