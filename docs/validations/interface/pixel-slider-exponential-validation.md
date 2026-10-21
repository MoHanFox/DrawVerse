# 大范围像素滑条指数调节验收

2026-10-21，Windows / Qt6.8.3 Release。设计见 [像素滑条](../../interface/pixel-slider-exponential-design.md)。

验证范围：0起点、正数、15000px、正负百万像素及±511px范围，端点精确、单调、正反映射、正负零点；真实鼠标点击轨道、键盘1px/Home/End、外部模型更新；真实负数输入与加号1px。共享参数控件小范围像素/百分比保持线性。另复核整数单位、主/第二笔尖角度圆度、资源列表滚轮及工具百分比；真实跨窗口笔刷编辑、预览、像素与保存恢复、液化选项事务。

软件与D3D11下7项均通过（含初始化清理各9 passed），无QML警告；日志 `artifacts/pixel-slider-software.txt` / `artifacts/pixel-slider-d3d11.txt`。独立模型测试逐页检查像素单位与跨度启用规则，401px直径回传200.5px半径、外部更新246px同步、液化15000px提交后再外部改37px同步。跨窗口笔刷原测试软件/D3D11均通过；旧液化事务软件通过、D3D11等待CPU frameRevision失败，另记在[统一验收](../planning/2026-10-21-remaining-fixes-validation.md)，不作为本项通过证据。

额外D3D11浮面板拖放3行功能通过，透明交换链提示仍存在，由U5跟踪；没有屏蔽提示。完整Release、Qt及四ABI目标构建通过、生成头一致，`git diff --check`通过。未跑全量、跨平台及实体数位板；无Rust/ABI/存储变化。
