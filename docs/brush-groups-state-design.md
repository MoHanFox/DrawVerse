# 分组笔刷状态保存

2026-10-17，接[目录模型](brush-groups-design.md)。本阶段实现可复用的有界组织DTO及state20保存/恢复，不冒充VBR分组包或Qt完成。VBR清单、整树UUID映射和原生/Qt在后续复用此DTO。

state20新增organization：model固定ordered-tree-v1、显式roots及groups记录。条目为kind= preset/group及id；组有id/name/items。JSON未知字段/模型/类型/null拒绝，列表反序列化按1280条目/256组上限读取，核心再验证总条目、深度、唯一成员、引用、UUID/名称和可达性。组织的DFS预设序列必须与presets行序一致；在加载图片/来源前验证，不能悄悄重排有歧义文件。空组完整保留，组记录数组自身不代表显示顺序。

有组才写organization并选择state20，平目录继续选择参数所需最低旧版本且字节保持。显式organization即使groups为空也必须state20，不能夹带到低版本；该空组织与等价平序列可规范化为旧版本，因为没有隐藏组/层级数据。所有关闭参数/图片三用途/来源及完整源档案仍走已有严格读写，不能因为新增元数据重编译来源或重置笔刷。

旧原生Legacy/Wet/Taper/WetDual准备入口在采用前拒绝有组目录，当前ABI/Qt尚不能镜像，不允许通过新Rust解码器使旧Qt悄悄丢组。无新ABI或VBR版本，本阶段不改Package/manifest。旧未知groups根字段仍拒绝；新organization仅明确20版本。

验收：空组/嵌套/同名及显式顺序往返、旧平字节/解组与根重排、非法类型/降级/歧义顺序/环和各预算边界在资源访问前拒绝；实际图片/纹理/第二图片/湿混双色/起收笔/关闭值及来源跨新目录保存恢复、两笔像素/双载色和历史；旧原生四入口拒绝且当前目录不变。Windows必要回归/Release/ABI及旧Qt目录，整体手感用户验收、跨平台后置。
