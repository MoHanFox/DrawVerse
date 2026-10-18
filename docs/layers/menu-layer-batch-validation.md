# 菜单分类与图层批量修复验收（2026-10-11）

行为与边界见 [设计](menu-layer-batch-design.md)，正式接口见 [ABI 1.13 契约](../../contracts/abi.md)。

## 修复与验证范围

- 多选原来只用于高亮，删除/拖放仍传单 ID；现在经共享 PaintCoreClient 批量接口提交一条核心命令。选中新组/新层会同步选中集合，避免删错旧选择；拖动选中行不覆盖多选，拖动释放不再触发单击重选。
- 新核心回归验证非连续多选顺序、蒙版随所有者、祖先/后代去重、循环/锁定/最后像素层拒绝，以及组删除的两个模式。多选删除保存移除节点及前后结构；磁盘历史撤销恢复像素，模拟暂存写入失败保留文档与 redo。
- ABI 输入即时复制、1–4096 有界且去重，非法参数清零 sequence；不存在 ID 的异步失败不部分修改。既有单层拖放也使用同一剪贴继承规则。
- 图像调整/画布旋转及滤镜模糊使用共享侧向悬停菜单；明确开启 hoverEnabled，固定 6×8 逻辑像素的箭头。原生窗口置顶与 Qt 6.5 独立宿主回退都纳入测试；菜单链外部点击统一关闭。
- LayersPanel 原来由同步 Loader 创建；现改为异步孵化，图层行保持 ListView 虚拟化，缩略图仍在后台生成。80 个真实图层首开测试测量打开调用与事件循环延迟，并限制实例化行数；缩略图变化不重建模型。
- 已检查实际分类子菜单与组删除对话框截图：`artifacts/menu-layer-batch.menu.png`、`artifacts/menu-layer-batch.group.png`（均为本地诊断产物，不提交）。

## 最终验证

- `python tools/check.py` 完整通过：6 项真实 Git 检出/工具测试、Rust fmt / Clippy / workspace 全目标及文档测试、Release 构建、生成头严格字节检查，以及 4 项 C11 / C++20 静态/动态 ABI 检查。CTest 6/6，无失败或跳过。
- Qt 6.8.3 普通界面回归 80/80，两倍 DPI 50/50。异步图层控件的既有测试等待实际创建；旧移动取消测试等待前一笔历史提交，避免把尚未完成的笔触计为取消操作新增历史。原有行为断言保留。
- Windows 原生渲染专项 6/6；原生两倍 DPI 专项 5/5。包含窗口菜单与独立宿主回退、相邻分类悬停、整条菜单关闭、原生置顶、批量真实 DropEvent、组删除三按钮、首次打开图层面板。
- 80 个真实图层首次打开：普通离屏测试调用 2ms、5ms 心跳最长间隔 12ms，仅实例化 5 行；两倍 DPI 和原生专项也保持 5 行。Windows 原生渲染调用 2ms、最长间隔 15ms；原生两倍 DPI 调用 2ms、最长间隔 16ms。这是本机回归测量，并非所有硬件的延迟保证。
- Qt 6.5 回退使用 Qt 6.8.3 强制关闭 windowPopupsAvailable 验证；没有安装实际 Qt 6.5 SDK，因此不声称已运行该 SDK。回退路径修正父菜单遗留定位偏移，路由原生弹窗抓取后的父菜单鼠标移动，并在创建时捕获字体/调色板，避免关闭菜单链时空对象绑定警告。
- `cmake --build build/qt --config Release` 完成可运行程序与 ABI 1.13 DLL 部署；真实 `drawverse --preview` 启动、三笔绘制、截图及正常退出成功。检查最终主窗、菜单和组删除对话框截图，无溢出或重复界面。

本地诊断：`artifacts/menu-layer-batch-final-check.log`、`artifacts/batch-native-windows.txt`、`artifacts/batch-native-highdpi.txt`、`artifacts/menu-layer-batch-final.png`。构建输出及诊断产物不提交。
