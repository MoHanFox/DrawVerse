# 散布原生Qt验收

2026-10-14，Windows 11 / Rust / MSVC / Qt 6.8.3。ABI1.39，state8/VBR1.6/behavior5/scatter-stamps；无DTO布局变化。设计见 [散布入口](brush-scatter-ui-design.md)。

完成：PS左侧散布分类，启用/双轴/0–1000%偏移/1–16数量/0–100%数量抖动，通用紧凑输入、滑动条及1秒提示；全部参数沿Rust准确准备，真实预览与绘画共用程序。完整seed字符串、关闭配置、独立预设/复制、后台保存与VBR导入导出保持；目录和多行导入使用最高状态版本，不被后行旧参数降级。默认开关恢复不升级旧状态。解除旧package临时拒绝。

实际验证：

- Qt Release构建和生成头严格检查通过。`scatterSettingsPersistExchangePreviewAndCapturedPixels` 在DPR1/2通过：真实分类/复选框/数量输入，完整seed、关闭值、坏值原子拒绝、默认恢复、混合目录/批量导入、VBR及重开恢复，统一预览真实变化；起笔后关闭散布仍按捕获程序完成，路径外实际像素及40%整笔透明度上限，撤销重做恢复精确图像。
- `scatterImageExchangeKeepsActualPixelsAfterSourceRemoval` 通过：含孔洞图片，散布/seed/透明度包导入到新目录，删除所有原始资源后，真实完整笔触逐像素一致。
- 相关Qt回归通过：`deviceDynamicSettingsSaveExchangeAndExecuteCapturedGeometry`、`pressureCurveEditorPersistsAndExchangesNativeCurves`、`vbrSourceScopeAndFreshStorePreserveOriginalAfterCurrentTipChanges`（加入关闭散布和完整seed，同时保留来源/基线）、`vbrImagePackageImportsIntoFreshStoreAndExecutesAfterOriginalRemoval`、`liveTabletPoseTracksTiltRotationPressureAndCapturedStroke`。7个不同UI测试，QML警告检查通过。日志在本地 `artifacts/scatter-ui-*.txt` / `artifacts/scatter-image-source-tests.txt`，不提交产物。
- `cargo test --manifest-path core/Cargo.toml -p paint-ffi --test brush_source --locked` 5项通过，包括公开原生散布包读写、完整种子和关闭数量保持、未使用caller尾部不变、既有来源/设备/压力路径。
- `cargo clippy --manifest-path core/Cargo.toml -p paint-core -p paint-io -p paint-ffi --all-targets --locked -- -D warnings` 通过，Rust格式化通过。
- C/C++静态/共享ABI smoke 4项通过，新增散布feature位断言。

未跑完整 `tools/check.py`；实体数位板、跨平台及本模块专项性能尚未验证。当前随机落点不移动输入笔尖光标，种子按VBR定义恢复但运行时笔触序号不持久化。随机尺寸/角度/颜色、纹理/双笔刷、平滑收尖、湿混、完整ABR动态兼容与格式冻结仍待后续，本阶段不代表完整笔刷系统验收完成。
