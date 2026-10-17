# VBR 原生笔刷格式草案

2026-10-18逐印章双色：VBR1.28新增`interleaved-stamp-v1`双色渲染模型，behavior17及`dual-color-stamp`能力；它只改变主/第二贡献的执行顺序，不改变颜色对象结构。VBR1.24分组/state20、1.25-1.27纹理格式及旧state21字节保持；VBR1.27读取到逐印章模型必须拒绝。见[设计](brush-dual-stamp-design.md)、[验收](brush-dual-stamp-validation.md)及颜色对象[Schema](../contracts/dual-color-stamp-v1.28.schema.json)。

2026-10-18时间喷枪：VBR1.29新增parameters.airbrush=`time-interval-v1`（enabled、intervalMs 1–1000），behavior18及`airbrush`能力；旧VBR1.28不携带该对象。见[设计](brush-airbrush-design.md)、[验收](brush-airbrush-validation.md)及[Schema](../contracts/vbr-airbrush-v1.29.schema.json)。

2026-10-18速度动态：VBR1.30新增parameters.speedDynamics=`speed-v1`，完整保存启用、最大速度和尺寸/流量/不透明度三条曲线，behavior19及`speed-dynamics`能力；旧VBR1.29不携带该对象。见[设计](brush-speed-design.md)、[验收](brush-speed-validation.md)及[Schema](../contracts/vbr-speed-v1.30.schema.json)。

2026-10-18切向压力动态：VBR1.31新增parameters.tangentDynamics=`tangent-v1`，完整保存启用和尺寸/流量/不透明度三条曲线，behavior20及`tangent-dynamics`能力；旧VBR1.30不携带该对象。见[设计](brush-tangent-design.md)、[验收](brush-tangent-validation.md)及[Schema](../contracts/vbr-tangent-v1.31.schema.json)。

2026-10-18笔触距离动态：VBR1.32新增parameters.distanceDynamics=`distance-v1`，完整保存启用、最大距离和尺寸/流量/不透明度三条曲线，behavior21及`distance-dynamics`能力；旧VBR1.31不携带该对象。见[设计](brush-distance-design.md)、[验收](brush-distance-validation.md)及[Schema](../contracts/vbr-distance-v1.32.schema.json)。

2026-10-18倾斜幅度动态：VBR1.33新增parameters.tiltDynamics=`tilt-v1`，完整保存启用和尺寸/流量/不透明度三条曲线，behavior22及`tilt-dynamics`能力；旧VBR1.32不携带该对象。见[设计](brush-tilt-design.md)、[验收](brush-tilt-validation.md)及[Schema](../contracts/vbr-tilt-v1.33.schema.json)。

2026-10-18笔旋转动态：VBR1.34新增parameters.rotationDynamics=`rotation-v1`，完整保存启用和尺寸/流量/不透明度三条曲线，behavior23及`rotation-dynamics`能力；旧VBR1.33不携带该对象。见[设计](brush-rotation-design.md)、[验收](brush-rotation-validation.md)及[Schema](../contracts/vbr-rotation-v1.34.schema.json)。

2026-10-18笔触方向动态：VBR1.35新增parameters.directionDynamics=`direction-v1`，完整保存启用和尺寸/流量/不透明度三条曲线，behavior24及`direction-dynamics`能力；旧VBR1.34不携带该对象。见[设计](brush-direction-design.md)、[验收](brush-direction-validation.md)及[Schema](../contracts/vbr-direction-v1.35.schema.json)。

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

湿混Document已接Rust独立起笔/结束并返回载色，但仍未添加BrushSettings/JSON字段；当前VBR版本及能力保持不变，湿混交换待完整IO阶段。

湿混设置后续更新：WetSettings现已进入Rust BrushSettings/Definition/Program，含启用、取样、自动清洗/装载和WetMix；尚无JSON/VBR字段，旧state及来源转换基线拒绝非默认湿混以免丢数据。文件格式版本与能力仍不变，完整交换待下一阶段，见brush-wet-settings-validation.md。

## 湿混子集1.15

2026-10-15：预设顶层wetMix使用linear-premul-grid-v1。严格完整字段model、enabled、allLayers、autoClean、autoLoad及wetness/pickup/mix/load/deposit/consumption（0..1，舍入前验证）。默认省略，关闭非默认保留；对象存在要求drawverse.brush/behavior14、wet-mix能力和manifest minor15，状态state17。实际工具两色/二维载体不入预设；有效湿混+有效双色拒绝，关闭配置可交换。Schema与合法/非法样例见contracts/vbr-wet-v1.15.schema.json及contracts/fixtures。实际两笔载色/像素、图片/纹理/灰度双路径/PCHIP/平滑/散布与原ABR来源已往返，见brush-wet-io-validation.md。ABI1.49/旧Qt准备和库包入口暂拒湿混，完整用户入口、基准及工具寿命待后续接入。

