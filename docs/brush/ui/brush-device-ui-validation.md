# 设备动态Qt/原生入口验收

2026-10-14，ABI1.37 / DEVICE_TIP_DYNAMICS=4294967296（u64），DTO不变。Qt接state7/VBR1.5/behavior4，旧package临时拒绝已解除。形状动态提供角度来源、倾斜圆度和最低比例，关闭保留编辑数据，默认回退不升级旧状态。每预设配置随复制/选择/持久化与原生修订；Qt不执行几何算法。

最小实际验证：

- Qt `deviceDynamicSettingsSaveExchangeAndExecuteCapturedGeometry`：鼠标点击角度选项与圆度开关、关闭保持最低比例、复制、坏setter/旧state6保持状态、默认配置回到旧版本、原生VBR往返通过；旋转开启与关闭得到不同的真实预览PNG，实际90°设备输入写出旋转椭圆像素，起笔后改配置不影响已捕获笔触；关闭并重新创建客户端恢复最低比例/角度来源/大小。没有用QML模拟笔触。
- `pressureCurveEditorPersistsAndExchangesNativeCurves`、`proceduralBrushTipPersistencePreviewCursorAndPixels`、`vbrImagePackageImportsIntoFreshStoreAndExecutesAfterOriginalRemoval`、`vbrSourceScopeAndFreshStorePreserveOriginalAfterCurrentTipChanges`共4项旧Qt回归通过；合计5项，记录在本地 `artifacts/brush-device-ui-tests.txt`，无QML警告。
- FFI `brush_source` 4项通过，设备包原生读写与恢复Direction配置实际成功，剩余caller缓冲保持；旧压力与完整来源交换继续通过。
- Release构建及生成头检查、核心/IO/FFI all-target clippy `-D warnings`、C/C++静态/共享4项ABI smoke通过；C测试检查超过32位的新能力标记。

未运行全量tools/check.py、真实数位板/跨平台/性能基准。方向与倾斜的核心/IO像素范围见前两阶段验收，当前Qt新增实际绘画测试专门覆盖旋转通路，不能称已覆盖所有设备交互。画布轮廓仍用基础几何，下一阶段由原生pose查询接实时动态光标；不把本轮设置入口称为完整笔刷系统完成。随机、纹理、散布、双笔刷、颜色、平滑、湿混、ABR动态与分组扩展仍继续。
