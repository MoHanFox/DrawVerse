# 统一调整画布验收（2026-10-30）

实现与约束见 `docs/tools/canvas-resize-design.md`，ABI 真值为 Rust DTO 和生成头。

## 定向验证

- `cargo test -p paint-core canvas_resize --lib --locked`：4 项通过。跨瓦片裁剪、保留画外像素再扩展恢复、删除后不可恢复、默认白/所选色扩展、非法输入原子性、尺寸与 DPI 撤销重做、磁盘历史及注入写失败、组/隐藏锁定层/蒙版与历史跳转恢复。
- `cargo test -p paint-io --test canvas_resize --locked`：2 项通过。ORA 重新打开后保留画外内容及 300 DPI，PNG pHYs 的 300 DPI 往返。
- `cargo test -p paint-ffi --test async_session canvas_resize_publication_dpi_history_and_rejected_input --locked`：1 项通过。非法宽度拒绝且序号清零，真实 actor 发布尺寸与同 publication DPI，撤销恢复尺寸/72 DPI。
- `cargo clippy -p paint-core -p paint-ffi -p paint-io --all-targets --locked -- -D warnings` 通过；格式化与生成头检查通过。
- `qt_ui_tests` Release 构建通过。软件/offscreen 与实际 Windows/D3D11 的画布交互、选区右键和两种级联菜单共各 4 个功能行通过（含初始化/清理为各 6 passed）。真实边角拖动、取消无历史、扩展与 DPI 确认、撤销重做，以及默认不删除；右键绑定所属文档且不进入历史；图层/图像/选择进入编辑开头，旧裁剪/图像大小不再出现。日志 `build/canvas-resize-ui-final.txt` / `build/canvas-resize-ui-d3d11.txt`。
- 四种静态/共享 C/C++ ABI CTest 通过。`cargo test -p paint-core history_travel --lib --locked` 6 项通过，覆盖跳转/损坏回滚和调整画布恢复。

## 调查及限制

补充 UI 用例曾误用仅 QWidget 提供的 QTest::keyClicks，改成 QWindow 的逐键 keyClick 后重建。首轮 UI 用例本身通过，但菜单用例收集到错误的 QML Connections 信号（onFailure）警告；修正为 onErrorChanged 后复验通过。先前 `inputsValid` 警告为构建过程中两个资源版本不一致，最终一起重建后消失；首轮日志 `build/canvas-resize-ui-first.txt` 保留。第一次 QML validator 后的分号语法错误已修正，构建输出保留。共享 build 曾与另一任务的 Ninja 生成并发导致 failed recompaction/Permission denied，待其结束后重新构建。

宽泛的 `cargo test -p paint-core history --lib --locked` 选中了与此次修改无关的 saturated mix 全算子长测试；运行约 20 分钟后主动中断该测试进程，不作为整个 history 入口通过。未执行总 check、实体数位笔、所有手柄/DPR组合、大图性能矩阵或 JPEG/WebP DPI 验收。选区右键菜单当前五项实际能力与其余参考项边界见 `docs/tools/selection-context-menu-design.md`。

标准 Release EXE 正在运行而保留，交付独立 `build/bin/CanvasResizeReview/drawverse.exe`，静态链接本次核心与 UI。
