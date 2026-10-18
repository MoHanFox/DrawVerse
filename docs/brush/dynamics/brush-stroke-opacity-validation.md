# 独立笔刷尺寸 / 整笔不透明度验收

2026-10-14，用户明确要求覆盖此前共享尺寸断言。预设 radius 现在是实际大小；切换细线圆笔恢复 3px 半径，ABR 导入恢复其 4px 半径；编辑、复制、恢复及原生起笔保持一致。画笔与橡皮使用活动预设自己的尺寸；旧 toolRadii 校验后不再作为执行来源。

普通画笔和擦除的低不透明度现在限制整笔覆盖，流量保留印章累积。核心测试以 1% 直径间距往返绘画验证 10% 上限和第二笔 19%，低流量实际起笔 1% 后渐增；验证擦除、选区、蒙版、单瓦片驻留分页、取消及撤销重做。涂抹强度未改。既有 opacity / VBR dabOpacity 对普通笔触采用新规则，旧历史像素不重算。

数位笔按下与移动经真实 Qt 事件路径更新光标，鼠标移动/悬停在数位笔绘画期间不覆盖其位置；释放后鼠标恢复。覆盖窗口与 item 递送。未使用真实数位板硬件验收，不将合成输入回归等同硬件测试。

通过的最小检查：

- `cargo test --manifest-path core/Cargo.toml -p paint-core --test brush_opacity --test brush_pressure --test brush_program --test brush_mask --locked`：16 项；新增第三项 opacity 测试另行运行全部 3 项通过。
- `cargo clippy --manifest-path core/Cargo.toml -p paint-core -p paint-io -p paint-ffi --all-targets --locked -- -D warnings` 通过。
- Qt Release 构建与生成头检查通过；6 项相关 Qt 测试通过，见本地 `artifacts/brush-opacity-ui-tests.txt`，包含压力编辑入口、独立尺寸、实际绘画、ABR 和数位笔路径。
- C/C++ 静态/共享 ABI smoke 共 4 项通过。

未跑完整 `tools/check.py`，不声称全量通过。现有压力控制是尺寸/流量；独立不透明度压力开关和曲线尚未实现，其余动态/湿混按总体计划继续。
