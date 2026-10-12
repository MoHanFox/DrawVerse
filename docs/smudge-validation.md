# 涂抹及参数提示验收（2026-10-12）

按用户要求只运行最小定向回归，没有运行 tools/check.py 全量验证。

- 核心：`cargo test --manifest-path core/Cargo.toml -p paint-core --test smudge --test selection --locked`，13/13 通过。三项新回归覆盖真实搬色且不注入前景色、静止无历史、单笔历史 26、撤销/重做/取消、压感强度开关、手指绘画、1% 间隔、合成取样只写活动层、活动层偏移、选区写入限制、透明锁/完全锁及蒙版黑色隐藏和撤销；十项既有选区回归覆盖 AA/羽化/组合和画笔/擦除共享覆盖。
- FFI：`cargo test --manifest-path core/Cargo.toml -p paint-ffi --test async_session smudge_validation_copy_real_pixels_and_single_history --locked`，1/1 通过。88 字节结构、嵌套偏移 32、空请求/保留位/布尔/NaN/嵌套长度/非法模式/低间隔拒绝及序号清零，调用时复制、实际红色帧、单条涂抹历史与撤销。
- Qt：只运行 `smudgeSharedBrushParametersInputAndDelayedHelp`，Windows 原生窗口 100% 和 200% 缩放各一次通过（各含初始化/清理 3/3）。独立 R 工具、预设复用、参数编辑和复选框点击、1% 间隔保存/恢复、鼠标和带压力数位笔输入、实际蓝色帧/历史/撤销；所有工具参数复选框都校验说明非空且延迟 1000ms，真实移动鼠标验证 400ms 未显示、随后出现、离开隐藏。截图检查参数布局和 1% 间距。
- Rust fmt、核心/FFI all-targets clippy（-D warnings）、Release 构建及 cbindgen 头严格字节检查通过。C/C++ ABI smoke 已编译链接，本轮未执行。

日志：artifacts/smudge-native-1.txt、smudge-native-2.txt、smudge-build.log；截图：artifacts/smudge-ui-1.png、smudge-ui-2.png。生成输出不纳入 Git。

实现保留现有圆形笔尖；高级自定义笔尖尚未实现。涂抹参数在当前 client 生命周期保留，不新增跨启动配置。所有图层取样不支持蒙版目标并明确报错。最大笔尖受现有半径 256 限制，源暂存只占一个 dab patch，段预算计入额外取样；未做全量性能基准或真实硬件验收。
