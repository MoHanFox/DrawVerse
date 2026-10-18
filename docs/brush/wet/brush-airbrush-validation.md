# 时间喷枪验收

2026-10-18，Windows，对应 [设计](brush-airbrush-design.md)。本阶段完成预设设置、state23/VBR1.29、ABI1.57、Rust 定时沉积和 Qt 计时器；不运行全量 `tools/check.py` 或跨平台验收。

- `paint-core --test brush_airbrush` 2项通过：40ms未到50ms间隔不沉积、50ms和100ms各追加一次且整笔一条历史；禁用、移动点和涂抹模式分别拒绝。
- `paint-io --test brush_airbrush_exchange` 1项通过：`time-interval-v1`完整保存到state23/VBR1.29/behavior18/`airbrush`能力，重开读取原值；旧VBR1.28和state22明确拒绝。
- `paint-core --test brush_dual_color` 13项与相关既有回归保持；双色/湿混/涂抹行为未被喷枪路径改变。
- 严格Clippy通过；Windows Release构建、cbindgen头一致性及四个C/C++ ABI烟测通过。
- Qt `airbrushTicksWhileStationaryAndStopsAtStrokeEnd` 通过：按住不动约140ms后低不透明度像素实际加深、结束仍为一条历史、state版本23；已有双色Qt三项回归通过。
- 设置页在“传递”分类提供“喷枪”和1–1000ms间隔；预览仍固定白色/正常模式，定时器只在真实绘画期间运行。

边界：喷枪只支持干画笔和橡皮；普通涂抹、湿混和手指绘画涂抹不启用时间喷枪。喷枪 tick 复用文档事务和历史，不单独增加历史命令。双色喷枪的累积色量仍按双色场规则，不宣称与单色喷枪有相同逐tick加深曲线。证据在`artifacts/airbrush-core-tests.log`、`artifacts/airbrush-io-tests.log`、`artifacts/airbrush-qt-build.log`及Qt测试结果。
