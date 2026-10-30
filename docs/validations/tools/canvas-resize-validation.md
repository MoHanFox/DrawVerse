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

## 最终收尾复验（2026-10-30）

在暂存提交 4c44cea 上完成最终 Release 重建并重新静态链接独立包。新增命令序号回执的最终资源与 C++ 一致加载：软件及实际 Windows/D3D11 各 6 passed / 0 failed（四个功能行），包含比例 0:0、交换、DPI 输入未失焦直接 Return、真实边框拖动、取消/确认/撤销重做、选区右键及两种级联菜单。日志 `build/canvas-resize-ui-receipt-software.txt` 与 `build/canvas-resize-ui-receipt-d3d11.txt`；旧日志保留。

`build/bin/CanvasResizeReview/drawverse.exe --preview ...` 在 D3D11 环境下完成三笔绘画与窗口抓图，退出 0，产物 `build/canvas-resize-app-smoke.png`。标准正在运行的 EXE 未替换。该冒烟不代替实体输入或所有调整组合验收。

工具栏对齐续修：交换按钮的原生隐式高度抬高布局，取消/确定亦使用了未限定高度的默认按钮。现统一在36px栏内垂直居中，三按钮固定26px；取消/确定为close/check矢量图标加文字。参数区可横向滚动，为右侧操作留固定空间。Qt用例补充按钮/单位/DPI的中心及上下边界断言，并确认contextDistort/contextPerspective不存在。软件与实际D3D11各4 passed（两功能行），日志canvas-resize-alignment-software.txt与canvas-resize-alignment-d3d11.txt，截图canvas-resize-alignment.png；独立CanvasResizeReview已重新链接。

冷历史续修：调整画布进入现有 prepare_result_in，Canvas 命令引用前后不可变瓦片，计账尺寸/选择/图层元数据与瓦片引用；像素按文档 PagePool 的驻留/暂存预算换出，不额外编码完整历史。元数据超预算或存在非本池瓦片时仍走原编码后备。新增1/16驻留瓦片、保留/删除裁剪的重复跳转测试，检查 Arc 身份、注册瓦片不增长、暖缓存零额外 page read、尺寸/DPI/选区及驻留上限；故障测试显式降低元数据预算以覆盖编码后备。首轮不合理1字节历史预算触发既有减法下溢，改用有效1024字节；并发 scratch maintenance busy 改串行复验，未据此更改生产缓存策略。

最终串行 canvas_resize 5项、history_travel 6项及 paint-core --lib 严格Clippy通过；保留初始失败记录，不作端到端耗时或全量验证结论。


## 2026-10-30 三侧描边、调整草稿隔离历史与导航验收

- 核心 `cargo test --manifest-path core/Cargo.toml -p paint-core --lib selection_actions -- --test-threads=1`：7 passed。新增内/外/中覆盖范围、中心空白、原图恢复验证；既有羽化、填充、复制/剪切、冷页和编码历史保持。
- 公开 `paint-ffi --test async_session selection_pixels_public`：1 passed，循环七种 kind；非法值改为 7，FIFO、单历史及撤销/重做通过。ABI 1.96 追加两种描边 kind，结构不变。
- `paint-core` / `paint-ffi --lib` 严格 Clippy、全工作区 fmt --check、生成头 --check 通过；四 ABI CTest 全通过。
- 最终 `selection-resize-existing-controls-{software,d3d11}.txt` 各 5 passed（3 功能及初始化/清理）。菜单测试改为真实右键菜单项与实际弹窗按钮鼠标点击，确认填充显示像素及历史；调整测试涵盖拖动、参数独立撤销重做、Ctrl+Z/Ctrl+Shift+Z、原工具条按钮、中键/空格平移、滚轮缩放、取消不改文档历史、确认单条冷历史与 DPI 恢复。断言无额外历史列表或调整栏撤销按钮。
- 初始失败日志保留：首次导航测试使用修改前的 CanvasItem 对象，禁用绘画时动画缩放被停止，最终即时导航分支重编后通过；延迟 forceActiveFocus 抢走 DPI 输入焦点，改同步；空格由原 CanvasItem 全局过滤器先消费，调整层改观察既有 spacePanning 状态后通过。公开头注释更新后 --check 曾报 drift，重新生成后通过。
- 用户明确不需要新增历史界面，最终删除尝试中的悬浮列表和调整栏新按钮，仅复用原有工具条、编辑菜单和快捷键，草稿记录不混入文档历史。
- Release UI 与核心构建通过。标准 EXE 被用户现有进程占用，未关闭进程或覆盖该 EXE；独立 `build/bin/SelectionCanvasReview/drawverse.exe` 静态链接新 UI/核心并附运行依赖，实际 D3D11 `--preview` 冒烟退出 0，日志及 PNG 在 build/selection-canvas-review-smoke.*。

未运行总 check；未作实体输入、物理显示或新性能宣称。羽化/填充无反应的用户症状本轮未复现，用户随后确认正常；不把它描述为已定位的生产缺陷。
