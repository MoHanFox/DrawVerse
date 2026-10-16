# 平滑收尾、缩放快捷键与色轮保存验收

2026-10-16，按用户确认保留拖绳距离，抬笔停在当前笔画末端、不补线。对应[设计](brush-follow-color-zoom-design.md)。无C ABI、state或VBR新版本；已提交画布历史不重算。

原收尾在结束笔触时合成最多512个追赶点并连接原始终点，高参数使这段明显成为直线。现已移除此处理，使用最后实际过滤点完成正常末印章；不额外追赶压力或释放稳定距离。绘画中的位置/压力平滑和高参数延迟继续保持。稳定距离提示同步说明“不补线”。

Ctrl++、Ctrl+=及Ctrl+-在所属CanvasItem原生窗口路由，支持Shift与小键盘，只缩放当前可见画布。文档/像素/历史不变，文本输入和绘画期间不消费。颜色在共享BrushLibrary偏好快照内保存colorState前景/背景，旧字段缺省兼容；最近色统一ARGB表示并保持顺序。所有窗口通过既有单写入者同步；预设重置、原生目录和VBR不保存或回退全局颜色。正常关闭等待偏好保存队列。

最小相关验证：

- Rust平滑8项通过（artifacts/brush-follow-core.log）：新增高参数折线、Paint/Erase/Smudge结束前后真实像素不增长、远端无补线、单历史undo/redo；保留时间拆分一致、独立通道、范围/时间/预算拒绝、禁用旧像素、取消、蒙版/选区/偏移及分页。命令按brush_smoothing名称筛选，其他集成测试未执行。
- IO平滑边界/交换4项通过（artifacts/brush-follow-io.log）：完整关闭参数、非法DTO、state/VBR真实两笔及降级规则。
- paint-core all-targets Clippy -D warnings通过（artifacts/brush-follow-clippy.log）。
- Release及生成头字节检查通过（artifacts/brush-follow-release-final.log），C/C++ static/shared四项通过（artifacts/brush-follow-abi.log）。
- Windows11 / Qt6.8.3 / windows平台、software Quick、环境倍率1，4个功能用例含初始化清理6 passed、0 failed（artifacts/brush-follow-final-qt.txt）。真实鼠标/数位笔进入画布并抬笔，远端无连接；前景/背景交换和alpha、12色顺序正常关闭后重建客户端恢复，非法colorState原子拒绝、旧字段缺省及VBR/目录不含全局颜色；主窗口Ctrl+=/Shift加号/减号、浮窗小键盘及文本输入保护、中心/历史/modified/revision保持，多文档颜色和历史同步；既有平滑控件/真实预览/捕获/交换/重开回归。
- 色轮单/双击与CapsLock光标、颜色动态两色捕获及真实像素交换2个既有功能用例通过，含初始化清理4 passed（artifacts/brush-follow-color-regression-qt.txt）。

首轮测试揭示默认最近色与恢复后字符串表示不一致、QML快捷键别名路径不满足加号往返；已统一默认ARGB并改为一次原生窗口输入消费，最终结果如上。未运行无关全量或其他平台验收。合成数位笔事件不等于实体驱动认证；完整笔刷系统其他未完成范围继续后续。
