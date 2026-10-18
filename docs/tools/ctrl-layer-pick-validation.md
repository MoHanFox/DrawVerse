# Ctrl 选层、搜索焦点与紧凑变换输入验收

2026-10-13，Windows / Qt 6.8.3，原生 windows 平台、software 渲染、缩放 1。

- `cargo test -p paint-core --test layer_pick --locked`：2 项通过；透明区域、空白、隐藏组、祖先位置锁、蒙版、剪贴基底、置入图片与单条移动历史/撤销/重做。
- `cargo test -p paint-ffi --test async_session pick_layer_queues --locked`：1 项通过；真实排队选层与移动、空白无历史、非法位移输出 sequence 清零。
- `cargo test -p paint-ffi --lib capabilities_report_only_real_current_features --locked`：1 项通过，ABI 1.24。
- `cargo clippy -p paint-core -p paint-ffi --lib --locked -- -D warnings`：通过。
- `cargo run -p paint-api-gen --locked -- --check`：生成头严格字节一致；Release Qt 构建通过。
- Qt `firstLayerSearchClickAndCtrlCanvasPickMove`：首次异步面板布局完成后真实点击搜索并输入/Esc，无须重开；橡皮工具下 Ctrl 点击透明穿透选层、拖动 6×3 像素、松开 Ctrl 后保持原工具、快捷键撤销/重做、空白不误移、框选工具下 Ctrl 选层不创建选区。
- Qt `placedImageFreeTransformRasterizeAndShortcuts`：既有变换/置入/栅格化回归通过；数值控件为 52×20、9px 字号，位置与斜切输入以及会话撤销/重做通过。

原生 Qt 报告 `artifacts/ctrl-pick-native-1.txt`：2 项功能用例通过（含 init/cleanup 共 4 passed），0 failed、0 QML warnings。仅执行以上相关检查，没有宣称完整仓库测试通过。

Ctrl 的数位笔路由复用同一开始/结束移动路径；本轮原生交互验收使用鼠标。移动与已有 V 工具相同，在释放指针时提交一条历史。
