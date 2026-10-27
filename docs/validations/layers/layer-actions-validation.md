# 图层操作验收

## Ctrl+X门禁冻结（2026-10-28）

89e20f6续接，原三份在途文档保留。新增真实快捷键剪切普通层/组及选区内容，验证跨文档粘贴、组蒙版/父子去重、选区外像素不变、一次历史/精确撤销重做；锁/无选区/显式蒙版拒绝不修改文档和旧剪贴板，文本焦点继续原生剪切。核心/公开ABI及真实QML两后端定向、四ABI消费者/生成头/相关Clippy/Release为门禁；产物存artifacts/layer-cut，性能与全套未测，不改变文档格式。

- Rust：组与蒙版复制、跨文档粘贴、ID 独立、编辑不串像素、原子 undo/redo（含磁盘历史）；单层向下、多选子树、组合并；锁与损坏剪贴板拒绝且状态保持。
- Rust/Qt：拖到组/图层下边缘，组内目标继承父组，自己边缘无变化、真实循环拒绝。
- Qt：右键分支、主菜单、缩略图/蒙版焦点、快捷键及文本输入不被拦截；只运行对应最小回归和构建。

执行结果（2026-10-12）：

- `cargo test --manifest-path core/Cargo.toml -p paint-core --test layer_actions --locked`：3/3 通过；复制包含组/蒙版、跨文档、独立编辑、损坏数据与父锁拒绝、磁盘历史 undo/redo，单层/组/多选合并，下边缘归组与自身边缘无变化。
- `cargo test --manifest-path core/Cargo.toml -p paint-ffi --test async_session layer_actions_copy --locked`：1/1 通过；FIFO、输入立即复制、跨 session、独立历史与无效字段拒绝。
- Release 构建通过，生成头严格检查及 C/C++ 静态/动态 ABI 链接构建通过。
- Qt `layerContextClipboardFocusShortcutsAndLowerEdgeDrop`：Windows 原生窗口、100%/200% 缩放各通过，无 QML 警告。真实右键分支、Ctrl+C/V/J/E、Delete 蒙版与所有者区分、白色预览框/无行外框、文本输入保护、真实下边缘拖放及目标组继承、主菜单复制组与撤销。
- 依用户要求仅执行最小针对性测试，未运行全量 `tools/check.py`。


## Ctrl+X执行结果（2026-10-28）

产物根目录 `artifacts/layer-cut/`；准确起点 HEAD `89e20f6bf6284dd06ece29d2da1ef6ab31a88300`，分支 `General-purpose-GPU-synthesis`。`native-1/invocation.json`、`working-tree.patch` 保存实际源码差异和四个运行二进制摘要，原三份无关在途文档保留。

- `core-final.log`：新增核心3项通过，覆盖组/蒙版和父子去重、选区外像素与选区保留、单历史和精确撤销重做、锁/无选区/无效ID/显式蒙版失败原子性。`public-2.log`公开ABI1项通过，覆盖命令序号/旧剪贴板、跨session粘贴、失败不覆盖和单历史；`selection-public.log`原选区公开1项通过。
- `native-1/results.json`：软件和D3D11真实Main.qml窗口各8项通过（含init/cleanup）：整层/组/选区实际Ctrl+X、跨文档实际像素粘贴、一次撤销/重做、文本焦点原生剪切、锁定与当前蒙版保护、原选区Delete/Copy。快捷键列表各3项通过（含init/cleanup），原绘制像素/警告断言保持。D3D11实际RTX4060适配器日志保留；不是实体数位板或性能量测。
- `consumers.log`：四个C/C++静态/动态消费者全部通过，实际执行action6/7常量检查、剪切最后图层到空文档、有界读取剪贴板、粘贴回空文档与历史深度检查。`header-final.log`生成头检查、`fmt-final.log`、`clippy.log`相关paint-core/ffi all-targets严格Clippy通过。
- `build-2.log`构建/部署Release应用和相关Qt/四ABI目标通过；标准`build/bin/Release/drawverse.exe`更新。`release-smoke-1/result.json`及`qt.log`记录该标准程序在D3D11/RTX4060上启动、绘画生成及退出0，GPU开关显式开启的冒烟不替代Canvas合成证据。EXE SHA256 `1b24b9284af32be0fd03eb3b42f5d8f7608e02a4eb93f115aea6cb87a4f29cd9`；DLL `7e4bbfc3088327dfd4685b7f0e8e9861fd78c563146c9d4f753eabff06cecbef`。

保留失败：`public-1.log`初版新增测试误把sticky last_error当作每条命令清空；按原session契约改为保留失败sequence、核对成功命令sequence/剪贴板/历史，`public-2`通过。扩大相关回归的`actions-regression.log`中layer_actions3项通过，layer_batch12通过/1失败：`subtree_delete_normalizes_overlaps_and_failures_are_atomic`第278行仍期待拒绝删除全部图层。对未修改HEAD的Git archive源码实际运行，`baseline-delete.log`同样失败；该旧断言及源码未修改，不报告相关回归全绿。基线编译仅作归因，不部署替换当前标准运行时。

ABI1.84仅新增layer_action种类6/7；CPU文档/事务/历史权威与文档格式、state45、composition version1及现有GPU支持范围保持。未运行完整`python tools/check.py`，未新增性能、实体输入、真实设备故障或G5验收。G4组/祖先蒙版及剪贴闭包仍按通用GPU合成验收继续推进；本节的“剪切”不是GPU剪贴组支持。
