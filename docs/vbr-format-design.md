# VBR 原生笔刷格式草案

状态：2026-10-14，完整格式尚未冻结。已有1.0程序笔刷与1.1图片笔尖可执行子集，见 [程序阶段设计](vbr-procedural-design.md)、[图片阶段设计](vbr-image-design.md)、[程序Schema](../contracts/vbr-procedural-v1.schema.json) 与 [图片Schema](../contracts/vbr-image-v1.1.schema.json)。1.2来源IO及 [来源Schema](../contracts/vbr-source-v1.2.schema.json) 已有阶段实现，原生Qt交换及显式原ABR范围选项已接入，见 [来源IO验收](vbr-source-validation.md) / [来源入口验收](vbr-source-entry-validation.md)。压力曲线1.3与 [压力Schema](../contracts/vbr-pressure-v1.3.schema.json) 已有IO阶段实现，见 [压力IO验收](brush-pressure-io-validation.md)；Qt入口已接入。独立不透明度压感1.4与 [Schema](../contracts/vbr-opacity-v1.4.schema.json) 已接原生Qt，见 [设计](brush-opacity-pressure-design.md) / [验收](brush-opacity-pressure-validation.md)。设备几何1.5与 [Schema](../contracts/vbr-tip-dynamics-v1.5.schema.json) 已有IO阶段实现，见 [验收](brush-device-io-validation.md)；原生Qt入口已接入，见 [验收](brush-device-ui-validation.md)。散布1.6与 [Schema](../contracts/vbr-scatter-v1.6.schema.json) 已接IO，见 [验收](brush-scatter-io-validation.md)；原生Qt入口已接并解除临时拒绝，见 [验收](brush-scatter-ui-validation.md)。形状抖动1.7与 [Schema](../contracts/vbr-shape-jitter-v1.7.schema.json) 已有IO阶段实现，见 [验收](brush-shape-jitter-io-validation.md)；原生Qt已接并解除临时拒绝，见 [验收](brush-shape-jitter-ui-validation.md)。颜色动态1.8与 [Schema](../contracts/vbr-color-dynamics-v1.8.schema.json) 已接IO，显式linear-hsv-v1与工具颜色分离，见 [验收](brush-color-io-validation.md)；原生背景与Qt已接并解除临时拒绝，见 [验收](brush-color-ui-validation.md)。纹理1.9与 [Schema](../contracts/vbr-texture-v1.9.schema.json) 已接IO，预设级texture绑定覆盖PNG并共享周期mip，清单5120/ZIP6145边界，见 [验收](brush-texture-io-validation.md)；原生Qt设置和资源交换已接并解除库包临时拒绝，见 [验收](brush-texture-ui-validation.md)。双笔刷1.10与 [Schema](../contracts/vbr-dual-v1.10.schema.json) 已接IO，两模型/完整第二动态与四角色PNG并集、6144资源/7169ZIP边界，见 [验收](brush-dual-io-validation.md)；原生资源生命周期、Qt四子页和第二图片导入/资源交换已接，见 [验收](brush-dual-ui-validation.md)；库包临时拒绝解除，组合轮廓与PS核验继续后续。分组/其他高级动态和扩展仍是拟议范围；读写器拒绝未实施模块。完整格式发布前按 [笔刷验收计划](brush-system-validation.md) 冻结版本。

VBR 作为 DrawVerse 的公开、跨平台笔刷交换格式，与工程文件分离。一个文件可以包含一个笔刷或一个带分组的笔刷包，均使用 `.vbr` 扩展名。不依赖源 ABR 路径、系统字体、Qt 对象或开发机目录。

## 1. 容器与目录

ZIP 容器，允许 Store/Deflate，JSON 为 UTF-8。逻辑路径使用 `/`，大小写敏感；不需要把条目解压到磁盘。

```text
manifest.json
brushes/<brush-id>.json
assets/<sha256>.png
previews/<brush-id>.png
```

manifest 必须包含格式标识、major/minor、包 ID、预设清单、资源清单。分组顺序显式保存，不依赖 ZIP 条目顺序。预设和包 ID 使用稳定 UUID；资源以完整内容 SHA-256 命名及校验，可在包内共享。

每个预设记录自己的 `engineProfile` 和 `behaviorVersion`，并列出必需能力。格式版本说明文件结构，行为版本说明参数如何计算；两者不得混用。ZIP 时间戳和条目顺序不参与预设身份。