湿混原生后续：ABI1.50新增显式prepare_state_wet、library_package_read_wet/write_wet及共享工具接口，完整保留state17/VBR1.15，旧入口继续拒绝以保护旧Qt。Qt镜像/基准/界面及真实有色预览仍待接入，见brush-wet-abi-validation.md。

湿混Qt后续：state17镜像、完整关闭值和resetState基准已进入Qt，文件读写使用ABI1.50显式wet库包入口，保存重开及新目录VBR恢复通过，见brush-wet-qt-validation.md。载体仍仅运行时持有，不进入VBR；旧原生接口的保护性拒绝继续保留。

## VBR1.16：ABR整笔不透明度来源

包含drawverse.abr.basic converterVersion2来源记录时最低minor16；该版本映射非100% toolOptions.Opct到已有整笔opacity。来源记录自身仍version1，v1/v2原记录及原ABR逐字节保持；SourceBatch最高转换器版本只用于选择包版本。旧来源不重新转换，未用新行为不升级来源，去除来源的导出仍按实际参数/资源选择版本。解码拒绝新来源降级夹带，新增完整schema和合法/非法来源示例见 [设计](abr-opacity-design.md) / [验收](abr-opacity-validation.md)。不新增笔刷行为版本、必需能力或C ABI/state字段；其他动态、湿混组合和VBR分组等范围继续后续。

## VBR1.17：ABR笔压来源

converterVersion3来源基准保存已有pressureDynamics（三通道线性曲线/最低比例与opacity开关），仍为来源record1；旧v1/v2基准、报告及原记录字节不重算。含v3来源要求manifest minor17，导出按全部来源最高版本需求选择，旧minor夹带新来源拒绝。执行参数继续沿用pressure-curves/opacity-pressure和behavior2/3，state5/6已能表示，无新执行能力。只有不包含来源的新笔压预设导出仍按已有VBR1.3/1.4能力需求选版本。公开Schema contracts/vbr-abr-pressure-v1.17.schema.json，见abr-pressure-design.md；完整ABR/16位等继续后续。

## VBR1.18：ABR散布来源

来源converterVersion4保存既有scatterDynamics基准，完整关闭配置可保留，pressureDynamics仍为独立可选对象；原记录结构version1。包含v4来源最低minor18，按全部来源最高需求导出，17及以下夹带拒绝；旧三代来源字节/基准不重算。执行继续使用既有behavior5/scatter-stamps与state8，无来源时仍按已有VBR散布能力选版本。Schema contracts/vbr-abr-scatter-v1.18.schema.json及合法/非法来源样例已发布。PS随机分布明确近似，设备/渐隐散布控制及16位等仍待后续，见abr-scatter-conversion-validation.md。

## VBR1.19：ABR颜色动态来源

来源converterVersion5必须保存既有colorDynamics基准，可独立组合pressureDynamics/scatterDynamics；颜色字段不会混入压力DTO，纯颜色来源不强加散布对象。原记录仍为version1。包含v5来源最低minor19，按所有来源最高需求导出，18以下夹带拒绝；旧四代来源、报告和原记录字节不重算。执行沿用behavior7/color-dynamics与state10；没有新来源时原生颜色配置仍按原有VBR版本需求导出。Schema为contracts/vbr-abr-color-v1.19.schema.json。PS随机/颜色空间明确近似，纯度与设备颜色调制尚未执行，见abr-color-conversion-design.md。

## VBR1.20：ABR形状抖动来源

来源converterVersion6必须保存shapeJitter基准，可独立组合既有pressureDynamics/scatterDynamics/colorDynamics，其他动态仍明确拒绝。原记录仍为version1。包含v6来源最低minor20，按全部来源最高需求导出，19以下夹带拒绝；旧五代转换入口/记录字节/报告不重算。执行沿用behavior6/shape-jitter与state9；普通原生形状配置不因本模块提高版本需求。Schema为contracts/vbr-abr-shape-v1.20.schema.json。随机分布、压力及随机下限组合、圆度下限换算均明确近似；设备形状控制等仍未执行，见abr-shape-conversion-design.md。

