# 画布90度旋转验收（2026-10-30）

实现：双向整数置换，交换宽高、DPI不变；整文档层树、隐藏/锁定、蒙版、置入源几何、选区一并旋转。单次Canvas历史保留前后选区，超内存历史预算沿原编码/暂存盘路径回退，未加入结果引用缓存白名单。内部暂存tag13扩展双选区，暂存不跨进程持久化；ORA/PNG格式不变。ABI1.94追加transform11/12和history47/48，DTO布局不变。

通过：
- `cargo test -p paint-core --lib quarter_turn`：4项，含非方形跨瓦片精确像素、四次还原、低驻留/最小合法历史预算的Encoded命令与历史跳转、灰度选区、失败原子性、隐藏锁定嵌套蒙版和偏移、置入源身份。
- 核心canvas_resize 5项、history_travel 6项（有重叠）。
- 公开async_session双向旋转1项及原翻转1项：像素、单历史、撤销重做、参数验证。
- Qt软件及Windows D3D11：旋转尺寸/历史、两行菜单、画布调整各4功能行通过（含初始化/清理共6）；日志build/canvas-rotation-software.txt与canvas-rotation-d3d11.txt。
- paint-core/paint-ffi all-targets严格Clippy、fmt、生成头、四项C11/C++20静态/动态ABI通过。
- 标准build/bin/Release/drawverse.exe和Qt测试构建更新。

调查失败记录：首次测试使用非法1字节历史预算，改为最小合法预算后通过；首次ABI夹具沿用旧视口造成像素失败，改为按旋转尺寸请求并等待对应request_id后通过。最初fmt缺--all、生成器--help及构建目标名错误；首次Qt构建缺MSVC标准库环境，加载vcvars64后通过；第一次UI执行命中尚未重链接的旧测试程序，重链接后通过。初始Release测试使用旧非法夹具失败，最终核心定向结果为Debug。

未跑总check、性能或实体输入/物理屏幕验收。
