# ABR 整笔不透明度验收

2026-10-15。转换器 v2 接通已有整笔 opacity，不修改像素引擎或工具选择；只在新导入实际使用非100% toolOptions.Opct 时产生 v2 来源。未使用新行为的源记录仍为 v1，旧来源不重新转换。来源记录结构仍version1，包含v2来源的公开交换包为VBR1.16；state与C ABI布局/版本不变。

## 有效验证

- `cargo test --manifest-path core/Cargo.toml -p paint-io --test abr_conversion --test abr_import --test brush_source --test vbr_source --test brush_source_state --test brush_wet_settings --test brush_dual_io --locked`：37项通过，日志 `artifacts/abr-opacity-core-final.log`。转换8、整包导入3、来源8、VBR来源9、来源状态4、湿混来源保护2、双笔刷来源3。v1显式入口保留旧基准/报告；0/0.1/25.5/99.999999/100、舍入前越界/NaN/错类型；程序和孔洞图片笔尖真实密集往返25.5%上限、单历史/undo/redo/取消；新旧来源共享原ABR、原记录字节保持、最高minor16及旧minor2/15降级拒绝、真实重开像素一致。发布合法/非法来源样例也由Rust验证。
- `cargo test ... -p paint-ffi --test abr_import --test brush_source --locked`：11项通过，日志 `abr-opacity-ffi.log`。公开ABR缓冲/整包接口和准确prepared状态携带25.5%，来源查询字节与存储档案一致且版本2/基准正确；旧版导入、未发布状态、坏包原子拒绝，以及原来源资源桥/交换回归保持。
- `cargo clippy --manifest-path core/Cargo.toml -p paint-io -p paint-ffi --all-targets --locked -- -D warnings`通过，日志 `abr-opacity-clippy.log`。没有增加产品依赖、修改Cargo.lock或引入新unsafe。
- 以临时已安装的jsonschema4.23.0执行Draft202012Validator.check_schema/validate：公开schema的v1/v2合法来源通过，未知转换器版本及baseline不透明度负值/大于1/字符串共4个非法对象均拒绝。不是仅用JSON语法检查声称Schema有效，也不把它当运行时校验替代品。相关文件在 `core/crates/paint-io/tests/fixtures/schema-history/abr/vbr-abr-opacity-v1.16.schema.json` 和 `contracts/brush/abr/fixtures/abr-opacity-source-*.json`。
- Release构建及构建中的生成头字节检查通过，日志 `abr-opacity-build.log`；C/C++静态/共享ABI四项通过，日志 `abr-opacity-abi.log`。SDK缺翻译索引的既有警告仍用--no-translations处理。
- 原生Windows Qt6.8.3/软件QtQuick、环境缩放倍率1：`qt_ui_tests.exe abrOpacityImportsDrawsResetsAndExchangesWithVersionedSource abrUserImportAddsWholePackageWithProvenanceAndSurvivesSourceRemoval vbrSourceScopeAndFreshStorePreserveOriginalAfterCurrentTipChanges wetPolicyPersistsSharesAcrossDocumentsAndImportsFromVbr`，4项功能用例通过（含初始化/清理6 PASS），日志 `abr-opacity-qt.txt`。新用例验证25.5%导入/实际往返绘画全图alpha上限、来源v2与基准、编辑后重置、精确撤销重做、含来源导出、删除输入后全新目录VBR导入及来源保持；旧ABR/来源界面和湿混IO回归通过。没有新增界面控件，不宣称新高DPI布局或硬件测试。

## 兼容边界

SourceBatch缓存经过验证记录的最高转换器版本，只用于选择包minor，不改写任何记录。v1/v2混合包仍逐记录保留原字节和SHA身份。没有来源数据的新导出继续按实际参数/资源选择旧版包，不为不相关预设一律升到1.16；已有湿混或其他动态参数的行为版本与字段保持。旧转换器仍将非100%标未执行，当前转换器准确标Mapped；Mapped仅指字段执行，不代表整个ABR兼容或PS像素等价。

本轮最初FFI测试尝试直接使用未声明的serde_json失败，改为比较公开返回字节与已有typed来源档案，未新增依赖。fixture扩展后Clippy发现旧fixture入口在一个测试目标未使用，已保留旧入口调用，不以禁用告警解决。最终构建/相关检查通过，无临时调试代码。

未运行全量tools/check.py。完整ABR形状/散布/传递/纹理/双笔刷动态映射、16位和旧参数采样、授权Photoshop真实笔触语料、湿混双色/橡皮、收尖/非分隔、VBR分组及总体性能/设备/跨平台仍继续后续。本阶段只完成ABR基础不透明度接通，不标记完整笔刷系统完成。
