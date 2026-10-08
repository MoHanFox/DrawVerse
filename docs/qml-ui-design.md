# QML UI 接入设计

用户在 2026-10-08 指定使用 QML，优先接入界面，取代先前 QMainWindow/QDockWidget 的首版选择。

## 当前边界与实现方案

- Rust 不依赖 Qt。ABI 1.2 增量扩展保留旧接口；PaintCoreClient 封装句柄与调用，QML 不得到核心指针。
- 已用 Rust 文档 actor 与 CPU Renderer 替换首版 C++ FIFO/整张缓存。主线程只提交小命令/视口，Qt worker 读取已发布元数据和 RGBA8 帧；笔触、历史、图层、合成与显示编码在 Rust 后台执行。见 [任务与 CPU 视口设计](task-render-design.md)。
- BackendWorker 拥有 core/session；关闭先禁用 GUI 桥，再在后台取消、join、释放。正常关闭异步完成后退出 GUI；析构等待是异常退出时的资源保护。
- Rust FIFO 最多 4096 条普通命令；溢出剔除当前笔连续 Move 尾部、预留 Cancel 并回滚整笔，控制请求保持顺序。Qt 接收队列每槽最多一张待领取帧及一次待领取元数据，GUI 忙时不积压整帧。
- 帧带 generation/request/revision；新文档与视口替换隔离旧帧，撤销/取消有 revision 屏障。主画布请求可见区域，导航用独立低优先级槽；每槽最多 4096 边长、4,194,304 像素。显示缓存不修改模型。
- 初始 960×640；新建每边上限 1,000,000，内核仍使用稀疏瓦片与资源预算。显示内存随视口有界；CPU LOD 为全局对齐方块平均，ICC/HDR/GPU 仍待实施。

## 画布

QQuickItem 使用 Qt scene graph 呈现缓存 QImage，仅在 updatePaintNode 更新纹理，不在 GUI 线程合成像素。帧按对应文档区域映射，不能把裁剪视口拉伸到整张文档。软件 scene graph 时保留显示回退。文档坐标与逻辑 DPI 分开；输出按窗口 DPR 请求物理像素，监听 screen/DPI 改变，不因帧尺寸变化重置缩放。

左键绘制，鼠标用 pressure=1；中键/Space 平移，滚轮以指针为中心缩放，F 适配，Esc 取消。数位板事件在画布所在 QQuickWindow 过滤，转发 pressure/tilt/rotation/tangential/buttons/tool/capabilities；接收后 accept，拒绝合成鼠标重复笔触。只读显示帧与 GUI 输入对象不互跨线程。Qt 模拟事件测试与真实硬件验收分开记录。

## 工作区与外观

QML ApplicationWindow，原创石墨灰工作区、青绿色强调、暖白画布、低干扰工具区。只提供真实功能：画笔/橡皮擦、颜色、半径/流量、图层、历史深度、缩放/适配、新建、打开、ORA 保存和三格式导出；未支持的变换/文件格式不放置假可用按钮。

WorkspaceManager 管理注册面板与 groups，面板只能有一个拥有组。主窗左右停靠列使用 SplitView，组内标签切换；native QDrag + 专用 MIME 支持跨 QQuickWindow 拖放。拖标签移动单面板，拖组标题移动整组；落在组上合并标签，落在左右停靠列边缘创建组，未命中停靠目标时创建独立 QML ApplicationWindow。边缘目标不覆盖面板，避免父 DropArea 抢走标签合并。浮动窗可继续组合/并回主窗；关闭浮动窗把面板归回主窗，不删除内容。当前为左右列的垂直分割、标签组和独立组窗口；任意嵌套分割树后续扩展。

浮动窗口显示尺寸/位置按 QScreen.availableGeometry 修正；组布局、活跃标签和窗口几何版本化保存到应用 QSettings，恢复校验 panel-id、唯一拥有关系、数量/字符串/尺寸上限，坏配置回默认。用户可重置布局。自定义面板为声明式色板/笔刷参数类型，允许自定义名称；不执行任意 QML/脚本。Python 面板扩展最后通过数据协议接入。

当前持久化面板组归属、标签、浮窗几何；布局 v2 移除笔记定义及实例，并清理空组；SplitView 分隔比例还未写入布局格式。重置恢复默认内置面板并移除自定义面板实例。所有画布窗口共享同一个 PaintCoreClient。画布通过 scene graph 呈现的依据见 [QQuickItem](https://doc.qt.io/qt-6/qquickitem.html)，接收 tablet 事件防止合成鼠标重复输入遵循 [QTabletEvent](https://doc.qt.io/qt-6/qtabletevent.html)，跨窗口拖放使用 [QDrag](https://doc.qt.io/qt-6/qdrag.html)。

## 验收

Qt Test：客户端异步初始化、绘制/pressure/取消/undo/redo/图层、GUI 事件循环响应、资源关闭；工作区合并/拆分/浮动/唯一拥有/关闭归还/保存恢复/坏配置；坐标逆变换/缩放/鼠标和模拟数位板输入；真实 QML 加载与面板切换无错误。异步文件打开/保存/导出/失败恢复/取消与关闭、旧笔记迁移；实际运行应用生成主窗预览并检查布局。继续跑原 Rust 与 C/C++ ABI 测试；不为尚未实施的 Python 插件创建假 pytest。
