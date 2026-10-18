# ABR 采样笔尖读取

开始以ABR1/2采样记录建立真实二进制读取层，再继续6/10分节与描述符、程序笔尖和动态映射。输入大端、有界顺序reader；记录的声明长度就是子reader边界，禁止跨记录读取或静默跳过未知类型。首阶段不提供用户ABR导入按钮，不把采样读取冒充完整PS参数转换。

格式依据：GIMP上游 [ABR加载器](https://raw.githubusercontent.com/GNOME/gimp/master/app/core/gimpbrush-load.c) 的字段布局核对（独立Rust实现，不移植源代码）；Adobe [文件格式规范](https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/) 确认按行PackBits。AdobePSD规范不作为完整ABR规范。

1/2文件头version/count；记录type/length；采样包含misc、spacing、v2 UTF16名称、AA、短/长边界、depth、compression、像素。当前执行8位覆盖，0原始/1逐行RLE；基于上游直接mask用法保留覆盖方向，不能先验地反转所有ABR。16位/程序/未知类型明确暂不支持。保留原始间距与名称，当前UI间距上限不能静默裁剪。

输入128MiB、最多1024记录、图片4096边/16MiB原覆盖、整包mip驻留128MiB；UTF16最多1024码元。长度、行计数、解压输出、尾随数据、非法Unicode严格校验。每行只能产生声明宽度，no-op允许；失败/取消不返回部分样本。样本只有不可变mask和来源元数据，不写磁盘或目录。

验收：独立生成有明确来源的二进制v1/v2样本（原始/PackBits）、Unicode/名称/间距、准确覆盖与真实孔洞绘画、截断/行超限/边界溢出/未知记录/过大数量/尾随/取消整包拒绝。生成样本不是PS真实兼容样本；合法授权真实ABR语料与PS对比仍是后续兼容门槛。
