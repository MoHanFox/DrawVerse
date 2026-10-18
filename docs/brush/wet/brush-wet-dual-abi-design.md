# 湿混双色原生入口 ABI1.52

增加WET_DUAL_COLOR能力(bit47)及明确的prepare_state_wet_dual、library_package_read_wet_dual/write_wet_dual入口，接受完整state19/VBR1.23和此前子集。准备Recipe记录新的支持级别，旧Legacy/Wet/Taper仍拒绝新策略（包括关闭值），旧票据和图片资源修订重建必须保留支持级别。不改变既有DTO或导出布局。

既有prepared_preset_with_blend和两色捕获使用完整新定义，经Engine/Document双载色执行；不再新增笔触句柄或复制绘画算法。包探测/错误输出指针不安装资源，写入在完整验证后原子保存。

新增80字节PaintWetToolInfoDual：struct_size/flags(0/4)、primary/secondary_revision(8/16)、primary/secondary_edge(24/28)、primary/secondary_center_linear_premul_rgba(32/48)、`reserved[4]`（64字节）。flags复用INITIALIZED=1并增SECONDARY_INITIALIZED=2，原40字节info继续只读主角色。新info一次快照两个角色，BUSY/失败清零完整输出（保留struct_size）。

新增reload_prepared_dual：复用原32字节PaintWetLoadDesc（临时主半径和前景RGBA）及24字节PaintBrushBackgroundDesc。两描述符全部验证，再使用准确prepared修订；活动工具BUSY不等待编译。有效湿混双色按捕获策略选背景/前景作为第二色，并用核心共用的标称尺寸映射原子装载两角色；普通湿混仍只装主角色、保留未使用的副载色。disabled湿混拒绝；旧reload_prepared对新策略明确Unsupported，不能只装主笔冒充完整装载。Clean沿既有整工具清洗两角色。

验收：5个新导出及C/C++布局/静态共享调用；公开C ABI准确准备/关闭值/真实两笔像素及两载色、手动尺寸/颜色和Busy/失效/错误指针、跨会话及销毁令牌保留捕获、包图片三用途/旧修订重建及不完整输出不安装资源，旧入口拒绝和旧接口像素保持。仅Windows，Qt镜像/组合下一阶段，state/VBR不再加版本。
