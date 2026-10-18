# 整条笔触描边保存交换

2026-10-19。对应[整条笔触描边](brush-stroke-outline-design.md)，本阶段只接预设参数、资源与交换；原生准备和Qt控件随后接入。

state41新增`strokeOutline`，VBR继续1.0写manifest，行为版本33、能力`stroke-outline`。默认描边继续省略且不提升旧state/VBR；关闭但仍选择来源、位置、宽度或第二颜色的非默认对象必须完整往返。VBR按实际字段执行，不以minor判断能力。

对象固定`model=stroke-contour-v1`，包含`enabled`、`placement`（outside/center/inside）、`secondaryColor`、完整`width`（相对标记、像素和百分比）和`source`。宽度在转换为f32前按f64校验：像素0..2000、百分比0..10000；未知字段、显式null、未知模型和非法组合均拒绝。

来源有三种：`self-brush`只含模型；`existing-brush`在`brush`中保存不可变第二笔尖快照，复用既有双重笔刷参数并提取其笔尖，不引用预设身份；`texture`在`texture`中保存完整纹理参数。已有笔刷图片和描边纹理各自使用内容ID，ImageBundle的manifest资产、预检和全新目录恢复必须同时覆盖主笔尖、纹理、双笔刷和描边资源。

普通BrushSettings起笔自动捕获`outline`；VBR `definition()`和state资源恢复在绑定资源后返回同一完整配置。纯文字VBR仍拒绝资源型对象；图片VBR按描边资源是否被引用做严格一致检查。ABR旧转换器来源基线无法表达描边，非默认配置明确拒绝写回来源记录，但用户当前预设仍可独立保存新配置。

本阶段不改变ABI，旧原生准备和VBR入口仍会拒绝state41对象，防止旧Qt镜像静默丢值。下一阶段增加显式原生入口，再按ABI状态与VBR资源完整捕获、重建和在途预设身份接入Qt。

