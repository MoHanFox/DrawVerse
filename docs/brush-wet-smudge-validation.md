# 二维湿混涂抹验收

2026-10-15。Rust 核心与原生会话复用已有二维载色，Qt 独立涂抹使用同一预设和 prepared 入口，无 ABI/state/VBR 版本变化。默认关闭湿混的旧笔刷与涂抹路径保持。

## 核心与原生

- `cargo test --manifest-path core/Cargo.toml -p paint-core --test brush_wet_smudge --locked`：6 项通过，日志 `artifacts/brush-wet-smudge-core-fix.log`。检查手指开关/前景色、强度×流量×压感只乘一次、保留载色与透明搬运、当前位置拾色后跨瓦片拖出颜色、undo/redo、取消及非法输入回滚、工具取样优先级/只写目标、透明锁/零强度仍拾色、选区外取样、蒙版灰度往返与非法合成取样拒绝。
- `--test brush_wet_document --test brush_wet_settings`：12+7 项通过，日志 `brush-wet-smudge-core-final.log` 中对应两个 suite；该次运行的新涂抹 suite 曾失败，修复后另由上面的专用日志确认通过，不把失败日志作为整条命令通过。图片/纹理/散布/灰度双路径组合扩展为 16 个画笔和 16 个湿混涂抹组合，比较常驻/单瓦片分页与拆分输入的完整真实像素；涂抹组合同时启用颜色动态。原湿混画笔 27 模式等回归通过。
- `cargo test ... -p paint-core --test smudge --test brush_wet_color --locked`：3+5 项通过，日志 `brush-wet-smudge-regression.log`，覆盖旧涂抹和湿混颜色动态原有像素/历史规则。
- `cargo test ... -p paint-ffi --lib wet_ --locked`：8 项通过，日志 `brush-wet-smudge-actor.log`。新增湿混涂抹 actor 真实两笔对照 Document，比较完整复制图层字节、多个二维载色位置、工具采用和精确撤销重做；共用租约的占用/取消/错误/关闭及桥和所有者预算回归通过。最初使用文件名作为 test filter 得到 0 项，已改为实际模块/名称过滤，不作为有效验证。
- `cargo clippy --manifest-path core/Cargo.toml -p paint-core -p paint-ffi --all-targets --locked -- -D warnings` 通过，日志 `brush-wet-smudge-clippy.log`。

蒙版回归曾发现将显示灰度直接当线性 RGB 会改变均匀灰度，已只对新增湿混涂抹分支进行取样解码，保持旧湿混画笔像素；载色采用与前景色一致的线性空间，写回转换可见灰度为隐藏量。均匀内部以 1e-6 浮点容差验证，边缘保留有界载色采样的插值行为。

## Qt 与构建

Windows/MSVC、Qt6.8.3 Release、软件 QtQuick。`qt_ui_tests.exe wetSmudgeUsesOwnToolStrengthAndFingerPolicyWithUndo smudgeSharedBrushParametersInputAndDelayedHelp wetSettingsPageManualInkAndPreviewStaySeparateFromCanvas` 验证真实 Qt→prepared→原生像素：独立涂抹启用湿混，手指关闭不自动带入前景色，开启后强度 40%×压力 50% 得到约20% alpha；起笔后修改工具参数/颜色不污染在途笔触，历史为涂抹，undo/redo 图像保持。旧 R 快捷键、鼠标/数位笔、工具栏/延迟提示及湿混参数页/独立预览回归覆盖。

结果见 `artifacts/brush-wet-smudge-qt.txt`、`brush-wet-smudge-dpr2.txt`、`brush-wet-smudge-abi.log` 与 `brush-wet-smudge-build.log`：普通缩放下 3 项 Qt 功能用例通过，双倍缩放下新增湿混涂抹像素/历史用例通过；4 项 C/C++ 静态/共享 ABI、Release 构建及生成头检查通过。额外运行旧工具栏用例时，双倍缩放的延迟提示可见性断言失败（普通缩放通过），单独复跑仍失败。诊断显示初次 hovered 为 true，延迟后变为 false；尝试对齐系统指针未解决，已撤去临时诊断和无效测试改动。此项原因仍待查，不将整个双倍缩放 suite 计为通过，也不宣称已证明是产品还是测试事件的问题。没有新增布局或图标，不以该轮测试声称实体数位板和跨平台验证。Qt SDK 缺翻译索引的既有警告由 `--no-translations` 处理。

## 使用与剩余范围

在画笔设置启用湿混后选择 R 涂抹；湿混页六比例和清洗/载色策略生效，工具栏仍控制强度、压感、所有图层和手指绘画。自动前景色载入同时要求自动载色和手指绘画；手动/保留载色不因关闭手指而被隐式清除。提示统一 1 秒，预设预览仍为固定正常画笔湿混示例，不随工具切换。

未运行全量 `tools/check.py`；未完成湿混双色、湿混橡皮/翻笔输入、收尖、非分隔、完整 ABR 高级动态/16 位、VBR 整体扩展，以及总体性能/实体设备/跨平台矩阵。该阶段不是完整笔刷系统完成。
