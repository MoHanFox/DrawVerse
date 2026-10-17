# ABR基础参数转换与报告

在严格身份关联上转换sampledBrush/computedBrush：Dmtr/#Pxl、Angl/#Ang、Rndn/Spcn/Hrdn/#Prc映射原生尺寸、角度、圆度、直径间距与程序硬度。参照 [ag-psd字段定义](https://raw.githubusercontent.com/Agamnentzar/ag-psd/master/src/abr.ts) 核对wire单位；不移植执行引擎。缺失/错类型/错单位/非有限值或超出原生1–512px、1–50%间距等范围明确失败，不裁剪。

结果包含真实可编译BrushSettings、完整源预设描述符借用及逐字段报告（Mapped/Approximate/NotExecuted）。Mapped仅指该字段已按定义传入内核，不声明PS整体逐像素相同。非零角度保留数值并明确方向未完成PS像素核对；Intr=false/flipX/flipY=true及所有未映射动态/纹理/湿混参数逐叶字段NotExecuted。报告最多4096项/1MiB路径/递归32层，二进制键用hex标识；超限明确失败，不隐藏多出的字段。原值保留在完整来源树。

默认基础转换不凭空开启压感。缺失工具流量/不透明度则显式记录100%近似；有toolOptions.flow按百分比映射；Opct非100%不能套用印章opacity冒充PS整笔透明度，记录未执行，基线仍100%。只要存在Approximate/NotExecuted，调用方必须将结果视为部分转换，后续UI显示报告与来源，而非完整兼容成功。

转换先整包生成所有参数并用真实BrushProgram编译，失败不返回部分列表，无磁盘/目录/ABI副作用。先完成基础桥，再接持久来源与UI；动态映射会逐步替换明确未执行项。验收：单位/范围/缺字段、程序/图片共享与真实像素、同名顺序、未知具体路径、非零角度/翻转/spacing-off/工具opacity区别、整包失败/取消，公开外部诊断参数核对（资产不分发，未声明PS渲染等价）。

2026-10-18 converter9补充：`bVTy=3/6/8`的尺寸/流量/不透明度控制分别映射倾斜、方向、旋转动态和来源矩阵；`Mnm`/`minimumDiameter`沿用最低比例，渐隐与随机抖动继续逐字段报告。converter9要求handoff8，源记录允许1..9并要求矩阵基线，VBR source converter9要求minor38。
