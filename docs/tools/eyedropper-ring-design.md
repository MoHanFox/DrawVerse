# 吸色取色环（2026-10-12）

独立 I 吸色和 Alt/Option 临时吸色在按下拖动时显示 Photoshop 风格取色环：上半环为当前采样色，下半环固定为按下前前景色，外圈中性灰，中心透明保留精确取色十字。悬停仅显示十字，松开/失焦/失去捕获隐藏取色环；拖动每个有效点实时更新前景色，越出文档不改变颜色并隐藏环，返回文档继续采样。默认开启，吸色参数栏提供“显示取色环”开关，说明延迟 1 秒，开关仅影响显示。

复用 CanvasItem::pickColorAt → PaintCoreClient::sampleDocumentPixel → 现有 C ABI 显示帧取样，不在 QML 读取文档像素，不增加 Rust/ABI。CanvasItem 维护本次取色前颜色及实时有效采样；鼠标与数位笔共用 begin/update/end 状态。文档更换、client 切换/销毁、禁用、窗口失焦与失去数位笔 proximity 清理状态；空格平移优先，选区 Alt 组合和移动工具不触发取色。

环使用 QML Shape 半圆描边，固定 120 逻辑像素（不随画布缩放），透明中心、上下色区各 18px、外部灰色衬托及薄边，不接收鼠标；CanvasPane 共用，主窗/浮窗与多文档一致。半透明色按真实 RGBA 显示。无整帧复制或额外后台渲染任务。

最小验收：一项原生 Qt 定向回归在 100%/200% 缩放验证上/下颜色、拖动、释放、Alt、数位笔、越界、开关及历史/修订不变；截图检验真实环颜色和空心中心。只运行有关界面回归和 Release 构建，不运行全套核心测试。

交互参考：[Adobe 吸色工具说明](https://helpx.adobe.com/photoshop/using/tool-techniques/eyedropper-tool.html)。
