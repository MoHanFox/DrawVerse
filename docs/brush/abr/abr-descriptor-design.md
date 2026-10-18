# ABR描述符类型读取

从有界desc来源字节构建保留键顺序/类型/单位的树，为后续参数映射和兼容报告提供依据，不将未知值静默转换为默认值。wire规范依据 [Adobe Descriptor结构](https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/)；raw data长度和32位单位float与参考结构边界另核对 [ag-psd独立实现](https://raw.githubusercontent.com/Agamnentzar/ag-psd/master/src/descriptor.ts)。独立Rust实现，不移植源代码。

包含Unicode类名/文本、原始字节classID/key、Objc/GlbO、列表、double/带单位double或float、32/64整数、bool、enum、class/global class、alias/raw/path有界原始字节；标准reference结构按类型保留。未知OSType/引用类型明确Unsupported，不猜长度。单位保留四字节原码，未知单位不赋予执行语义。非有限数、非0/1布尔、重复字典键与非法Unicode拒绝。

16MiB描述符字节、深度32、总节点65536、每容器4096项、class/key最多1024字节、Unicode8192码元；前缀version16、完整根对象与尾随数据均校验。每递归节点检查取消。解析不读取alias/path指向的文件，不修改资源或目录。

验收：所有已支持值类型/嵌套/单位/二进制键/中文、Reference身份顺序、全部截断、重复键、未知标签、畸形布尔/浮点/Unicode、深度/节点/长度限制、尾随/版本/取消。生成协议夹具不等于授权真实ABR兼容语料；语义映射、程序笔尖和动态仍须后续实现验证。
