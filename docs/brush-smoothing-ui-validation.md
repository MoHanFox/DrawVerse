# 时间平滑原生桥与Qt验收

2026-10-14，ABI1.46/state14/VBR1.12/behavior11，PAINT_FEATURE_PATH_SMOOTHING=2199023255552。既有准确修订、资源目录、两色和模式捕获入口直接复制完整time-grid-v1，解除前阶段临时拒绝。无DTO/导出签名变化；旧无库程序包仍拒绝扩展，不能用正常程序包冒充。

设置新增“平滑”分类、统一紧凑开关/四字段/滑条/1000ms说明，默认关闭、12ms位置/8ms压力/50%速度/0px稳定距离；关闭值可编辑。预设镜像和完整重置基准、共享偏好、native规范化及VBR保存完整对象；最高目录版本不被后续压力或旧行降级。显示整数不在加载时回写舍入值。启用预览使用真实8ms递增时间输入和同一Rust过滤，预览仍固定正常模式，用户文档不变。

最小对应验证：
- native_smoothing_preparation_capture_and_package_entry：启用/关闭准确采用、后续mutate不改变捕获修订、完整库包写/重开参数及旧程序写入不覆盖文件，通过。
- smoothingPresetsKeepClosedValuesHighestVersionAndResetBaselines：关闭24.125ms精确保存、复制基准/重置、state14最高目录、非法舍入前范围/模型/类型原子拒绝、旧state13拒绝、VBR模型往返，通过。
- smoothingControlsPreviewCaptureExchangeAndPreferencesReopen在DPR1/2通过：真实鼠标编辑和开关、实际正常预览发生变化并等待图片Ready/渲染；VBR导入到另一客户端，以同时间输入比较真实像素；起笔后关闭仍使用已捕获平滑，下一笔关闭像素不同；真实历史/后台保存与关闭重开值一致。截图artifacts/brush-smoothing-ui-1.png、-2.png检查窄列与可见笔触。
- DPR2双笔刷资源/历史、PCHIP曲线、普通混合捕获/正常预览和完整基准相关6项通过（不含init/cleanup）。Release Qt及生成头字节检查通过，C/C++静态/动态ABI四项通过，paint-ffi全targets Clippy -D warnings通过。

证据：artifacts/brush-smoothing-ui-native.log、brush-smoothing-ui-1.txt、-2.txt、brush-smoothing-ui-image-1/2.txt、brush-smoothing-ui-build.log、brush-smoothing-ui-clippy.log。没有运行tools/check.py全仓，不报告完整验证通过。未宣称收尖、双色复合/自动描边、湿混或高级ABR已经完成；这些仍在完整目标内继续后续。
