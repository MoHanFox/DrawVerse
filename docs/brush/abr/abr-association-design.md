# ABR样本与预设关联

基于 [ag-psd上游ABR读取与测试结果](https://github.com/Agamnentzar/ag-psd/tree/master/test/abr-read/sample-and-pattern) 核对字段：根Brsh列表包含预设，预设Brsh对象声明sampledBrush/computedBrush；sampledData文本对应samp Pascal字节身份。现代sample前缀长度应随身份长度变化（再加10/264字节元数据），不固定假定所有身份36字节。

关联层保留预设与shape的完整描述符引用、原名称和共享不可变覆盖；只剥除一个文本末尾NUL，拒绝内嵌NUL/空身份/重复样本身份/缺失sampledData引用/畸形字段，不按索引猜配对。computedBrush不强加图片，其他形状类型明确尚未支持。descriptor-only容器允许在没有samp时读取来源，语义成功仍须经过desc解析与关联。名称和具体执行参数留到下一转换阶段，关联不代表动态/纹理已经执行。

上游MIT许可明确排除图片/笔刷资产：[许可](https://raw.githubusercontent.com/Agamnentzar/ag-psd/master/LICENSE)。外部公开src.abr仅存ignored artifacts用于显式本地诊断，不作为授权可分发语料。记录来源/hash/诊断结果，生成可提交协议夹具；正式兼容语料与参数转换仍须继续。

验收：可变长度身份/重复拒绝、多个同名预设共享资源而不依序配对、UTF16末尾NUL规范化、缺引用/重复/类型拒绝、computed无图片与完整字段保留、取消/预算；外部上游样本(ABR6.2，112×105覆盖)读取与真实映射核对，不据此宣称完整PS兼容。
