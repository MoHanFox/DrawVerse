# 颜色动态原生Qt验收

2026-10-14，Windows11 / Rust / MSVC / Qt6.8.3。ABI1.41、state10/VBR1.8/behavior7/color-dynamics。设计见 [入口](../../../brush/ui/brush-color-ui-design.md)。新增24字节背景DTO和新prepared导出，旧ToolDesc/入口尺寸语义保留；旧入口白色背景。Qt启动至少ABI1.41。

完成：颜色分类/启用/逐笔尖与整笔/前景背景和HSV抖动、通用百分比输入滑动条及1秒提示；模块关闭可继续编辑参数且不自动启用，同时解除散布和形状父组禁用。独立每预设对象、完整seed/model/关闭值、目录与批量最高版本、复制/恢复和VBR交换。真实绘画同时捕获当前两色，隔离预览固定红/蓝示例沿Rust同一程序。解除库package临时拒绝。

实际检查：

- Qt Release构建和生成头严格检查通过。
- `colorDynamicSettingsUseCapturedForegroundBackgroundAndRoundTrip` 在DPR1/2通过：真实分类/开关/比例输入/关闭编辑、频率、坏模型/越界/种子及旧版本原子拒绝、默认归零/切换复制、混合目录和批量导入、完整模型/种子VBR与重开；真实预览变化；起笔后两色改绿并关闭模块，本笔仍保持红蓝多色/绿色为零、25%透明度上限、精确撤销重做。
- 相关Qt：形状抖动、散布（均新增关闭字段仍可编辑断言）、压力曲线、图片交换（新增启用颜色与红/蓝工具输入，删除来源后新目录真实像素一致）、ABR来源范围（新增关闭颜色及seed，来源基线保持）、实时数位笔姿态，共7个不同UI测试通过，无QML警告。日志本地 `artifacts/color-ui-tests.txt` / `color-ui-final.txt` / `color-ui-dpr2-final.txt`，不提交产物。
- `paint-ffi --test async_session background_prepared_begin`：1项通过。新背景DTO size/reserved/NAN/NULL拒绝且sequence归零，调用方两色修改及库销毁后真实异步笔触保持；旧入口白色与新入口蓝色生成不同实际帧。既有准确修订/库销毁回归 `prepared_preset_begin_uses_exact_revision` 1项通过。
- `paint-ffi --test brush_source`：7项通过，颜色公开包读写保留模型/完整seed/关闭值、caller尾部不改，旧来源/压力/设备/散布/形状路径保持。
- 三包all-targets clippy（`-D warnings`）、Rust格式化通过；C/C++静态/共享ABI smoke 4项通过，COLOR_DYNAMICS位及24字节背景布局检查。

未跑完整 `tools/check.py`，实体设备/跨平台与本模块专项性能未验证。颜色模型仍是明确的linear-hsv-v1，不宣称PS感知HSV一致；VBR不含用户工具两色。运行时随机序号不持久化。ABR动态映射、纹理/双笔刷/平滑收尖/湿混和完整格式冻结继续后续，不代表完整系统验收完成。
