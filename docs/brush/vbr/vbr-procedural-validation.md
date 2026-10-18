# VBR 程序笔刷包验收

2026-10-13：首个可执行程序子集已接 Rust IO → ABI 1.26 → Qt 后台 worker → 笔刷库。画笔面板提供导入包/导出当前预设，默认导出扩展名 vbr。参数、UUID、必需能力与行为版本往返；同 ID 导入生成副本，容量不足或包错误整体拒绝；前景色、临时工具尺寸、文档历史与光标状态不进入包。

已验证：

- `cargo test -p paint-io -p paint-ffi --locked` 全部通过，包括 VBR 四项（往返可执行像素与单位、坏定义/取消保存保留旧文件、路径/别名/超限/版本/资源拒绝、Schema 数值边界与内核匹配），FFI 输出大小/非法长度/文件不变与旧异步接口。
- `cargo clippy -p paint-io -p paint-ffi --all-targets --locked -- -D warnings` 通过。
- Release Qt 构建/生成头检查通过；五项相关原生 Qt 测试通过，其中新增真实导出/导入、重复导入副本、参数一致、坏包不改库及文档。无 QML 警告。
- C/C++ 静态/动态四项 ABI smoke 通过。日志 `artifacts/vbr-*`。

范围限制：仅程序笔刷参数，尚未实现资源哈希/PNG 笔尖/纹理、分组、来源扩展和 ABR；包含这些内容的包明确拒绝。UI 沿用64预设上限。Schema 只描述本子集，Rust 另行验证 ZIP 路径与身份关系；未运行第三方全量 JSON Schema 验证器。完整 VBR 草案尚未冻结。本轮是相关模块验证，没有宣称运行完整仓库检查。