示意结构（字段尚未冻结，UUID/哈希占位符不得作为正式合法样本）：

```json
{
  "format": "DrawVerseBrush",
  "version": { "major": 1, "minor": 0 },
  "packageId": "<uuid>",
  "brushes": [
    { "id": "<uuid>", "path": "brushes/<uuid>.json" }
  ],
  "groups": [],
  "assets": [
    {
      "id": "<sha256>",
      "path": "assets/<sha256>.png",
      "sha256": "<sha256>",
      "kind": "tip-mask"
    }
  ]
}
```

## 2. 预设内容与参数语义

| 部分 | 内容 |
|---|---|
| 身份 | ID、名称、作者、说明、可选授权信息和标签 |
| 行为 | 引擎 profile、行为版本、必需能力、默认工具提示（如有） |
| 笔尖 | 程序形状或资源 ID、基准尺寸、圆度、角度、硬度、间距 |
| 动态 | 模块启用状态、目标通道、控制来源、曲线、最低值和随机变化 |
| 渲染 | 流量、整笔不透明度、积累规则、颜色计算规则 |
| 材料 | 纹理、双笔尖、载色/湿混及相关参数 |
| 来源 | 导入格式、原始名称、转换器版本、近似/不支持项说明 |
| 扩展 | 有命名空间和大小上限的非执行附加数据 |

单位统一：长度为文档像素；时间为秒；角度为度；比例为 0–1；spacing 为基准直径比例，0.01 表示界面 1%。超出普通比例的量必须另行声明范围，不以任意 0–100 数字隐式代表百分比。

动态曲线为按输入升序排列的控制点，输入归一化为 0–1，插值方式显式指定。初版支持线性插值；高级插值仅在真实实现后成为正式能力。曲线端点、重复输入、输出范围和组合顺序须由 Schema 与 Rust 语义验证共同约束。JSON 禁止非有限数，不对非法值静默裁剪。

拟议 profile：迁移旧预设的 `legacy.round` 与统一引擎的 `drawverse.brush`；仅实施后才声明对应版本可执行。参数和来源信息可共存，但不能执行来源数据中的未知字段。

## 3. 图片资源

笔尖 `tip-mask` 是覆盖数据：灰度白色表示完全覆盖，黑色表示无覆盖；PNG 若有 alpha，最终覆盖为灰度值乘 alpha。数据不应用显示颜色空间转换。ABR 笔尖的反向灰度等差异由导入器转换，不把 ABR 特有解释传播到 VBR。

纹理 `grain` 使用明确的灰度数据语义，记录平铺规则和采样方式。首版不接受任意颜色图片冒充灰度纹理；彩色印章作为未来独立能力定义。像素原点为左上，X 向右、Y 向下；旋转正方向在统一图像坐标中明确，CPU/GPU 必须一致。

预览 PNG 为 sRGB 显示图片，可缺省、可重新生成，不参与笔刷执行。预览轨迹、模拟压力、底样、行为版本和预览器版本可作为缓存签名。不能以包内预览图片证明该笔刷已正确导入。

## 4. 明确不保存的状态

- 当前鼠标位置、压力、上一印章、随机发生器运行状态和载色缓冲。
- 当前文档、图层、选区、画布缩放、历史与暂存盘位置。
- 当前前景/背景色和工具临时大小覆盖；笔刷颜色动态规则本身可以保存。
- 涂抹等工具的当前强度、所有图层取样和手指绘画开关；这些由工具状态保存。
- 全局笔刷光标描边、颜色/线宽、中心十字、图片透明度和显示时机。
- 设置面板布局、窗口位置、当前分类与编辑焦点。

包与预设的作者信息不构成自动授予资源使用权；导入/导出保留用户提供的授权说明。

## 5. 版本、兼容与转换

- 未知 major：拒绝执行并给出版本原因，不以默认圆笔替代。
- 新 minor：只在必需能力全部支持时执行；不认识的可选附加信息可以保留。
- 未知行为 profile/版本或启用的必需模块：预设不可执行，报告具体原因。
- 未启用且不影响当前行为的未来模块：允许按有界来源数据保留，保存时不得丢失而无提示。
- 字段存在不代表功能已经实现；支持清单来自真实引擎能力，而非 UI 分类名。

首次保存形成规范化 JSON 和稳定的资源引用；重命名不改变笔刷 ID，保存副本分配新 ID。编辑引擎参数生成新定义修订，文件行为版本不因普通参数编辑变化。升级旧行为版本必须显式转换并可对比预览，保留旧副本。

