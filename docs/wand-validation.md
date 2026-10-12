# 魔棒验收（2026-10-12）

按用户要求只做定向最小验证，未运行 `tools/check.py` 全量测试。

已实现四种选区组合、点取样及 3/5/11/31/51/101 方形平均、容差（默认 10）、AA/连续（默认开启）、所有图层取样（默认关闭）。W 激活；鼠标和数位板按下只选一次，Shift 添加/Alt 减去/Shift+Alt 交叉；主窗与浮动画布共用 client。魔棒图标为矢量杆和星光，工具列、参数栏与历史共用。取样平均在画布边缘裁切，按实际样本计数。

核心定向测试：`cargo test -p paint-core --test wand --test bucket --test selection --locked`，17/17 通过。新魔棒两项测试覆盖四邻接与全局不连通区域、四种组合、历史 22、撤销重做、像素不变、七种取样大小与边缘平均、活动层正负偏移、合成取样差异、锁定图层上选择、半覆盖 AA 与关闭 AA、灰度选区文本往返、无效参数与超大文档全局拒绝。既有桶和选区的真实画笔/橡皮限制、羽化/组合/缓存回归继续通过。

FFI 定向测试：`cargo test -p paint-ffi --test async_session wand_request --locked`，1/1 通过。48 字节 DTO、空指针/结构长度/尺寸/容差/布尔/operation/reserved 拒绝、序号清零、调用时复制、实际掩码查询/容量需求、publication 失效、修改标志、单条历史与撤销、越界异步报错。caller-memory 仍仅由 pointers.rs 访问。

Rust fmt、核心/FFI all-targets clippy（warnings 为错误）、paint-ffi Release、生成头严格字节检查及 Qt Release 构建通过；C11/C++20 静态/共享 smoke 已编译链接，此轮未执行其测试。

Qt 只运行 `wandOptionsSelectRealMaskOnceAcrossMouseTabletAndFloatingCanvas`：原生 Windows 普通/两倍 DPI 各一项（含初始化/清理各 3/3），无失败或跳过。验证默认参数、七项取样菜单、参数修改与无效值拒绝、三个实际点击开关及组合按钮、W 激活、实际圆形像素岛选区、包围框角落不被轮廓包含、鼠标拖动不重复选择、撤销、数位板与独立浮动画布、切换套索不污染记忆形状。截图检查真实圆形轮廓及参数布局，日志 `artifacts/wand-native-1.txt` / `artifacts/wand-native-2.txt`，截图 `artifacts/wand-ui-1.png` / `artifacts/wand-ui-2.png`。

本轮限制明确保留：工作集 4096 访问瓦片、全局/包围掩码 16Mi 像素及选区编码 1MiB 上限，超限报错不留下部分选区；AA 使用共用颜色阈值的一色阶覆盖过渡。灰度掩码持久化采用 g 前缀 RLE，旧二值编码仍兼容读取，新编码在旧核心上明确拒绝。参数保留于 client 生命周期，不新增跨启动保存格式。
