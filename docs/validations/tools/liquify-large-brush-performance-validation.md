# 大尺寸液化采样验收

2026-10-19，Windows Release，见[设计](../../tools/liquify-large-brush-performance-design.md)。不降低采样精度，不扩大2000px、单输入访问和位移瓦片预算。

## 固定基准

`cargo run -p paint-core --release --example liquify_large_performance --locked`：2048×1536固定RGBA图案，每组6次输入，重复3次取中位数；计时包含液化计算，排除建图及最终摘要遍历。修改前后六组RGBA浮点位摘要全部一致。

| 工具 | 半径px | 修改前ms | 修改后ms | 降低 |
| --- | ---: | ---: | ---: | ---: |
| 推移 | 128 | 45.258 | 26.266 | 42.0% |
| 推移 | 512 | 836.571 | 455.041 | 45.6% |
| 推移 | 1000 | 2113.920 | 1491.918 | 29.4% |
| 旋转 | 128 | 32.478 | 28.823 | 11.3% |
| 旋转 | 512 | 683.689 | 501.419 | 26.7% |
| 旋转 | 1000 | 2239.392 | 1734.083 | 22.6% |

原始记录：`artifacts/liquify-before.txt` / `artifacts/liquify-after.txt`。时间只描述该输入及本机，不外推其他工具、文档、硬件或实体输入流畅度。

## 最小验证

- `cargo test -p paint-core --lib liquify --locked`：4项通过。两个采样/256瓦片固定上限用例覆盖负坐标、瓦片边缘、缺失瓦片、隐式白底边界，与无缓存四读取参考逐位比较；两个历史落盘失败/重试/撤销用例通过。
- `cargo test -p paint-core --test liquify --locked`：11项通过，1项手工性能基准按原标记忽略。覆盖四工具、压力/密度/速度/时间、选区/偏移/蒙版/锁、预算失败原子性、局部/总历史和既有像素摘要。
- `cargo clippy -p paint-core --lib --locked -- -D warnings`与`cargo fmt --all -- --check`通过。
- Windows应用/Qt/许可Release及生成头检查通过。Qt新增2000px/512×512实际推移、局部撤销重做、取消、提交与总历史像素恢复通过；GUI提交0ms，等待帧完成51ms，仅为该小文档入口检查。旧真实Main液化选项/跨窗口/文档锁/历史用例也通过，记录`artifacts/feedback-liquify-qt.txt`。

无ABI、持久化或交换格式变化。未跑全量、实体设备、跨平台或GPU；大文档持续输入和整体手感仍需用户复核。
