# VBR分组清单与副本映射验收

2026-10-17，Windows，对应[设计](brush-groups-vbr-design.md)。VBR1.24可保存ordered-tree-v1组织树，包括嵌套、空文件夹、同名组、稳定身份和顺序。旧groups槽仍必须为空；目录元数据不改变笔刷behaviorVersion或能力声明。ABI仍1.53，state仍20。分组的完整原生交接、目标库原子合并及Qt管理界面尚未接入，旧四类原生包读入口明确拒绝有组包，不宣称用户入口完成。

实际验证：

- 最终四个IO目标共33项通过：vbr 4、vbr_image 8（含新分组4）、vbr_source 14、brush_wet_dual_exchange 7。覆盖程序/图片笔尖、空组/嵌套/同名与顺序、规范化平组织旧字节保持、实际writer清单与契约合法例一致、旧minor/未来版本/null/错误模型/引用顺序和旧groups冲突拒绝。公开Package被改坏后保存失败且现有目标文件保持。
- 副本映射要求完整且新旧身份分离，包/预设/组之间不能撞ID；缺项、多项、非法UUID、重复、旧ID复用与取消均不改变原包编码。成功映射只改元数据，保留资源Arc和PNG字节、名称/顺序及完整参数，重开树保持。目标库冲突检查仍待后续导入事务，当前不是完整用户导入。
- 既有真实ABR6/2来源与三图片用途交换用例增加有组分支：写包/重开/整树映射后再重开，源ABR及来源记录字节保持；真实两笔像素、全部双载色格点与undo/redo历史匹配。资源/来源装入新目录后状态恢复仍匹配。此用例包含在上述7项中，不重复计数。
- 新原生保护1项通过：空文件夹和有成员组，两种容量分别调用四个旧包读入口，返回Unsupported/required=0且输出哨兵与资源目录保持；四个旧写入口拒绝新增organization根字段，原目标文件保持。写入保护测试使用organization:null验证严格字段拒绝，不冒充完整分组写入API。
- [VBR1.24 Schema](../../../contracts/vbr-groups-v1.24.schema.json)经Draft202012Validator检查，[合法例](../../../contracts/examples/vbr-groups-valid.json)通过、[降级非法例](../../../contracts/examples/vbr-groups-invalid.json)拒绝。Schema描述结构和列表上限，树可达性/全局唯一/深度及预设投影仍由Core和IO检查。
- paint-io/paint-ffi全目标Clippy（警告视为错误）通过。Windows Release、4项C/C++ static/shared ABI及旧Qt两项包导入/真实绘画与重开回归结果见下方最终记录。未进行整体设备手感或跨平台验收。

首轮原生测试错误引用未依赖的serde_json，改为直接构造附加字段的JSON字节后编译/测试通过，未新增依赖。最终验收用例采用真实writer清单与独立契约文件对照；构造器补充完整验证后重新运行上述目标和Clippy。

本机日志：artifacts/brush-groups-vbr-{final-tests,native,clippy,schema,release,qt-console,abi}.log及brush-groups-vbr-qt.txt，不入库。

最终Windows Release构建成功，4项ABI全部通过。Qt的vbrImagePackageImportsIntoFreshStoreAndExecutesAfterOriginalRemoval和wetDualImageVbrFreshStoreAndReopenMatchCapturedPixels通过（含init/cleanup为4 passed）；实际测试仍是旧平包入口，不作为分组Qt支持的证据。
