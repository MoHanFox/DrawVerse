# 选区右键操作验收（2026-10-30）

范围：滤镜首项调整子菜单；选区羽化、复制/剪切建层、填充、内部描边；所属画布客户端与模态窗口；单步历史及失败原子性。

新增定向检查：

- paint-core --lib selection_actions：颜色/透明度、负偏移、内侧描边、组合选区孔洞羽化、复制/剪切逐像素 coverage、重复撤销恢复、锁和瓦片预算失败、精确距离场/滤波、冷热分页与编码后备。
- paint-ffi async_session selection_pixels_public：公开入口非法参数、FIFO、五项操作单历史及撤销/恢复。
- Qt selectionContextOperationsModalCancelApplyAndSingleUndo：通过实际 Action 打开模态窗口；取消无历史，填充像素，复制/剪切层数与单历史，羽化不改工具参数，描边确认，错误/绑定循环检查。
- categorizedMenusHoverIntoCompactNativeSubmenus：调整首项归属滤镜、真实菜单与侧窗后备、旋转仍在编辑→图像；原 selectionRightClickMenuUsesOwningDocumentAndDeselects 回归。

初始失败保留：测试首版错误使用不存在的 Layer.locks（实际 appearance.locks）；偏移复制断言错误使用局部 pixel 而非 canvas_pixel，修正坐标后原覆盖断言通过。新窗口 minimumHeight 绑定 height 触发循环，改独立 preferredHeight。首轮 UI 新操作及原右键通过，级联两行在从旋转悬停文件时失败：测试从滤镜迁移后直接 invoke 编辑 open 未更新顶层跟踪，改真实点击编辑入口后复验。首次误在新测试链接完成前运行旧 EXE，报告 unknown test，非功能通过证据。

未运行总 check，不提供实体输入/物理显示延迟或新性能结论。工作区同期画布旋转与翻转/变换历史修改独立保留，不计为本次新操作实现。

补充失败及修复：低预算强制编码首次报 HistoryStorage（原 LayerEdit tag 11 只存一个图层数并要求同 ID），为完整森林新增内部 tag 14 和两份长度/结构校验，保留 tag 11 原约束；六项核心测试最终通过，包含驻留 1/16、历史预算 1024/131328、复制/剪切及三轮恢复。冷页测试首次用 Document::pixel 交替读取两层导致无意义逐像素换页，改逐层行读取，首轮耗时/失败不作为性能数据。首次新编码错误用了核心 Error 作为 io::Error，修正为 InvalidInput。尝试 --test history_travel 指向不存在集成目标，随后按 --lib history_travel 六项实际通过。

独立导出暂存变更到 build/SelectionOperationsStagedCheck：paint-ffi check、selection_actions 六项和生成头 --check 均通过，证明未依赖同期旋转工作区改动。第一次零上下文暂存补丁因排除同期行导致插入错位，隔离编译发现语法错误；改按 HEAD 原始行坐标精确构造暂存内容后重新独立检查通过。

最终 UI 首轮软件 7 passed（含五功能行及初始化/清理）；D3D11 同期六行通过，新操作行在 sampleDocumentPixel 断言失败。该入口读取主 CPU view0，而 GPU 按需策略可暂停该视图；测试改为抓取实际 Canvas 场景图并断言填充像素，不为测试强制 CPU 或更改生产策略。初始失败日志 selection-operations-d3d11-final.txt 保留，后续复验另存。

最终结果：

- 核心新增六项通过；原 selection_pixels 三项、--lib history_travel 六项通过。
- 公开 selection_pixels_public 一项（循环五种操作）通过；paint-core / paint-ffi --lib 严格 Clippy、fmt --check、生成头 --check 通过。
- 最终软件与实际 D3D11 各 7 passed（五功能行，含新操作、原右键、级联两后备、画布调整，另初始化/清理）：selection-operations-software-recheck.txt / selection-operations-d3d11-recheck.txt。确认实际画布抓图为白色填充及历史名称；保留初始失败日志。
- 四 ABI CTest 通过；标准 drawverse 与 qt_ui_tests Release 构建通过，标准 build/bin/Release/drawverse.exe 已更新。实际 D3D11 --preview 应用冒烟退出 0，selection-operations-app.png 与 out/err 日志保留在 build。
- 三个模态窗口抓图 selection-dialog-{feather,fill,stroke}.png；检查内部边距、标题、输入和底部确认取消。标准应用当时无运行占用，未关闭用户进程。

未做总 check、实体输入或性能宣称。独立暂存内容的代码/头检查与核心六项通过；标准运行包包含工作区同期已构建的其他改动，本提交仅包含选区操作及调整菜单迁移。
