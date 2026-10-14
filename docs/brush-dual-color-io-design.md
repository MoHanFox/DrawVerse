# 双色双笔刷保存与交换

2026-10-15：dualBrush新增可选color对象，model=separated-stroke-v1，enabled/useBackground/overflow三个布尔，order=second-behind/second-in-front，primaryBlend/secondaryBlend为27种绘画混合模式的稳定英文名称。所有字段必需，未知字段/模型/模式/null/错误类型拒绝；组穿透不属于绘画模式。前景/背景实际RGBA仍由工具起笔捕获，不写入预设。

state16/VBR1.14/behavior13与dual-color-stroke能力显式表达该对象。默认色彩对象省略，旧字节/版本保持；双笔刷和双色模块任意关闭时的非默认配置完整保留。相对尺寸/平滑/图片/动态组合取最高版本，旧state15或VBR1.13不得夹带新对象。原ABR converter基线不表达双色，当前用户编辑和原始来源独立，不能改写来源记录。

当前ABI1.47/Qt最多识别state15，原生准确准备、包资源接入和公开库包写入暂拒新色彩对象，防止成功后丢字段；下一阶段完整接通后解除。无ABI变化。核心已有separated模型，非分隔逐印章模型不能写成此模型假装支持。

验收：双模块开关、第二色来源、前后/溢出、27模式及关闭配置完整往返；相对尺寸/平滑/压感组合，恢复后两笔真实有色半透明像素/历史一致；默认旧字节、旧格式和缺失能力/behavior/严格字段拒绝，Schema一致；图片和来源身份不丢；原生拒绝不发布、不改文件。对应最小回归后提交。
