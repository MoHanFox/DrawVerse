# 第二笔尖尺寸保存与交换验收

2026-10-14，state15/VBR1.13/behavior12；ABI保持1.46。sizing严格对象位于dualBrush，默认省略，关闭或独立模式保留完整非默认值。能力dual-relative-size与最高版本检查生效；内核算法未改。

- brush_dual_size_exchange 3项通过：关闭/开启×独立/相对×跟随/固定规则八组，合并真实time-grid平滑，state与VBR恢复后两个带时间/压力的笔触逐像素一致、两条历史及撤销重做保持；旧state14及manifest1.12拒绝；缺失能力或错误behavior拒绝。默认状态编码/恢复字节相同、旧版本不升级。
- 严格DTO验证未知model/mode/rule/字段、缺失字段、null/类型错误与f64舍入前越界。Schema模块及manifest版本断言通过；未运行完整JSON Schema解释器验证。
- brush_dual_size_boundary 1项通过：关闭/开启及独立/相对非默认选项完整回核心；默认DTO恢复保持默认。替代上阶段旧格式临时拒绝测试。
- 原生native_relative_sizing_gate_preserves_current_catalog_and_file通过：state准备及资源包接入均Unsupported，当前预设未发布；公开库包写入返回Unsupported且已有文件字节保持。Qt还未接新尺寸，所以这些是临时入口保护，并非功能完成。
- 相关brush_dual_exchange 3项、brush_smoothing_exchange 3项通过；paint-io/paint-ffi all-targets Clippy -D warnings及workspace all-targets cargo check通过。日志brush-dual-size-io-tests/new/native/clippy/check.log。
- Release Qt应用及Rust FFI构建通过（brush-dual-size-io-build.log）；应用未运行时更新开发目录exe/DLL，DLL与验证构建SHA256一致。

未运行全仓tools/check.py或Qt交互测试。下一阶段接通原生准备与Qt分类控件/重置/有效尺寸显示，并解除临时入口拒绝；双色合成、溢出与自动描边及复合光标仍待实施。
