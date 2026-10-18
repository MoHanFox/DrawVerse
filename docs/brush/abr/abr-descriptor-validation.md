# ABR描述符读取验收

2026-10-13：有界version16描述符typed tree已实现并接入SectionedAbr.desc显式读取。保留class/key字节、项顺序、单位原码、Objc/GlbO区别、列表、文本、双精度/单位双精度/单位float、32/64整数、bool、enum、class/global class、alias/raw/path原始有界数据、7类引用；不读取路径文件或执行未知参数。

3项描述符、1项ABR桥与2项分节回归通过：18项异构类型（含二进制键/中文/嵌套/引用顺序）、逐处截断、非有限浮点/非法bool/Unicode、重复key/未知tag/尾随/版本/取消、递归/容器/累计节点/文本上限、采样像素与desc独立关联。paint-io全目标Clippy -D warnings通过；日志 `artifacts/abr-descriptor-tests.log`。

协议样本由测试独立生成，不是Photoshop真实文件。path/alias是原始块，未据此宣称其内部OS路径结构已解释。对象数组ObAr及未知类型仍明确Unsupported，原始desc在ABR来源分节中保持。此阶段没有Qt/ABI入口修改，未运行完整仓库检查。

仍需真实授权ABR语料、样本身份/预设关联、具体PS参数/程序笔尖/动态映射与报告、来源持久化及用户导入；不能将通用描述符读取成功视为完整ABR或动态支持。