## VBR1.21：ABR设备形状来源

来源converterVersion7必须保存tipDynamics基准，pressureDynamics/scatterDynamics/shapeJitter/colorDynamics独立可选。来源record仍version1，旧六代记录字节/报告不重算；有v7来源时最低minor21，20以下夹带拒绝。执行沿用behavior4/tip-dynamics及state7，无新C ABI或执行行为。原生设备配置无新来源时仍使用原来的VBR版本需求。Schema为contracts/vbr-abr-device-v1.21.schema.json。方向、笔旋转、倾斜角度/圆度及原生能力回退标近似；关闭设备保留在来源而不执行，其他设备与渐隐继续未执行，见abr-device-conversion-design.md。

## VBR1.22：起收笔

Preset顶层strokeTaper采用distance-envelope-v1：model/enabled/start/end完整必填，各端lengthPx（0..4096文档像素）和size/opacity曲线（curve/minimum/interpolation，linear或pchip）。每条2..17节点，X严格递增、XY在0..1且端点为[0,0]/[1,1]；先f64验证再转换核心f32，重合拒绝。默认省略，关闭非默认完整保留；存在对象要求drawverse.brush、behavior15、stroke-taper能力及minor22，state18。混包取全部执行/来源最高需求，旧包不升级，降级拒绝。旧ABR基线不包含起收笔，当前用户修改可与不变的原来源独立交换。Schema contracts/vbr-taper-v1.22.schema.json，实际两笔和图片/双色/平滑/湿混/来源验证见brush-taper-io-validation.md。ABI1.50旧准备和包入口暂拒新配置，完整原生及Qt入口待接。

原生后续：ABI1.51新增显式prepare_state_taper、library_package_read_taper/write_taper，完整保留state18/VBR1.22与先前湿混数据；原入口和wet入口继续拒绝新起收笔以保护旧客户端。准确票据/资源重建及真实异步绘画验收见brush-taper-abi-validation.md，Qt镜像/重置基准/界面仍待接。

Qt后续已完成：完整state18镜像/关闭配置/复制和resetState、ABI1.51显式准备及包入口、真实白色预览和距离/独立尺寸opacity曲线设置；实际在途捕获、像素历史、新资源目录VBR交换与偏好重开见brush-taper-ui-validation.md。没有新增格式版本。

## VBR1.23：独立湿混双色载色策略

wetMix可选dualColor字符串，唯一当前值independent-carriers-v1，声明主/第二笔刷各自持有二维载色。缺省保持旧湿混；显式null、未知模型和非字符串拒绝。存在标记就要求drawverse.brush、behavior16、wet-dual-color及wet-mix能力、minor23和state19，即使湿混或第二色关闭也完整保留。实际两角色必须在湿混、第二笔刷和双色都启用时执行；没有声明策略的旧湿混双色仍拒绝，不自动改变旧语义。载色像素属于瞬时工具状态，不写入笔刷。

旧状态不写标记、不升级版本或改字节，混包取全部执行/来源需求最高版本，低minor夹带拒绝。原ABR来源及转换基线保持旧版本/原字节；新策略不能写成旧基线，当前用户修改可与不变来源和图片独立交换。完整Schema为contracts/vbr-wet-dual-v1.23.schema.json，实例及真实两笔/双载色/资源/来源验收见brush-wet-dual-io-validation.md。当前ABI1.51的Legacy/Wet/Taper准备和包入口均暂拒新策略防止旧Qt丢字段，新原生/Qt组合入口继续下一阶段。

2026-10-16湿混双色Qt阶段：完整state19镜像/关闭配置/复制和resetState、新ABI1.52准备及VBR1.23包、两色手动装载和隔离白色预览已接通，真实绘画/历史/图片三资源用途与新目录导入重开像素验证见[验收](brush-wet-dual-ui-validation.md)。Windows新3旧6Qt、环境倍率2/Release/ABI通过；旧入口拒绝保护保留。大跨度密集图片输入仍会触发预算回滚，分块调度未实现；原参数输入偶发问题未定位。完整剩余范围见[功能清单](brush-system-remaining.md)，用户要求本阶段完成后等待指示。

