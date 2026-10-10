# 顶部菜单与首选项（2026-10-10）

用户要求把顶部菜单改成 `文件 · 编辑 · 图像 · 图层 · 选择 · 滤镜 · 窗口`（工作区并入窗口，删除视图），并在编辑下提供首选项，首选项用左侧分类侧栏，其中性能与历史两类各有真实设置，未实现的项目按"待支持"如实标注。

## 顶部菜单

- **文件**：新建画布、打开、保存、另存为 OpenRaster、导出 PNG（走现有导出对话框，按其过滤器可选 PNG/JPEG/WebP）、导出 WebP/JPEG、导出 TIFF/GIF（待支持）、置入（待支持）、退出。
- **编辑**：首选项…（Ctrl+K，`objectName:"storagePreferencesAction"` 保持沿用）、键盘快捷键…、撤销、重做。
- **图像**：图像大小、画布大小、裁剪、旋转 90°、翻转水平/垂直、拼合图像、使选区居中——全部仅列出并标注"待支持"，不可点击。
- **图层**：新建图层、删除图层、新建组、解组、添加白色蒙版、创建剪贴蒙版（都走现有 `PaintClient` 能力）；复制图层待支持。
- **选择**：保持原有矩形/椭圆选框、全选、取消选择、反向选择。
- **滤镜**：高斯模糊、锐化、杂色、风格化、像素化、扭曲、渲染——全部"待支持"。
- **窗口**：工具条归位、浮动画布、画布返回工作区、适合窗口（F）、实际像素、面板显示开关、自定义面板…、保存当前布局、默认布局（原"工作区"菜单内容整体并入，`objectName:"workspaceMenu"` 与"工作区"入口删除）。
- **视图**菜单删除；"适合窗口/实际像素"移入窗口菜单。

真实可用的项都接到既有实现，未实现的一律 `enabled: false` 并在文案里写"待支持"，不出现看似可用但点了没反应的项。

## 首选项对话框

`ui/qml/PreferencesPanel.qml`（`objectName:"preferencesDialog"`，入口是编辑→首选项…，快捷键 Ctrl+K）：

- 左侧分类侧栏：**性能 / 历史 / 界面**，选中项高亮，右侧只显示该分类内容，分类之间用分隔线。
- **性能**：像素驻留缓存、单文档瓦片暂存上限、保留磁盘空闲空间、暂存目录（沿用既有 `PaintClient.storageSettings` / `saveStorageSettings` / `inspectStorage`，保存后下次启动生效，与旧"性能与暂存盘"对话框同一套配置）；其下"GPU 与多屏合成""内存上限（文档自动回收）"标注待支持。
- **历史**：最大记录（60–1000，步进 10）+ 应用按钮，落到真实的历史条数上限，立即作用于当前文档；其下"记录条数按总字节上限（自动压缩）"标注待支持。
- **界面**：主题与强调色、界面缩放与字体、快捷键自定义、数位板映射——全部标注待支持。
- 底部："标注"待支持"的项目尚未实现，不能设置。"、保存设置（写入历史上限 + 存储设置）、关闭。

待支持条目由内联组件渲染，显示灰字 + 独立"待支持"标签，不带任何交互控件。

## 历史条数上限（ABI 1.11）

`paint-core` 的 `DocumentOptions::max_history_commands` 原本只能在创建文档时确定，因此新增真实可用的运行时上限：

- `paint-core`：`History::apply_command_limit(limit)` 与 `Document::set_max_history_commands(limit)`——合法范围为正值，笔触录制中拒绝（返回 BUSY）；下调时立即裁剪最旧的撤销记录、丢弃重做分支，并标记边界不可达（沿用既有 `evicted` 语义，因此裁剪后不再显示"初始状态"，与第 30 条一致）。
- `paint-ffi`：新增 `paint_session_set_history_limit(core, session, max_commands, out_sequence)`，作为一条命令入队、在会话 worker 上按序应用；ABI 小版本 1.10 → 1.11（只新增函数符号，不改任何 DTO 布局，向后兼容）。
- `PaintCoreClient`：新增 `historyLimit` 属性（10–1000，界面限制 60–1000）与 `applyHistoryLimit()`；上限持久化到既有偏好文件 `history/limit`，启动时读取，并在会话建立后立即应用，使本轮的第一份文档也生效；非法值在写入前收敛。

## 键盘快捷键对话框

编辑 → **键盘快捷键…** 打开 `ui/qml/KeyboardShortcutsDialog.qml`（`objectName:"keyboardShortcutsDialog"`），只读展示当前**实际生效**的快捷键，按类别分组：文档（Ctrl+N/O/S/Shift+S）、编辑（Ctrl+Z、Ctrl+Shift+Z、Ctrl+K、Ctrl+Alt+G）、选择（M、Shift+M、Ctrl+A、Ctrl+D、Ctrl+Shift+I）、工具（V、B、E）、视图（F、Esc）、画布（空格拖动平移、Alt＋滚轮缩放、Ctrl＋拖动调笔刷大小）、数位板（CapsLock 轮廓/十字）。内容与 `Main.qml`、`CanvasPane.qml` 里真实绑定的 `Shortcut` 一一对应，不改任何现有绑定；页脚写明"自定义快捷键（待支持）"，不提供看起来能改却不生效的控件。

## 验收

- 定点 Qt 用例（offscreen + 软件渲染）：`defaultLayoutRestoresTwoDockedColumnsAndMenuEntry`、`compactStatusReflectsDocumentAndZoomControlsPreserveCenter`、`topMenusUseNativeWindowsAboveFloatingPanels`、`menusReopenWithoutGhostsAndFloatBesidePanels`、`storageSettingsDialogShowsAppliedAndPendingValues`、`invalidStartupStorageCanBeCorrectedFromQml`、`historyUsesRealToolLabelsAndHidesEvictedInitialState`、`editMenuOpensKeyboardShortcutList` 全部通过。`menuEntry:<标题>` 断言更新为新菜单集合。
- `editMenuOpensKeyboardShortcutList`：编辑菜单里的动作可触发并打开对话框；`shortcutList.count == 22`（与真实绑定表一致）；再用 `grabToImage` 对列表区域取像素，要求渲染出的亮像素超过阈值，避免"只报模型不画行"的假通过。
- `menusReopenWithoutGhostsAndFloatBesidePanels` 原先依赖键盘 Escape 关闭顶层菜单；offscreen 平台无法抓取键盘（QWARN "does not support grabbing the keyboard"），改为显式 `close()` 并断言菜单确实消失，避免把平台限制误判成失败。
- Rust：paint-core 新增的历史上限裁剪逻辑随 `cargo check`/`cargo test -p paint-core` 覆盖（详见验收文档）。
- 构建与部署：`cmake --build build/qt --config Release --target deploy_qt` 通过，`build/qt/bin/Release/drawverse.exe` 已更新并启动冒烟。

## 已知未完成

图像、滤镜两类的具体功能仍未实现，菜单中如实标注"待支持"；首选项的界面分类同理。原"性能与暂存盘"独立对话框（`StoragePreferences.qml`）暂未删除——它仍作为无入口的兼容组件存在，其能力已完整进入首选项"性能"分类，下一步可安全移除。
