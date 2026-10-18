# 重置预设验收

2026-10-14，设计见 [brush-reset-design.md](../../../brush/ui/brush-reset-design.md)，无ABI/执行state/VBR变化。右侧参数列表末尾、固定笔触预览上方的重置预设恢复完整基准，内置出厂/创建副本时/导入成功时配置分别保存；旧自定义以首次记录的有效配置为基准。身份和名称、工具颜色/光标、文档历史保持，基准不随原生目录规范化丢失。

独立resetState持久元数据经已有严格模型验证，当前catalogSnapshot不包含它；当前和基准各2MiB、完整偏好4MiB+16KiB、条目1024限制，递归基准拒绝，坏输入不覆盖有效镜像。删除同时删除基准。参数发布、失败恢复、资源重新验证、后台持久化和真实预览仍走既有原生修订桥。

Windows 11 / Qt6.8.3 / MSVC / 软件Qt Quick，Release构建与生成头检查通过，相关最小Qt Test：

- `resetPresetRestoresFactoryCopyAndImportedBaselinesAtomically`：工厂及完整动态重置、复制/导入/旧状态迁移、重开模型与原生规范化、非法递归基准原子拒绝、删除清理、基准不进入执行目录。
- `resetBaselineSurvivesNativePreferencesReopenAndCapturedStroke`：真实后台配置保存后关闭/重开、恢复创建时8px半径；当前3px笔触在重置后仍绘出3px范围，下一笔使用8px；重置不生成画布历史。
- `dualSettingsImportPreviewPixelsHistoryAndExchange`：真实点击重置按钮恢复第二PNG与大小，不改变已有历史；其他双笔刷真实像素/资源交换回归通过。

上述最终DPR1测试共3项通过；DPR2双笔刷、纹理、压力曲线、工具选择共4项通过，DPR1另有双对象状态回归。截图 `artifacts/reset-ui-2.png` 已检查，窄列按钮可见且参数/固定预览保留；无QML警告。git diff --check通过。Rust相关回归与clippy见 [平滑验收](../dynamics/brush-smoothing-validation.md)。未运行全量tools/check.py或跨平台/实体数位板验收。

双色与自动描边的新字段仍待实现；新增字段必须进入同一完整基准。VBR导出仅交换当前预设，重新导入以包定义建立基准，不宣称原重置基准随VBR交换。

按钮移位补充：dualSettingsImportPreviewPixelsHistoryAndExchange在DPR1/2真实滚动并点击参数末尾的重置，恢复资源/半径且历史不变；assert按钮低于参数、高于固定预览并完全位于可视区域。见brush-pressure-smooth-validation.md。
