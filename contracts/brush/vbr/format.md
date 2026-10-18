# VBR 1.0 当前契约

VBR 是 ZIP 包，当前写出版本固定为 `1.0`。manifest 的 `format` 为 `DrawVerseBrush`，`version` 为 `{"major":1,"minor":0}`。文件名后缀 `.vbr` 不决定能力；预设实际字段决定功能。

## 读写兼容

读取接受任意 `major: 1` 的 minor，不以历史功能首次出现时的 minor 拒绝文件。缺失的 `version.minor`、`groups`、`assets` 分别补为 `0`、`[]`、`[]`。不支持的 major、未知结构字段、非法数值或损坏资源仍会失败。

预设读取递归补全已存在对象中的缺项；已有值保留，可选对象整体缺失或为 `null` 仍表示关闭。随后按实际字段归一化 `engineProfile`、`behaviorVersion` 和 `requiredCapabilities`，已正确的能力顺序保持。写出和手工构造对象仍严格校验。ABI、state、behavior、source/converter 版本不是 manifest 版本，不重置为 1.0。

## 包内职责

| 路径 | 内容与约束 |
| --- | --- |
| `manifest.json` | 包 UUID、预设引用、资源引用、可选有序组织树 |
| `brushes/{uuid}.json` | 对应 UUID 的预设，支持程序或资源笔尖及实际功能字段 |
| `assets/{sha256}.png` | `tip-mask` 资源；主笔尖、第二笔尖和纹理按内容身份复用 |
| `sources/{sha256}.json` | `source-record` 来源记录 |
| `originals/{sha256}.abr` | `abr-original` 原始来源包 |

manifest 的 `groups` 保持空数组；实际分组在可选 `organization` 的 `ordered-tree-v1` 中表达。树的 DFS 预设投影必须与 `brushes` 顺序完全一致。原生交接使用独立的 `ordered-package-v1` 包装，见 [交接 schema](../library/brush-package-handoff.schema.json)。

包与预设 UUID 非零、小写且不重复；预设数为 1–1024，资源数最多 6144。路径必须与身份及类型一致，资源 hash 必须匹配内容；不允许额外未引用条目。JSON 单项上限 2 MiB、文件上限 128 MiB、展开总量上限 256 MiB，来源记录总量另有 32 MiB 上限。不同公开入口可以只支持功能子集，文件 minor 不替代入口能力和资源检查。

## 规范与实现

[manifest schema](manifest.schema.json) 与 [preset schema](preset.schema.json) 描述规范写出的外壳，明确要求 writer 的 `1.0`；它们不直接用于判断容错读取是否成功，也没有穷举嵌套参数模型。名称、动态数值、资源、编译预算和语义检查以 [VBR 预设实现](../../../core/crates/paint-io/src/vbr_format/vbr.rs) 与 [完整资源包实现](../../../core/crates/paint-io/src/vbr_format/vbr_image.rs) 为准。

兼容决策与验证见 [1.0 设计](../../../docs/brush/vbr/vbr-format-1.0-compatibility-design.md) / [验收](../../../docs/validations/brush/vbr/vbr-format-1.0-compatibility-validation.md)。开发迭代 schema 仅作为 [测试快照](../../../core/crates/paint-io/tests/fixtures/schema-history/README.md) 保留；当前契约中不再列出测试迭代版本。

## 双笔刷比例尺寸

`dualBrush.sizing.model=relative-ratio-v1`采用`diameterRatio`（有限0.0005–2000倍），与旧`relative-size-v1/diameterOffset`互斥；保留mode和widthRule。比例对象即使关闭也完整保留。执行为主半径乘比例后钳制到0.5–1000；旧像素偏移不迁移或重解释。当前写出state38、behavior30及`dual-size-ratio`能力，VBR包版本继续1.0。

## 起收笔相对长度

`strokeTaper.start/end`可带成对的`lengthMode`（independent/relative）和`lengthPercent`（有限0–10000），并保留`lengthPx`。缺少新增字段时沿用独立像素语义；独立模式也保存关闭的百分比。相对执行长度为起笔实际主直径×百分比/100，最多4096px，不扩大输入和尾部预算。该对象写state39、behavior31和`taper-relative-length`；VBR仍为1.0。不透明度minimum=1表示该端不额外减淡，尺寸曲线独立。
