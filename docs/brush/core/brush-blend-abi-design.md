# 画笔颜色混合原生捕获入口

2026-10-14，内核见brush-blend-design.md。本阶段ABI1.44新增PAINT_FEATURE_BRUSH_WRITE_BLEND及16字节PaintBrushBlendDesc：size/u32模式/u64 reserved，模式0..26、reserved=0；旧DTO及导出签名保持。新paint_session_begin_prepared_preset_with_blend传入现有tool/background/point与新blend参数，所有caller读取仍在pointers.rs，返回前复制拥有的颜色/模式。旧prepared入口显式正常，新入口拒绝组穿透及非正常擦除/涂抹，并在失败时清零sequence。

actor等待准确的准备票据，再把捕获模式应用到definition，不查询稍后工具/UI状态；库销毁/更新后已提交起笔仍持有票据。模式是临时工具参数，不进入BrushSettings/state12/VBR1.10，也不改变重置基准。

验收：大小/对齐/reserved/模式/工具及空指针失败原子性；真实Multiply/Screen与旧正常入口、两色及模式复制、准备修订/句柄生命周期、同笔上限/实际历史、透明锁/蒙版沿用内核；C/C++静态共享ABI、cbindgen严格检查。Qt工具栏/持久状态/跨文档及有色底预览后续接入，本阶段不展示未连接菜单。
