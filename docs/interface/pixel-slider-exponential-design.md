# 大范围像素滑条指数调节

所有像素单位且滑条跨度至少512px的参数统一使用指数轨道：笔刷库/设置大小、液化尺寸、起收笔像素长度、第二笔尖直径/相对直径偏移、描边像素宽度/笔尖大小、纹理水平/垂直偏移。没有滑条的数值框不增加控件；百分比、角度和小范围像素参数保持线性。

`MappedSlider.qml` 对外保留真实单位的 from/to/value，内部 CompactSlider 使用0–1轨道。映射为 `g(v)=sign(v)*ln(1+abs(v))`，归一化后用于显示；指针位置以反函数 `sign(g)*(exp(abs(g))-1)` 得到真实像素，再按实际步长量化、钳制。零起点和正负范围共用一套实现，两个端点精确返回，退化范围固定起点。1–2000px轨道中间约62px，1–15000px中间约172px；负偏移越接近零越精细。

提交使用临时 editedValue 发出 moved，模型仍是唯一持久状态，控件不赋值覆盖调用方的 value 绑定。左右/上下键每次一个实际步长，Home/End到两端；像素加减按钮仍为1px，精确输入保持原范围与校验。ParameterSlider 依据单位和范围选择映射，其他独立像素滑条显式启用。存储、核心算法、ABI/state/VBR均不改变，不增加GPU计算。

[验收](../validations/interface/pixel-slider-exponential-validation.md)