2026-10-17输入分页Actor阶段：实际Move/End接通Document有界页，完成水位/笔尾发布/单历史与双载色采用保持，邮箱及暂存共同限额、页间取消/重建/stop和屏障顺序已接。原64px单输入经公开API和Qt新目录交换重开对照通过；任务8项、新Actor6项及相关原生/公开回归、Windows Release/Clippy/4 ABI/Qt通过，见[设计](brush-input-paging-actor-design.md)及[验收](brush-input-paging-actor-validation.md)。无ABI/格式变化；大笔刷性能并非全部完成，单印章及总输入预算保持，整体设备手感由用户验收。

2026-10-17笔刷分组Rust目录阶段：不可变稳定ID组织/有序根及子项/DFS投影、原子建组改名移动/解组及受保护级联删除、同组复制导入与捕获保持已接；新6集成+1修订边界/旧5目录、IO5/原生2及Windows Release/Clippy/4 ABI/旧Qt2通过，见[设计](brush-groups-design.md)及[验收](brush-groups-kernel-validation.md)。旧状态编码有组时明确拒绝，未接分组保存交换/原生及Qt，不宣称用户入口完成；无ABI/state/VBR版本变化。

2026-10-17分组状态阶段：有界ordered-tree-v1 DTO/state20已实现，空组/显式顺序/关闭参数及真实三图片用途/ABR来源跨新目录两笔像素载色历史往返；最终IO20项、新旧原生四入口保护1项及Schema/Windows Release/Clippy/4 ABI/旧Qt2通过，见[设计](brush-groups-state-design.md)及[验收](brush-groups-state-validation.md)。平目录旧字节保持；旧原生有组明确拒绝。无ABI或VBR版本变化，VBR分组清单/整树映射/原生和Qt仍待接，不宣称用户分组完成。

2026-10-17分组VBR阶段：VBR1.24组织清单/空组与顺序、原子整树副本ID映射已接，三图片用途/ABR来源及真实两笔像素载色历史保持；IO33项、新四类旧读写保护1项、Schema/Windows Release/Clippy/4 ABI/旧Qt2通过，见[设计](brush-groups-vbr-design.md)及[验收](brush-groups-vbr-validation.md)。平包旧字节保持，无ABI/state变化；目标库原子合并/完整原生交接/Qt仍待接，不宣称分组用户入口完成。

2026-10-17分组目标目录合并阶段：Core支持完整有序子树在根/指定组位置原子合并、跨角色身份/合并深度容量与资源预算预检、单修订及失败保持；VBR真实资源和来源内存桥已接，两笔像素/双载色/历史保持。新核心4/旧11/修订1、IO21及Windows Release/Clippy/4 ABI/旧Qt2通过，见[设计](brush-groups-merge-design.md)及[验收](brush-groups-merge-validation.md)。无ABI/state/VBR变化；资源文件耐久事务、完整原生交接/Qt仍待接，不宣称分组用户入口完成。

2026-10-17分组资源发布阶段：候选目录先完整合并预检、包内PNG与来源先落盘后采用；故障保留原目录/共享文件，真实新目录state/组织与两笔像素载色历史恢复通过。IO40、Clippy/Windows Release/4 ABI/旧Qt2通过，见[设计](brush-groups-persist-design.md)及[验收](brush-groups-persist-validation.md)。无ABI/state/VBR变化；失败可能保留合法未引用公共缓存，不声称多文件整体回滚/GC。完整原生交接/Qt及重置基准全链仍待接。

2026-10-17分组状态原生阶段：ABI1.54/GROUP_STATE及显式state20准备入口、完整关闭配置/准确票据/图片来源和冷重建、mutate采用及真实两笔/载色/历史通过。公开新2/原生1/旧保护5/旧公开3及Windows Release/Clippy/4 ABI/旧Qt2通过，见[设计](brush-groups-abi-design.md)及[验收](brush-groups-abi-validation.md)。state/VBR不变；旧四准备/包保护保留，分组包专用原生交接/组织命令/Qt及重置基准仍待接。ABR纹理执行映射仍未支持，兼容报告与来源保留不等于已执行。

2026-10-17分组包原生阶段：ABI1.55/GROUP_PACKAGE及ordered-package-v1完整交接、2MiB与投影验证、容量探测无资源安装、实际图片/ABR来源读写与原子保存、state20专用准备串联已接。IO33/新原生1/旧保护5/旧公开3/Schema及Windows Release/Clippy/4 ABI/旧Qt2通过，见[设计](brush-groups-package-design.md)及[验收](brush-groups-package-validation.md)。state20/VBR1.24不变；旧四包保护保持，Qt副本树合并/完整镜像/重置基准及分组管理仍待接，不宣称用户分组已完成。

