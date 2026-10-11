# 遗留功能修复验收（2026-10-11）

范围和行为见 [设计](deepseek-recovery-design.md)。用户确认先交付修复，并追加工具栏吸色；GPU、液化、.vbr、图层复制/合并未在本轮实现。ABI 1.12、布局 v6 与 20ms 逐条历史回放保持。

## 失败基线与根因

- 完整统一入口首先失败于 FFI 的超大 capacity 检查：删除 isize 限制后，64 位平台接受了 u64::MAX。恢复边界并验证失败不写像素。
- 下一轮暴露 ORA v2 兼容测试的错误构造：夹具只删旧选择文本 `1|0`，真实空选择已使用 v2 编码。按实际序列化值移除 v2 不支持的属性；保留旧格式加载断言。
- B/E/V 快捷键只改单个布尔标志，造成旧选框/移动状态残留；选框图标又只读取活动状态。所有入口恢复互斥，形状记忆独立保存。
- 缩略图完成时强制 `m_publication=0`，重新遍历全部 ABI 元数据并重建 QML ListView；重建的行再次请求预览，造成循环开销。独立 thumbnailReady / layerThumbnailsChanged 通知保留行对象，后台 PNG 编码和 48px 输出不变。
- 剪贴修复只识别孤立链，未识别无声更换为另一基底；按重排前后基底 ID 比较释放受影响链。新层插入现有链时继承剪贴，结构变化和修复仍是一条可撤销历史。
- 关闭停靠组后纵向比例重铺剩余空间；保存剩余组实时高度，剩余部分显示背板，工具列仍占满整列。可选 retainedHeights 随布局 v6 保存，拖动取消也还原。
- Alt 取色原来仅切系统光标，不渲染跟随数位笔的十字；共享 CanvasPane 现在绘制黑白十字，独立工具栏吸色也使用同一输入和显示路径。
- 两倍 DPI 用例暴露吸色坐标直接把文档坐标当输出像素：按实际视口区域和输出宽高换算，覆盖缩放、LOD、分数原点与 i64 极值；输出仍是当前合成显示帧。
- 最大化恢复后鼠标抓取丢失，第二次移动不再跟随；恢复拖动由全局 WindowDrag 接管，释放/取消清理按键状态，已释放的延迟拖动立即结束。
- 折叠栏测试在 showNormal 后尚未完成布局/输入更新就按旧命中树发送事件；等待恢复后的界面派发，再断言实际按钮 pressed 与真实浮窗拖动。

## 验证证据

首轮完整 tools/check.py 全通过：C11/C++20 静态/共享 ABI 四项、普通 Qt 与两倍 DPI Qt 共 6 个 CTest（74.13s）；Qt 普通 76 项、DPI 46 项均含初始化/清理。随后补齐全局拖动释放状态回归，再跑完整入口作为最终验收，结果见下方最终记录。

新增/扩展回归覆盖：

- Rust 剪贴链插层、另一基底仍存在时移走原基底、剪贴层换基底、跨父级与撤销/重做。
- FFI 指针长度、分数/缩放视口采样与极端坐标；生成头严格字节检查不变。
- Qt 真实 B/E/V/I 和工具栏输入、椭圆槽位记忆、独立/Alt 吸色不产生历史、黑白十字真实像素、鼠标/数位板悬停、数位板 Space 平移。
- Qt 普通/两倍 DPI 缩略图通知不触发结构通知、不销毁原行、8 层缓存和面板重开（小于 1s 阈值）、新文档清空缓存。
- Qt 普通/两倍 DPI 关闭组后剩余高度、工具列全高、底部背板真实像素与布局恢复。
- Qt 自动释放剪贴历史显示真实名称/图标，并在释放后继续更新元数据、新建层与撤销/重做。

日志位于 artifacts/deepseek-recovery-check.log、artifacts/deepseek-recovery-final-check.log 和最终原生专项日志。Qt 6.8.3 / Windows 11 / MSVC；数位板覆盖为 Qt 事件接入回归，真实设备体验由用户复测。

## 最终记录

- `python tools/check.py`：通过。Rust fmt/clippy、全部 Rust 测试、cbindgen 严格字节检查、Python 工具真实 Git 检出测试、C11/C++20 静态/共享 ABI 与两个 Qt CTest 均完成；6/6 CTest 通过（74.25s）。普通 Qt 76/76（46.295s）、两倍 DPI 46/46（26.649s），含初始化/清理，无失败或跳过。
- Windows 原生专项：9 个测试槽加初始化/清理共 11/11，通过（9.650s）。覆盖真实最大化/还原/鼠标抓取、按住折叠栏拖动、原生菜单层序、吸色鼠标/数位板路径、面板高度/缩略图和跨原生窗口停靠。跨窗拖动产生两条目标窗口外鼠标坐标的 Qt Test 提示，断言和 QML 警告检查均通过。
- 最终日志：`artifacts/deepseek-recovery-final-check.log`、`artifacts/deepseek-recovery-native.txt`；未把构建产物或工具链加入版本控制。

- `cmake --build build/qt --config Release --target deploy_qt`：通过，应用和 Qt 测试先重编、再部署。当前可执行文件为 `build/qt/bin/Release/drawverse.exe`；构建日志 `artifacts/deepseek-recovery-final-deploy.log`。
- 原生界面截图专项 `referenceWorkspaceDockingAndColorPicker`：3/3 含初始化/清理通过（3.756s）；已检查主窗和浮动面板截图，吸色图标显示完整、顶部/正文和折叠列均正常。截图位于 `artifacts/deepseek-recovery-ui.*.png`。此用例另有一条对不接焦点 flyout 调用 requestActivate 的 Qt 平台提示，无 QML 警告。

- 正式 build/qt 全量构建再次通过，并同步最新 paint_ffi.dll，避免交付目录残留旧共享库；日志 `artifacts/deepseek-recovery-delivery-build.log`。部署后的 drawverse.exe 使用临时偏好运行 `--preview`，真实 ABI 画三条压感笔触、渲染截图并异步关闭，退出码 0；截图 `artifacts/deepseek-recovery-delivery.png`。
