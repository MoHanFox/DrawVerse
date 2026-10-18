# 吸色取色环验收（2026-10-12）

按用户此前要求只做最小定向测试，没有运行 tools/check.py 全量检查，也没有运行不受影响的 Rust 测试。核心与 ABI 未改变（仍为 1.18）。

Qt 仅运行新回归 `eyedropperRingComparesColorsDuringMouseAndTabletDrag` 和既有 `altEyedropperSamplesWithoutHistoryAndBracketKeysResize`，Windows 原生窗口在 100% 和 200% 缩放下各一次通过；各 4/4（含初始化/清理），没有失败、跳过或 QML 警告。

新回归验证默认开启、开关不影响实际取色、说明延迟 1000ms、悬停不显示环、按下时保存旧前景色、鼠标拖动从红色到绿色时上半环更新而下半环固定蓝色、越出文档隐藏且不采样、返回恢复、释放隐藏；Alt 临时吸色及数位笔按下/移动/释放共享行为，窗口失焦清理，历史深度和文档修订不变。抓取真实合成像素验证上半环红色、下半环蓝色、外圈 #808080、中心仍显示画布红色；固定 120 逻辑像素不依赖画布缩放。既有回归继续覆盖吸色读帧、临时 Alt 十字、数位笔悬停/平移、快捷键和浮动工具列。

Release 构建、现有生成头严格检查通过；QtQuick.Shapes 运行依赖已部署。检查截图上下颜色、灰圈、空心中心、参数栏开关与主界面布局。日志 artifacts/ring-build.log、ring-native-1.txt、ring-native-2.txt；截图 artifacts/ring-ui-1.png、ring-ui-2.png。生成文件不提交。

取色仍复用已有显示帧合成取样（缩小视图使用该帧的 LOD），没有改变取样算法。显示开关只在 client 生命周期保留，未新增跨启动偏好；本轮未做真实数位板硬件或浮动画布额外测试，浮动/多文档继续复用 CanvasPane。
