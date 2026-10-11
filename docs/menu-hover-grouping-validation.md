# 菜单悬停与多选建组验收

## 修复与专项验证

- 共享 CompactComboBox 使用列表上的 HoverHandler 统一计算目标行，避免相邻行进入/离开相互清除预览；打开、列表变化及恢复启用时也更新。显式锚定指针坐标到列表视口，保留真实 CanvasItem 连接的 Qt 回归在停靠与浮动面板中覆盖首次悬停、连续换行、静止鼠标下列表重新定位、关闭恢复、点击单次提交，整个过程不发送滚轮事件。
- 两处新建组入口共享 PaintCoreClient.groupSelectedLayers，ABI 1.14 在 actor 中一次建组。Rust 覆盖非连续选择的原顺序、蒙版和像素保持、跨父组的最近公共父级、祖先/后代去重、内外剪贴基底、非法/锁定选择、最大深度及 ID 原子性。大量元数据使用磁盘历史时可完整撤销/重做，暂存写入失败保留结构、活动图层、修订、redo 和后续 ID。
- 新 ABI 回归验证 ID/名称立即复制、无效指针清零 sequence、实际父级与名称、一次历史和撤销/重做。现有 ABI 符号与 DTO 保留，头文件由 cbindgen 生成。
- Qt 6.8.3 原生鼠标路径复现首行向右移动 3–4 个逻辑像素即关闭父菜单，旧测试仅直接移到子窗口，漏掉了弹窗抓取后的路径。Qt 的原生弹窗转发修改 scenePosition 而保留原 position；现在共享 WorkspaceManager 在弹窗转发前按全局坐标重建事件，两个坐标一致。菜单从显示时即参与路由，包含创建过渡期，返回顶部切换菜单也重新映射。
- 主菜单栏显式启用 hover；沿弹窗的实际瞬态父窗口链查找所属菜单栏，不依赖可选的 Windows 背景层。离屏平台和独立宿主也保留正确的顶部菜单切换。
- 原生普通与两倍缩放专项各 7/7，含原生菜单与独立宿主回退、Qt Test 逐像素路径和真实 QCursor 原生路径、返回顶部切换、关闭/外部点击、置顶、两种面板预览和两处建组入口。已检查实际子菜单截图，没有文字或边界溢出。Qt 6.5 回退由 Qt 6.8.3 强制选择独立宿主验证；未安装实际 Qt 6.5 SDK。

## 最终验证（2026-10-11）

- `python tools/check.py` 完整通过：真实 Git 检出/工具测试、Rust fmt / Clippy / workspace 全目标与文档测试、Release 构建、生成头严格字节检查、C11/C++20 静态/动态 ABI 链接运行；CTest 6/6，无失败或跳过。
- Qt 6.8.3 普通回归 86/86，两倍 DPI 57/57；Windows 原生普通与两倍缩放专项各 7/7。Rust layer_batch 13/13，新增暂存失败单元回归和 ABI 批量建组回归也在完整检查中通过。
- `cmake --build build/qt --config Release` 部署当前程序和 ABI 1.14 DLL。实际 `drawverse --preview` 启动、三笔绘制、截图和正常退出通过；已检查主窗与子菜单截图。

诊断：`artifacts/hover-group-release-check.log`、`artifacts/hover-group-native-release.txt`、`artifacts/hover-group-native-dpi-release.txt`、`artifacts/hover-group-release-app.png`。诊断和构建输出不提交。
