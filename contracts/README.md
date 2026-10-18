# 当前契约入口

当前笔刷文件格式统一为 **VBR 1.0**。写出固定 `major: 1, minor: 0`，读取接受所有 1.x 并按实际字段补全、校验和执行。功能不依赖 minor 门槛。

| 职责 | 当前契约 |
| --- | --- |
| FFI 的指针、所有权、线程和缓冲区 | [C ABI](ffi/abi.md)，声明由 [Rust ABI](../core/crates/paint-ffi/src/abi/api.rs) 生成到 [C 头](../ui/include/paint_api.h) |
| VBR 1.0 封装与读写兼容 | [格式契约](brush/vbr/format.md) |
| VBR 1.0 规范写出结构 | [manifest 外壳](brush/vbr/manifest.schema.json)、[preset 外壳](brush/vbr/preset.schema.json) |
| VBR 1.0 有序组树与原生包交接 | [organization](brush/library/brush-organization.schema.json)、[package handoff](brush/library/brush-package-handoff.schema.json)、[示例](brush/library/examples) |
| VBR 1.0 湿混和起收笔参数示例 | [wet fixtures](brush/wet/fixtures) |
| VBR 1.0 引用的 ABR 来源记录示例 | [ABR fixtures](brush/abr/fixtures) |

当前 schema 使用稳定的无版本后缀文件名。模型标识（如 `ordered-tree-v1`）、JSON Schema 标准版本、`$id` / `$ref` 引用身份、ABI、state、behavior 和 ABR 来源/转换器版本均有独立含义，不等于 VBR 格式版本。ABR 示例文件名中的 `converter-N` 明确指转换器版本。

两个 VBR schema 描述规范写出的外壳，不是完整读取验证器；嵌套参数范围、功能组合、缺项补全、资源身份、树投影和容量限制由 `paint-io` 执行。版本常量统一位于 [vbr.rs](../core/crates/paint-io/src/vbr_format/vbr.rs)。

开发期间的 43 个迭代 schema 已移至 [paint-io 测试快照](../core/crates/paint-io/tests/fixtures/schema-history/README.md)，只供历史兼容回归，不再作为当前 contracts 展示。历史设计说明保留在 docs；早期 ABI 头草案位于 `ffi/history/`，不代表当前接口。