2026-10-18 ABR纹理来源阶段：来源记录接受`drawverse.abr.basic` converterVersion8；`brushkit-abr 0.7.3`用于描述符参数，本地有界解码器负责8/16位raw/RLE `patt`。纹理参数映射到既有state/VBR资源模型，最新Schema的来源枚举加入8。其余未定义模式与16位笔尖仍明确未执行。

2026-10-18 VBR1.25颜色加深纹理阶段：`TextureMode::ColorBurn`使用`1-(1-base)/texture`的覆盖率标量近似，纹理对象mode增加`colorBurn`；包含该模式时manifest最低minor25，24及以下明确拒绝。VBR1.25 Schema `contracts/vbr-texture-modes-v1.25.schema.json`恢复纹理mode为必需字段并公开枚举；无state/ABI变化，其余未定义Photoshop纹理模式继续后续。

2026-10-18 VBR1.26纹理调整阶段：纹理对象新增可选`brightness`/`contrast`（-1..1，默认0），在采样后按亮度和中间灰对比度调整覆盖率；非默认值要求state21/VBR1.26，旧state/VBR缺省字节保持。Schema为`contracts/vbr-texture-adjustments-v1.26.schema.json`。ABR `textureBrightness`/`textureContrast`已映射；`minimumDepth`、深度动态、16位图案及其他未定义模式继续后续。

2026-10-18 VBR1.27纹理来源阶段：带非默认纹理的ABR来源记录升为version2，保存完整`TextureParameters`和资源身份；旧记录version1及原字节保持。包含v2来源或非默认深度动态时最低minor27，26及以下拒绝。纹理对象增加可选`minimumDepth`/`depthJitter`/`depthPressure`，按印章计算深度并以最低深度封底。Schema为`contracts/vbr-texture-source-v1.27.schema.json`，Qt来源恢复入口已接。

2026-10-18 VBR1.36高级纹理模式阶段：纹理mode新增`linearBurn`、`colorDodge`、`linearDodge`、`overlay`、`softLight`、`hardLight`、`exclusion`、`hardMix`，覆盖率组合为标量近似；非默认新模式要求state30/VBR1.36，35及以下拒绝。Schema为`contracts/vbr-texture-modes-v1.36.schema.json`，ABR `BlnM` 代码`linearBurn`/`CDdg`/`linearDodge`/`Ovrl`/`SftL`/`HrdL`/`Xclu`/`hardMix`已映射。其余未定义模式继续后续。

2026-10-18 VBR1.37纹理模式补全阶段：纹理mode新增`vividLight`、`linearLight`、`pinLight`、`divide`，覆盖率组合为标量近似；非默认新模式要求state31/VBR1.37，36及以下拒绝。Schema为`contracts/vbr-texture-modes-v1.37.schema.json`，ABR `BlnM` 代码`vividLight`/`linearLight`/`pinLight`/`blendDivide`已映射。Dissolve/Hue/Saturation/Color/Luminosity等颜色或随机模式仍不执行。

## VBR1.30：速度动态

`parameters.speedDynamics` 使用 `speed-v1`，必填 `enabled`、`maxSpeed`（1–100000 px/s）以及 `size`/`flow`/`opacity` 三条控制曲线。每条控制包含 2–17 个 `(0..1,0..1)` 节点、0–1 最低比例和可选 `smooth-pchip-v1`；缺省插值保持线性。关闭状态也完整保留。对象要求 `drawverse.brush`、behavior19、`speed-dynamics` 能力和 manifest minor30，state24；旧状态不写入、不升级版本或改字节，低 minor 夹带明确拒绝。

速度输入来自文档像素距离与输入段时间，曲线横轴按 `speed/maxSpeed` 归一化。尺寸作用于主笔尖和配对第二笔尖，流量与整笔不透明度上限来自同一冻结曲线；不影响随机序列、选区、蒙版或历史边界。Schema为 `contracts/vbr-speed-v1.30.schema.json`。ABR 导入不将速度字段映射为新对象，完整动态矩阵和跨平台验收继续后续。

## VBR1.31：切向压力动态

`parameters.tangentDynamics` 使用 `tangent-v1`，必填 `enabled` 和 `size`/`flow`/`opacity` 三条控制曲线；曲线结构与速度动态相同，但不含最大速度。输入为设备切向压力 -1..1 归一化到 0..1，关闭状态完整保留。对象要求 `drawverse.brush`、behavior20、`tangent-dynamics` 能力和 manifest minor31，state25；旧状态不写入、不升级版本或改字节，低 minor 夹带明确拒绝。Schema为 `contracts/vbr-tangent-v1.31.schema.json`。第二笔尖自身仍不接受切向压力配置，ABR来源映射、完整动态矩阵和跨平台继续后续。

