# 真实ABR对齐与语料验收

2026-10-15。修复ABR8BIM分节载荷后的四字节填充识别，14行生产读取器变更；原有无填充形式、记录内部对齐/覆盖方向、来源原字节及各版本转换不变，无C ABI/state/VBR版本变化。

## 真实来源与发现

在ag-psd与GIMP 16位原始采样分支发现字节序/量化分歧；现有检查文件均8位，因此16位仍明确拒绝。见[调查](abr-16bit-investigation.md)，不把本阶段当作16位完成。

真实Photoshop导出ABR的phry末尾、desc节间存在长度字段未包含的对齐填充，旧读取器失败。新增三个未修改的许可资产，见[NOTICE](../core/crates/paint-io/tests/fixtures/abr/NOTICE.md)：

- Myer Settlement Brushes：作者K. M. Alexander明确CC0；从作者ZIP核对ABR字节与上游镜像一致。148个8位RLE图片笔尖，901268字节，SHA256 4cb12679582ee908ed8f546d7b45071a2de48fb9af194e649a227f259838f1d6。
- Patchy两个自作者Photoshop2026探针：保留MIT许可证、原NOTICE与固定提交来源，两个各1个8位RLE图片笔尖，2944/3476字节；各自SHA固定在NOTICE和实际Rust回归中。第一个覆盖末尾填充，第二个还覆盖desc节间填充；双笔刷等未支持字段仍按原转换器逐字段报告。

所有资产标记Git binary，禁止文本换行变换。网络下载不是测试前提，常规测试直接读取仓库内许可资产。ag-psd原有资产许可未涵盖图片/笔刷，仍不提交。

## 最小相关验证

- `artifacts/abr-real-corpus-core.log`：paint-io 22项通过（real_corpus1、sections4、sampled3、abr_import4、vbr_source10）。三个真实文件完整解析/关联/导入，全部150个预设逐项比对图片覆盖、名称/索引及原ABR字节；代表笔尖真实绘画/孔洞/像素/undo/redo。生成回归覆盖1/2/3字节填充、非零填充、旧无填充、载荷元数据逐字节一致、错误边界/签名/多余尾随及原有raw/RLE/取消。
- 分节没有总长度/节数封套，完整载荷或其精确可选填充位置结束的前缀可能本来就是合法文件。测试明确枚举这些合法结束位置，仍拒绝跨载荷、非边界和半段填充，不把无法判定的合法前缀宣称为可检测截断。
- `artifacts/abr-real-corpus-external.log`：显式ignored外部诊断1项通过。更新此前旧szVr未执行断言，实际核对新v3尺寸/opacity笔压、满压力参考PNG alpha逐像素一致、半压力alpha上限/缩小范围以及来源保存；原资产SHA及参考PNG SHA保持。它仍不是PS完整笔触渲染对照。
- `artifacts/abr-real-corpus-ffi.log`：公开ABR读取/大小查询不写资源/整包发布/准确准备/来源与损坏后状态保持4项通过，新增真实148项整包。
- `artifacts/abr-real-corpus-clippy.log`：paint-io及paint-ffi all-targets、-D warnings通过。
- `artifacts/abr-real-corpus-build.log`：Release及生成头字节检查通过；`artifacts/abr-real-corpus-abi.log`：C/C++ static/shared四项通过。
- `artifacts/abr-real-corpus-qt.txt`：三个相关功能通过（含初始化/清理5 passed，0 failed）。真实148项用户入口导入、首末选择、实际绘画/历史、选中笔刷带来源VBR导出、删除输入文件后新目录导入的完整来源身份与逐像素一致；旧ABR整包和压力导入/重置/交换回归通过。

未运行无关全量检查。许可真实8位语料门槛取得实质进展，完整16位/旧ABR参数/高级动态、授权PS完整笔触参考、湿混双色/橡皮、收尖/非分隔等仍待后续；完整笔刷系统目标未完成。
