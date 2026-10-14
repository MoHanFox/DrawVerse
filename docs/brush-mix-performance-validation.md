# 混合与双笔刷性能验收

2026-10-14，无ABI/state/VBR变化（当前ABI1.45/state13/VBR1.11）。设计见brush-mix-performance-design.md。主/第二圆尖保守平方距离快路径；整笔达到当前压力上限且操作稳定时跳过重复混合，独立第二覆盖场达到1时跳过重复采样。纸纹和发生过笔/橡皮翻转的主路径不使用此跳过。间距、预算、选区/AA和事务不改变。

工具详情下拉使用窗口Popup跨原生面板置顶，Qt6.5窗口回退含焦点/Esc/外部点击关闭与父项恢复；普通面板下拉未升级到置顶窗口。预览固定正常模式，删除有色底和额外混合预算。模式改变仅通知工具并持久化，不重建预设列表，不失效预览；跨文档restore按完整当前预设行比较，保留相同定义的正常预览、错误、token与在途请求。修改定义仍换token。

## Release基准

命令：cargo run --manifest-path core/Cargo.toml -p paint-core --example brush_mix_performance --release --locked。512x512画布、有色底、40%整笔不透明度、主/第二1%直径间距、80个相同输入、3次中位数，计时含起笔/绘画/End真实历史，不含Qt视口渲染。前后10组稀疏像素摘要逐项相同；摘要不代替下面的全瓦片回归。

| 配置 | 直径 | 优化前ms | 优化后ms | 比例 |
|---|---|---:|---:|---:|
| normal | 128px | 118.923 | 22.753 | 5.23x |
| multiply | 128px | 183.419 | 22.905 | 8.01x |
| dual-path | 128px | 207.099 | 40.649 | 5.09x |
| dual-paired | 128px | 203.544 | 25.603 | 7.95x |
| multiply-dual | 128px | 244.039 | 43.140 | 5.66x |
| normal | 256px | 234.209 | 30.326 | 7.72x |
| multiply | 256px | 345.002 | 30.897 | 11.17x |
| dual-path | 256px | 362.809 | 55.420 | 6.55x |
| dual-paired | 256px | 394.413 | 37.676 | 10.47x |
| multiply-dual | 256px | 470.160 | 60.957 | 7.71x |

原始证据：artifacts/brush-mix-performance-before.log、brush-mix-performance-after.log。CPU/系统负载影响时间，本轮未测实体数位板输入延迟、复杂图片双笔尖或GPU，不把这些固定输入结果宣称为所有场景倍率。

## 最小对应回归

- paint-core brush_blend 6、brush_dual 9、brush_pressure 7项通过，含复杂图片/纹理/颜色/选择/分页与四类操作；饱和跳过参考回归开启测试专用旧路径，27模式×3目标×3双笔刷路径，比较所有真实图层/蒙版瓦片和撤销/重做，不仅比较合成预览。
- 圆尖保守边界对原hypot公式的硬度/大小/边缘逐值比较通过；paint-core全targets Clippy -D warnings通过。
- Qt toolComboDropdownStaysAboveNativePanels在DPR1/2的窗口/强制Qt6.5回退均通过：真实浮动面板覆盖彩色底、屏幕合成像素必须为菜单背景、Esc/重新打开/选择；截图brush-mix-dropdown-1/2.native/fallback.png人工检查。不是只检查z值。
- Qt模式模型与真实笔触/预览/捕获/共享持久化通过；正常预览缓存跨模式相同，主动重生成仍相同，真实混合/历史保持。DPR2另含普通图层混合悬停（停靠/浮窗）、预设独立值/旧请求拒绝、双笔刷真实资源与历史、涂抹和魔棒回归，共10项（不含init/cleanup）。Release Qt与生成头检查通过。

未运行tools/check.py全仓，不报告完整检查通过。双色复合/自动描边、时间平滑IO/原生设置、湿混与ABR高级映射继续后续。
