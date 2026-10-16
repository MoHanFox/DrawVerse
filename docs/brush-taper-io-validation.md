# 起收笔预设与交换验收

2026-10-16，Windows。对应[设计](brush-taper-io-design.md)，代码已由用户临时保存为d340ee7，本次补齐发布验收，不改写该提交。

BrushSettings/BrushDefinition/BrushProgram捕获完整起收笔，普通Rust起笔和配置湿混入口自动采用；显式起笔参数仍可覆盖。state18、VBR1.22/behavior15及stroke-taper能力保存启用/关闭的两端长度、四条独立曲线、最低比例和插值方式。默认省略，旧状态/包不升级；混包按全部需求取最高版本。f64范围、曲线端值与顺序先验证，再转换f32，舍入导致节点重合也拒绝。ABR来源记录维持旧模型，不能将新起收笔写入旧基线；用户当前配置可以独立附带原来源。

实际最小验证结果：

- 新增6项IO测试通过：启用/关闭与普通/双路径双色/湿混组合的真实两笔像素和载色、单笔历史及撤销重做；state和VBR往返、持有不可变修订后编辑目录、旧版本默认字节保持、混包最高版本和降级拒绝、严格字段/null/舍入前范围与节点端值、公开示例执行。图片/纸纹与ABR原来源在全新资源目录重开后像素一致，原包及来源记录字节保持，取消保存不覆盖原文件。日志`artifacts/brush-taper-io-final-tests.log`。
- 相关旧回归12项通过：平滑交换3项、湿混交换4项、图片VBR4项、v7来源与七代混包像素/字节1项。日志`brush-taper-io-tests.log`及`brush-taper-io-source-regression.log`；前一个日志包含当时5项新IO用例，新增公开示例和修订持有检查后的最终6项以final日志为准。
- 新增1项原生保护测试通过，分别覆盖启用和关闭配置、普通及wet准备入口、两代库包读写与普通写入口，全部Unsupported，不采用目录也不覆盖已有文件。日志`artifacts/brush-taper-io-native.log`。现有ABI1.50仍不宣称支持起收笔，Qt镜像尚未接入。
- Schema `contracts/vbr-taper-v1.22.schema.json`及三份公开示例已验证：启用/关闭通过、越界拒绝，另核对旧湿混/关闭双色/设备来源、舍入前端值和湿混+起收笔组合。JSON Schema说明跨节点严格顺序及f32重合由Rust语义验证。日志`artifacts/brush-taper-io-schema.log`。
- workspace所有目标编译、paint-core/paint-io/paint-ffi所有目标Clippy（-D warnings）通过；Windows Release及生成头字节检查通过，4项C/C++ static/shared ABI通过。Qt6.8.3 windows平台、software Quick、环境倍率1的现有双色绘画/保存交换和湿混跨文档持久化/VBR两项通过（含初始化清理4 passed、0 failed）。日志`brush-taper-io-{check,clippy,release,abi}.log`及`brush-taper-io-qt.txt`。未跑无关全量或跨平台验收。

命令使用现有本地工具链包装器，主要为`cargo test -p paint-io --test brush_taper_exchange --locked`、相关文件/函数过滤测试、`cargo test -p paint-ffi --lib old_native_entries_reject_taper --locked`、三crate Clippy、既有CMake Release和ABI CTest。构建日志记录实际工具与命令；无新增依赖或C ABI版本变化。

当前完成范围是Rust预设和保存交换，尚无起收笔设置控件。Qt镜像/重置基准、明确支持state18的新原生准备与库包入口、真实白色预览参数接入仍待下一阶段；旧入口保护届时按支持范围解除。完整ABR/16位、湿混双色/橡皮、非分隔、VBR分组和整体验收继续后续，完整笔刷目标未完成。Windows开发版仅在确认应用未运行后同步构建。