## VBR1.32：笔触距离动态

`parameters.distanceDynamics` 使用 `distance-v1`，必填 `enabled`、`maxDistance`（1–100000 px）和 `size`/`flow`/`opacity` 三条控制曲线。输入为当前活动笔触累计文档距离归一化到 0..1；关闭状态完整保留。对象要求 `drawverse.brush`、behavior21、`distance-dynamics` 能力和 manifest minor32，state26；旧状态不写入、不升级版本或改字节，低 minor 夹带明确拒绝。Schema为 `contracts/vbr-distance-v1.32.schema.json`。第二笔尖自身仍不接受距离配置，距离/渐隐 ABR来源和通用动态矩阵继续后续。

## VBR1.33：倾斜幅度动态

`parameters.tiltDynamics` 使用 `tilt-v1`，必填 `enabled` 和 `size`/`flow`/`opacity` 三条控制曲线。输入为设备倾斜二维长度除以90度归一化到0..1，设备无倾斜能力时为0；关闭状态完整保留。对象要求 `drawverse.brush`、behavior22、`tilt-dynamics` 能力和 manifest minor33，state27；旧状态不写入、不升级版本或改字节，低 minor 夹带明确拒绝。Schema为 `contracts/vbr-tilt-v1.33.schema.json`。第二笔尖自身仍不接受倾斜配置，ABR倾斜/渐隐来源和通用动态矩阵继续后续。

## VBR1.34：笔旋转动态

`parameters.rotationDynamics` 使用 `rotation-v1`，必填 `enabled` 和 `size`/`flow`/`opacity` 三条控制曲线。输入为设备旋转角按整圈归一化到0..1，设备无旋转能力时为0；关闭状态完整保留。对象要求 `drawverse.brush`、behavior23、`rotation-dynamics` 能力和 manifest minor34，state28；旧状态不写入、不升级版本或改字节，低 minor 夹带明确拒绝。Schema为 `contracts/vbr-rotation-v1.34.schema.json`。第二笔尖自身仍不接受旋转配置，ABR旋转/渐隐来源和通用动态矩阵继续后续。

## VBR1.35：笔触方向动态

`parameters.directionDynamics` 使用 `direction-v1`，必填 `enabled` 和 `size`/`flow`/`opacity` 三条控制曲线。输入为输入段运动方向角按整圈归一化到0..1，无位移时保持恒等；关闭状态完整保留。对象要求 `drawverse.brush`、behavior24、`direction-dynamics` 能力和 manifest minor35，state29；旧状态不写入、不升级版本或改字节，低 minor 夹带明确拒绝。Schema为 `contracts/vbr-direction-v1.35.schema.json`。第二笔尖自身仍不接受方向配置，ABR方向/渐隐来源和通用动态矩阵继续后续。

## VBR1.38：动态来源矩阵

`parameters.sourceMatrix` 使用 `source-matrix-v1`，必填 `size`、`flow`、`opacity` 三个来源；每个来源为 `all`、`speed`、`tangent`、`distance`、`tilt`、`rotation` 或 `direction`。`all` 表示六来源因子相乘，单选来源只取该通道对应曲线。对象要求 `drawverse.brush`、behavior25、`dynamics-source-matrix` 能力和 manifest minor38，state32；默认矩阵不写入，旧版本字节保持，低 minor 夹带明确拒绝。Schema为 `contracts/vbr-dynamics-matrix-v1.38.schema.json`。第二笔尖不携带独立矩阵，ABR动态来源映射继续后续。

ABR source converter9（`bVTy=3/6/8`设备来源）同样要求 manifest minor38；source info 的 converter 范围为1..9，converter9基线必须包含 `sourceMatrix` 及至少一个倾斜/方向/旋转对象。此约束不改变平包字节或第二笔尖格式。

## VBR1.39：纹理溶解

纹理 `mode="dissolve"` 使用确定性覆盖率二值抖动，要求 manifest minor39 和 state33；低版本明确拒绝。ABR `BlnM` 的 `Dslv`/`dissolve` 映射到该名称。Schema为 `contracts/vbr-texture-modes-v1.39.schema.json`。该模式是覆盖率近似，不是最终颜色空间的Dissolve图层混合。
