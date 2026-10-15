# 湿混 Qt 参数与真实绘画验收

2026-10-15，Windows/MSVC、Qt6.8.3 Release/软件QtQuick，核心ABI1.50、state17/VBR1.15/behavior14。本阶段仅运行对应Qt与ABI回归，未运行完整tools/check.py，也不以单元测试耗时宣称湿混性能达标。

- wetModelPreservesClosedPolicyResetBaselineAndExchange通过：完整11字段映射、默认关闭和默认旧state2、关闭非默认state17保留；副本独立基准/重置/恢复/导出导入；behavior14及wet-mix能力；未知模型/缺失字段/非布尔/舍入前越界、state16降级原子拒绝；有效湿混与有效双色双向拒绝，关闭组合与灰度覆盖允许。默认比对按原生f32语义归一化，避免默认0.1回传噪声误升级状态。
- wetSettingsPageManualInkAndPreviewStaySeparateFromCanvas通过：实际点击新分类/通用复选框，点击数字文本得到焦点并键入15%；窄面板滚动和手动按钮；载入红色后切前景蓝色，主文档仍画红且alpha约0.5；清洗后空载无拾色的一笔不新增历史，手动重载蓝色后实际蓝色像素及undo/redo逐图保持。8位显示通道使用量化容差，不削弱颜色来源和历史验证。
- 同用例验证真实独立红蓝底样；提高湿度/拾色/混合并开启自动载色后，预览改变且紫色混合像素比基准多20以上。还原策略后的实际画布仍使用原手动红色，证明预览没有清洗/染色主工具；全部预览不增加主文档历史。预览固定正常模式，从清洗后的私有载体开始，按预设自动载色规则试画，不读取主工具的保留颜色。
- wetPolicyPersistsSharesAcrossDocumentsAndImportsFromVbr通过：同一BrushCatalogConnection的独立文档共享载色，关闭载入方后另一文档仍画出保留红色；完整关闭策略/独立resetState基准在持久重开后保持；新库包接口导出及新目录导入VBR恢复全部湿混参数。载体本身不保存进预设/文件，重新启动是空工具。
- 三项新增用例及原dualColorClosedValuesHighestVersionResetAndPreviewTokens、colorDynamicSettingsUseCapturedForegroundBackgroundAndRoundTrip、outlineButtonsPressureCaptureCrossingsExchangeAndHistory共6项Qt功能用例通过，日志artifacts/brush-wet-qt-final.txt（含初始化/清理共8 PASS）。完整新参数页/真实像素/预览用例另在QT_SCALE_FACTOR=2通过，日志brush-wet-qt-dpr2.txt（含初始化/清理共3 PASS）。无QQmlEngine警告。截图brush-wet-qt.png与brush-wet-qt-dpr2.png已人工查看，标题、数值、复选框与纵向按钮均在窄参数列内。
- Release应用/测试构建和构建内生成头--check通过，日志brush-wet-qt-build.log；静态/共享C/C++四项ABI smoke通过，日志brush-wet-qt-abi.log。未修改Rust模块或提高ABI/文件版本。Qt部署工具仍提示SDK缺translations/catalogs.json，--no-translations构建成功。

“画笔设置→湿混与载色”提供六个0..100%比例、启用/所有图层/每笔自动清洗/每笔自动载色，以及清洗/载入前景色按钮。关闭时参数仍可编辑。保持来源与兼容页唯一重置按钮，分类索引末尾追加不挪动旧页。

主工具由共享连接持有、所有主文档会话绑定、最后worker停止时释放；预览专用工具不共享。手动操作捕获ID/准确准备序号/颜色/大小，先等待前序编辑完成，再检查占用和准备，异步结束；处理中禁止本会话起笔，3秒超时报告错误，关闭时取消而不冒充成功。当前Qt改用显式wet准备和库包入口，旧C ABI接口仍保留Unsupported保护。

本轮曾发现并修复窄面板横排按钮撑宽导致输入框被裁剪、预览底样命令缺模式两个问题；不保留临时调试输出。标题分行/复选框换行沿用通用复选框指示器。

仍未完成：湿混+有效双色、湿混Erase/Smudge尚明确拒绝（界面提示先关闭湿混）；收尖、非分隔绘制、高级ABR动态/16位及VBR整体扩展、完整设备/跨平台和性能验收继续后续。本轮未单独进行Qt湿混与图片/纸纹/蒙版复杂组合的端到端交互矩阵，其核心/IO/原生组合证据见前阶段文档；不据此标记完整笔刷系统完成。
