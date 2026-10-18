# 形状抖动原生Qt验收

2026-10-14，Windows 11 / MSVC / Qt6.8.3。ABI1.40、state9/VBR1.7/behavior6/shape-jitter，无DTO变化。设计见 [入口](../../../brush/ui/brush-shape-jitter-ui-design.md)。

完成：形状动态设备/随机子页、独立启用/大小与下限/角度/圆度与下限、通用紧凑输入滑动条和1秒提示。Qt只展示校验，真实预览和绘画沿统一Rust程序；完整种子字符串/关闭值/复制/恢复、最高目录及批量导入版本保持。解除库package临时拒绝，UI启动至少ABI1.40。

实际检查：

- Qt Release构建及生成头严格检查通过。
- `shapeJitterSettingsPersistExchangePreviewAndCaptureRealPixels` 在DPR1/2通过：真实分类/子页/开关/大小输入，关闭参数和完整种子恢复、略越界和错误种子/旧版本原子拒绝、默认开关恢复不升级、后行旧预设不降级目录/批量导入、复制切换及VBR与重开；预览变化、起笔后关闭仍按捕获配置生成真实路径外覆盖、40%透明度上限和精确撤销重做。
- 相关Qt回归通过：散布设置、设备姿态、压力曲线、图片散布交换（新增启用的形状抖动，删除原始资源后新目录真实完整笔触逐像素一致）、ABR来源范围交换（新增关闭形状参数与完整种子/重开，同时保留完整来源基线）、实时数位笔光标。共7项不同UI测试，QML警告检查通过；本地日志 `artifacts/shape-ui-tests.txt` / `artifacts/shape-ui-dpr2.txt` 不提交产物。
- `cargo test --manifest-path core/Cargo.toml -p paint-ffi --test brush_source --locked`：6项通过。公开形状包读写保留完整种子及关闭参数、caller未用尾部保持、既有来源/压力/设备/散布路径保持。
- 三包all-targets clippy（`-D warnings`）与Rust格式化通过；C/C++静态/共享ABI smoke 4项通过，新增SHAPE_JITTER feature断言。

未跑完整 `tools/check.py`，实体设备/跨平台及本模块专项性能未验证。随机印章不移动输入光标；运行时随机序号不持久化。ABR动态映射、随机颜色、纹理/双笔刷、平滑收尖、湿混与完整格式冻结继续后续；不是完整笔刷系统验收。
