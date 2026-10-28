# 颜色与基础参数增量发布验收（2026-10-26）

设计：[增量发布](../../../brush/performance/shared-settings-publication-fix-design.md)。修复前因果与基线：[调查验收](shared-settings-publication-investigation-validation.md)。本轮基于 HEAD eb0a6316268fc7c110bbba8bbaaa3c55d3cac523 的工作区修复；不把相同 HEAD 误当作相同产品二进制。

## 行为与边界

- `BrushLibrary::update` 对 radius/flow/opacity 只补丁有效快照的执行行及当前工具 profile；冷构造仍保留，reset 不改，原先即时 settings/preset/preview 通知与35ms发布保持。
- `restoreSharedSettings` 扩展已验证模型的选择同步，允许颜色与上述三值。其他内容、ID/顺序、schema、reset/目录和隔离模式精确比较；不同工具按接收方 context 投影。部分隔离的无效旧共享字段沿用既有处理，无变化行保持引用。
- 比较全部成功才应用候选，保留预览、接收方工具与单写入者；候选等于来源时共享来源字节与数组编码缓存。外部字节不能启用此分支，PaintCoreClient 仍核对来源快照与发布字节一致。复杂编辑或不等状态继续完整恢复。
- 无 Rust、ABI1.80/state45、VBR 或 QML 行为格式变更。

## 功能验证

Windows Release、软件和 D3D11 各通过以下定向集，无失败、跳过或 QML warning：

- `qt_color_state_tests`：10 passed（含初始化/清理），8条功能行。新增三档隔离×三种发送工具×三种接收工具共27组合，与独立完整恢复的 JSON/目录/后续工具切换值比较；颜色/三参数组合、零整库通知与零持久化反馈、复杂编辑快路径拒绝、外部非法状态原子拒绝。正式隐藏服务与历史采用初始客户端两种启动行均验证跨工具参数、颜色和保存；原立即起笔实际像素/重开、冷热缓存与混合修改也通过。
- `qt_workflow_tests` 六项：sharedSelectionAcrossToolContextsKeepsProfilesAndPreviews、partialToolSwitchIgnoresStaleInactiveSharedParameters、sharedBrushPreferencesRetainEachDocumentsIsolatedToolWithoutFeedback、brushToolIsolationRoundtripNativeSyncAndInvalidStateAreAtomic、fullPreferenceRestoreReusesRowsWithoutSkippingRoleValidation、brushToolIsolationLevelsPreserveParametersAndSharedCatalog。8 passed 含初始化/清理；512项选择/工具投影与预览、严格角色校验保留。
- `qt_ui_tests` 五项：brushParameterEditsKeepRowsAndImmediateStroke、resetBaselineSurvivesNativePreferencesReopenAndCapturedStroke、batchedBrushPreferencesFlushFromActiveDocumentOnCreateAndQuit、activeAndRecentColorsPersistAcrossRestartAndStayOutOfPresets、colorDynamicSettingsUseCapturedForegroundBackgroundAndRoundTrip。7 passed 含初始化/清理。

日志在忽略目录 `artifacts/shared-settings-20261026/regression-{software,d3d11}-optimized/`。开发中旧断言要求目录必须从偏好对象投影一次，而现在可以直接补丁既有目录对象，已更新为接受两种复用并保留字节/独立冷构造等价断言。一次脚本误指定了属于另一个测试目标的函数，未运行功能测试；更正后全部上述定向集通过，不计作通过证据。

## 连续输入与耗时

沿用调查探针与同一489项源配置/资源摘要、round-pressure、部分隔离档1、正式 Main.qml 启动（包含隐藏服务）。Windows窗口1600×1000，DPR1.5，D3D11日志确认 NVIDIA RTX 4060 Laptop GPU；软件日志确认 software。每后端1/2/3可见文档×4输入×2节奏×8移动=192次，两个后端共384次真实窗口合成移动。不是实体数位板/物理鼠标操作。

最终探针 SHA256 `92b6135364bc904c89df68f80802b76e2f3fb138faa0bc135921e78198a53ace`。最终完整报告：`fix-real-normal-optimized`、`fix-real-software-optimized`，均24场景通过、最终模型/所有接收方与实际 INI 一致、input_invalid=false、trace丢弃0。场景内 `preferences.restore.full`、`snapshotObject.rebuild` 和接收方 `presetsChanged` 全部0；创建文档本身不纳入连续场景。

正常单文档、请求50ms间隔慢速移动时的最大 GUI 心跳间隔（ms）：

| 操作 | 修复前 D3D11 | 修复后 D3D11 | 修复后软件 |
|---|---:|---:|---:|
| 色轮 | 296.818 | 22.144 | 24.503 |
| 大小 | 361.289 | 32.696 | 41.370 |
| 流量 | 366.287 | 31.878 | 33.848 |
| 不透明度 | 381.731 | 38.041 | 50.407 |

单文档请求8ms快速移动后的收尾最大心跳，D3D11由272–354ms变为19–29ms，软件22–31ms。最终三文档慢速段最大心跳仍有D3D11 87.915ms、软件64.026ms；共享对象比较/数组编码/原生目录准备仍有开销，不能称所有文档数量都零停顿。单文档 D3D11 广播槽最大从约271–287ms降为12–24ms。嵌套 spans 不相加。

这是每场景8次移动的一轮最大值，不是稳定p95、输入到显示延迟或跨机器承诺。修复前对照仅采用原报告正常单文档，不能把旧采用初始客户端的多文档结果冒充正常启动基线。中间 `fix-real-normal-1` 在末段检测到非计划输入而失败，保留原始证据；`fix-real-normal-final` 是更早实现的完整通过数据，均不作为最终性能数字。

复现（MSVC/Qt构建环境；每次输出新目录）：

```powershell
cmake --build build --target drawverse_runtime qt_color_state_tests qt_workflow_tests qt_ui_tests qt_brush_interaction_baseline -j 2
python tools/measure_shared_settings.py build/bin/Release/qt_brush_interaction_baseline.exe artifacts/shared-fix-d3d11 --settings artifacts/user-brush-repro/DrawVerse.ini --documents 3
python tools/measure_shared_settings.py build/bin/Release/qt_brush_interaction_baseline.exe artifacts/shared-fix-software --settings artifacts/user-brush-repro/DrawVerse.ini --documents 3 --backend software
python tools/analyze_shared_settings.py artifacts/shared-fix-d3d11/report.json artifacts/shared-fix-d3d11/analysis.json
ctest --test-dir build -R '^abi_' --output-on-failure
```

用户资源/偏好不提交；runner复制仓库已有隔离配置副本，不修改用户设置。未提供配置时可跑默认4项库。

## 交付与检查

Release `drawverse_runtime`、三个Qt功能目标和探针构建通过；Rust Release依赖、生成头一致性和着色器产物检查随应用构建通过，四项 C/C++ 静态/动态 ABI smoke 通过。`git diff --check`、两个Python工具 py_compile 通过。未运行仓库全量 `tools/check.py`，不称全量测试通过。部署工具有已有 translations/catalogs.json 缺失提示；使用 --no-translations 的标准部署仍成功。

标准应用已更新：`build/bin/Release/drawverse.exe`，SHA256 `764675c5fa91520cab5772086a2aa26aa0f05620ddb639428b1be9b54d5546e0`。生成头不变。实体持续输入与首次冷加载不在本轮验收范围。