未知扩展以命名空间对象保存，仅用于回写和显示，不允许脚本、shader、动态库、外部 URL 加载或任意资源访问。首版扩展不能作为可执行插件入口。

ABR 导入报告逐项区分准确转换、近似转换、暂不支持；如保留未知来源块，使用有界数据字段并记录解析器版本。VBR 导出后不要求再次访问原 ABR。VBR→ABR 导出不属于首版目标。

## 6. 资源上限与原子 IO

首版拟议默认上限：单包压缩文件 128MiB、解压条目总计 256MiB、最多 4096 个条目/1024 个预设、单 JSON 2MiB、单图片最长边 4096。实际解码驻留另设预算，流式验证和资源去重，不能把全部图片一次常驻。实施时可根据真实样本调整，调整需修改契约与边界测试，不允许无上限。

在分配前检查长度、乘法溢出、图片尺寸、解压累计大小、条目数量和 JSON 深度。拒绝重复规范化路径、大小写冲突、绝对路径、盘符、`..`、反斜杠别名、符号链接、加密 ZIP、缺失资源与错误哈希。即使不落盘也执行路径验证。不能仅靠压缩比识别资源超限，合法纯色图片也可能具有很高压缩比。

读取先验证整个包并建立临时定义/资源集合，再原子发布到笔刷库。取消、错误或存储失败不得留下部分预设。可报告哪些条目失败，但继续导入合法条目必须是显式的另一次选择，不能伪装整包成功。

保存写同目录临时文件，完成校验后原子替换；失败保留原文件。包内只使用资源 ID/逻辑路径，系统暂存路径不进入文件。资源哈希、规范化后的定义和行为版本共同构成执行缓存身份。

## 7. 开放发布要求

首版随读写实现发布：格式说明、JSON Schema、合法/非法示例包、迁移示例、能力兼容矩阵及资源单位说明。Schema、解析器和示例由仓库验证检查一致性，不能先宣称已开放完整规范而只提供未经验证的草案。规范授权在正式发布前明确写入文档。

## 已确认的双笔刷扩展

[双色复合与自动描边设计](brush-dual-color-extension-design.md)纳入后续格式范围：绘制方式、第二颜色来源开关、独立/相对尺寸及保留值、溢出、正常双色合成/叠放顺序和尺寸压力规则。实际前景/背景颜色由工具在起笔时捕获，不能把全局当前颜色写入预设。新执行模型需要新行为/能力及完整Schema和关闭值往返；当前VBR1.10不能表达此扩展，不预先分配版本或宣称已支持。等待用户再次发令后实施。

## 第二笔尖相对尺寸子集 1.13

2026-10-15：dualBrush.sizing采用relative-size-v1、independent/relative、diameterOffset直径像素偏移[-511,511]与follow-pressure/fixed-width。关闭或独立模式非默认值仍完整保存，默认省略保持旧版本；对象存在要求drawverse.brush/behavior12、dual-relative-size能力及minor13。严格完整字段/舍入前范围/null拒绝，和time-grid平滑共存选择最高版本。ABI1.47/Qt已接通，见brush-dual-size-io-validation.md、brush-dual-size-ui-validation.md及contracts/vbr-dual-sizing-v1.13.schema.json。这里只表达第二灰度覆盖尺寸，不宣称支持双色叠放或溢出绘制。

## 双色双笔刷子集 1.14

2026-10-15：dualBrush.color采用separated-stroke-v1、enabled/useBackground/overflow布尔、second-behind/second-in-front顺序、primaryBlend/secondaryBlend稳定27英文名称。实际前景/背景RGBA不存入预设。非默认关闭值保留，默认省略保持旧字节与版本；存在对象要求drawverse.brush/behavior13、dual-color-stroke能力与minor14，state16。Schema见contracts/vbr-dual-color-v1.14.schema.json，结果见brush-dual-color-io-validation.md。原ABR来源保持。ABI1.48/Qt已接完整色彩配置、库包入口和实际两色预览，见brush-dual-color-ui-validation.md；自动描边继续后续；非分隔逐印章模型未实现，不得冒用separated模型。

湿混进度：linear-premul-grid-v1目前只存在于Rust二维载色数值组件，尚未添加到BrushSettings或JSON；本阶段不预占state/VBR版本或能力名，不宣称VBR已支持湿混。后续接入仍需完整严格字段、关闭值、基准与资源/真实像素往返。
