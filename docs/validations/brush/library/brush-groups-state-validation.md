# 分组state20保存验收

2026-10-17，Windows，对应[设计](../../../brush/library/brush-groups-state-design.md)。本阶段完成Rust组织DTO和状态保存恢复：state20/organization/ordered-tree-v1，显式根和组子项、空组/同名组及稳定ID。VBR仍最高1.23，分组包清单、整树副本映射、明确原生入口及Qt文件夹/重启保存尚待接，不能将此结果外推为完整分组用户功能。

DTO与运行内核共用树验证；根/子项最多1280、组记录256，核心限制总条目/8层及唯一成员、可达性和身份。反序列化拒绝未知字段/类型/null和非字符串模型，DFS预设投影必须与presets行序一致，非法组织在读图片或来源前失败。模型不含启用标记，空组仍属于有组数据并保留。state20没有组织或显式无组组织可按实际平参数规范化；有organization不能夹带到旧版本。

有组编码选择20，平目录仍选参数所需最低旧版本，解组及原平状态字节保持；组织字段缺失不引入null或改变旧字段顺序。其他已实现参数及关闭配置、图片三用途与来源读写均复用原算子。四类旧原生准备（Legacy/Wet/Taper/WetDual）在采用前明确拒绝任何有组目录，保留当前状态，尚未给旧Qt开放新数据。

实际结果：

- 新组织状态4项通过：嵌套/空组/同名/完整关闭湿混、平滑和起收笔参数的规范字节往返；版本降级/未来版本、null/未知字段/错误kind/model类型、重复或失联/环和非法名称/ID、行序歧义拒绝；序列化列表1280/256、深度9拒绝；显式无组20规范回旧字节；合法和非法契约示例同时经Rust验证。
- 新真实资源来源1项通过（启用/关闭湿混两组）：实际ABR6/2 fixture导入，主/第二/纹理共用实际PNG、源档案及两组目录/同名/空组/同组副本写成state20；资源和完整来源写入新目录，删除原PNG后decode_captured恢复组织及全部状态字节，来源原始ABR字节保持；两笔真实像素、全部双载色格点及undo/redo历史与原定义一致。此处是状态+资源目录交换，不是VBR分组包。
- 最终IO四目标共20项通过：新组织4、基础状态5（含更新的分组状态用例）、来源状态4、湿混双色交换7（含上述新用例/既有6）。旧1–19参数/来源与VBR1.23边界继续通过，不将交叉提及的用例重复计数。
- 新原生保护1项通过，包含空组/有成员组×四类旧准备，最终任务结果Unsupported且当前目录编码保持；没有假成功采用后再丢组。
- Organization JSON Schema和合法/非法结构例通过Draft202012Validator；Schema描述结构和列表上限，环/成员全局唯一/总条目/深度和实际预设集合仍由Core验证。见[Schema](../../../../contracts/brush/library/brush-organization.schema.json)、[合法例](../../../../contracts/brush/library/examples/brush-organization-valid.json)、[非法例](../../../../contracts/brush/library/examples/brush-organization-invalid.json)。
- paint-io/paint-ffi全目标Clippy `-D warnings`、最终Windows Release/生成头检查与4项C/C++ static/shared ABI通过。旧Qt程序预设/持久化/实际预览与图片导入/在途捕获2项通过（含init/cleanup为4 passed）；未跑无关全仓、跨平台或整体硬件验收，硬件手感由用户自行进行。无ABI或VBR版本变化。

复核发现首稿使用Serde单值枚举会接受`{"ordered-tree-v1":null}`对象模型，补充用例明确复现失败，改为严格字符串+值检查，随后重新运行全部相关IO/保护/Clippy/Release/Qt/ABI。Schema工具首轮缺少运行时模块，复用现有ignored `.tools/schema-check`后校验通过，未新增依赖或隐藏失败。

本机日志：`artifacts/brush-groups-state-{basic,tests,source,native,legacy,final-tests,model-type,schema,clippy,release-final,qt-console,abi}.log`和`brush-groups-state-qt.txt`，不入库。下一步将此组织DTO接VBR清单、预设集合/顺序严格校验、UUID副本树映射和资源/来源往返，之后接原生/Qt完整用户入口。
