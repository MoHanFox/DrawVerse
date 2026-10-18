# 湿混与颜色动态验收

2026-10-15，Windows/MSVC/Rust、Qt6.8.3 Release。真实湿混Document允许既有ColorDynamics，未增加ABI、state或VBR版本。仅验证本阶段及相关回归，未运行完整tools/check.py。

- 新增brush_wet_color五项真实文档测试通过：两种频率×像素/蒙版/透明锁×四种工具混合共24组合，与普通纯载色单印章参考误差小于2e-6；改变标志及真实undo/redo一致。另验证透明沉积不因背景色产生油墨、压力opacity/载色alpha只作用一次、原始载体不被直接变色、关闭参数位级保持、两种频率可见差异、取消恢复。
- 同组测试以图片、局部纹理、纸纹、散布、灰度双路径和时间平滑组合验证分页/输入拆分、返回载体及历史；启用与关闭动态确有像素差异。与既有brush_color_dynamics九项、brush_wet_document十二项共26项集成用例通过，日志artifacts/brush-wet-color-core.log。
- prepared_color_transform_preserves_legacy_sampling_bits：20,736组开关/频率/参数/种子/笔触序号/散布child/两色alpha与灰度组合，对照保留的旧算法；prepare/apply及普通sample均逐f32位一致。Debug和Release均通过，日志brush-wet-color-reference.log及brush-wet-color-reference-release.log。
- 内部wet三项回归通过，暂存与历史写失败分别覆盖颜色动态开/关，整笔恢复像素、已有历史及调用方载体；滤波静止压感总预算回归保持。日志brush-wet-color-failure.log。既有brush_color_exchange五项真实参数/state/VBR/像素回归通过，日志brush-wet-color-io.log。去重共35项Rust用例，位级参考在两个构建配置执行。
- paint-core/paint-ffi全部目标clippy -D warnings、生成头--check、Release构建及四项静态/共享C/C++ ABI smoke通过，日志brush-wet-color-{clippy,header,build,abi}.log。
- Qt colorDynamicSettingsUseCapturedForegroundBackgroundAndRoundTrip通过：实际设置、两色与模式捕获、预览、VBR/保存重开、真实像素和undo/redo。日志brush-wet-color-qt.txt；这是现有普通笔刷回归，尚没有Qt湿混入口。

每实际主印章准备一次随机变换，沉积取样后再变色，RGBA账本保持湿混语义，蒙版在变色后转可见灰度；额外工作纳入印章与像素预算。未增加性能达标声明。

后续：湿混独立参数IO、完整准备桥/ABI/Qt和工具载色清洗/重载调度；有效双色湿混仍明确拒绝。收尖、非分隔绘制、高级ABR和完整跨平台/设备验收仍未完成，不将本阶段视为完整笔刷系统完成。
