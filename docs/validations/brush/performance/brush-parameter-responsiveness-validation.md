# 主线 A · Brush Parameter Responsiveness 验收

2026-10-24。实际产品基线为 C3 PR #10 的 merge commit
`a7fb6773cce94769a07bc7a1a6bf274bc7088ca3`；分支
`perf/issue-2-brush-parameter-responsiveness`。本轮只对应 Issue⑤。

## 修改前证据与范围

默认4/固定256预设规则复用 A：反复选择 round-pressure，再用合法 saveCopy 创建
issue2a-NNN 名称，恰好256项，最后选择 round-pressure。真实 Main.qml、1280×800、
512×384画布、实际 D3D11 AMD 1002:1638、DPR1、DPI96，不使用实体手绘板。
控件的窗口事件逐个登记实际对象，发现非计划输入/窗口失活则停止，不吞真实输入。

小样本 `pilot-256-v2`：大小、流量、不透明度各同值/离散/连续3次，共27次输入，
各独立目标与实际保存终值一致；另 pressureOpacity 真实开关一次改变成功。
三类离散同步约1–2ms；每次偏好对象构造约3.5–3.6ms后，目录又构造同样256行
约1.6ms。光标刷新约0.03–0.06ms。异步准备仍发生，不能跳过。
这些少量样本不用于稳定p95或真实资源结论。

首次默认库试跑 `pilot-default` 保留失败：流量轨迹恰落 .605 的 .01 量化中点，
oracle 的浮点舍入方向与控件位置运算不同。测试修订为内部位置略高于中点
（+0.00001），起点.38/终点.83保持，仍独立计算期望；不改产品或阈值。
同时使测试 teardown 的 Hide 不被误标为运行期间干扰，并避免失败后补发未执行终点。
这不解释任何历史学生问题。

新增精确参考测试在未优化代码中失败：
`catalogReusesCurrentPreferenceRowsWithoutChangingState` 观测到额外
`catalogSnapshot.presetRows=1`，期望0；完整目录像素无关，比较完整JSON和原冷路径。
原始证据 `artifacts/issue-2-brush-parameters/duplicate-before.txt`。
这证明重复构造，不证明学生卡顿的全部根因。

设计与独立参考边界见 [设计](../../../brush/performance/brush-parameter-responsiveness-design.md)。
后续测量与最终验收在本记录追加，历史数据保持。
