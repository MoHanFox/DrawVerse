# 主线 C3 — Move Start & Tracking Responsiveness

基线为合并 C2-B 后的 `9b36e2f80f414ddee062eb87fb6ed429359cae17`。
本轮首先区分 GUI 发生器延后、输入已到达后的命令链、worker/GUI 发布和
Canvas 节点采用。只有证实一个可局部修复的产品等待才实施优化。

## 按需探针与验收

复用 `qt_layer_move_baseline` 的 S2 满幅 Normal 栅格夹具、原生 1024×720
窗口、fit 缩放和实际 D3D11。新增独立按需目标，不改变 C1/C2-A 的输出契约。
隔离配置，具体 QMouseEvent 对象登记后经 QQuickWindow 正常路由；不移动 OS
光标，不屏蔽非计划输入或失活。使用已有 PendingMoveEvidence 的稳定上下文、
串行 grab 和实际 render identity 协议，像素读回在计时结束后。

起拖探针连续发送 Press 与 (384,0) Move，再归还事件循环；不得预先等 BEGIN。
持续探针使用 GUI 8ms QTimer，有限暂停后转向再返回最终 (384,0)。每点保留
计划/实际 dispatch、绝对 delta；不补发错过的定时输入。两种探针不混合统计。
I→撤销→W 与新进程直接 W 分开；W 以资产存在且上传批次已记录分类，
不声称 GPU fence 完成。实际未捕获 I 则明确标注。

沿用单调时钟、有界默认关闭 InteractionTrace 和 Rust move_trace，补 worker
元数据读取/发出、GUI 入口与 resident 的 generation。节点采用保留同步阶段的
不可变 identity 和时刻，普通应用无 observer。跨语言仅用同一 sequence 的同步
FFI 包含区间校准，不能相减未对时的时间戳。渲染信号只记录调用线程时间，
不读取 GUI 对象，不把 afterRendering/frameSwapped 称为物理显示完成。

必须保留取消/错误/超时/非计划输入，不用成功替换失败。验证 early Move、
绝对更新、单历史、撤销/重做、取消和完整帧接管。拟修复项需旧条件失败/
延迟、等待链和相同工具的前后证据；否则只交付归因，不猜测性修改策略。
不改变 16ms 合并、worker 频率、sequence 屏障、缓存资格、采样和画质。

原始证据使用独占创建的 `artifacts/issue-2-main-c3/` 子目录。具体结果、版本、
有效场景及未测边界见对应验收报告；实体设备和物理显示延迟不在本轮证明范围。

## Shader artifact / precompile 对照

先核实实际链接资源而非仅检查 .qsb 文件名。Qt 6.8.3 的 PRECOMPILE 在 Windows
调用 SDK fxc，将 HLSL 5.0 替换为 DXBC；保留其他目标和 Standard 变体。
只处理 liquify.vert / liquify.frag（移动、液化和常驻变换共用），其他 shader
继续原构建参数。Windows 默认启用；需要 Developer Shell 中的 fxc，缺失时报错，
可显式关闭选项保持运行时编译路径。非 Windows 保持原行为。构建后验证 DXBC，
防止 qsb 调用外部编译器失败却留下 HLSL 的软失败被误报为成功。

固定共用工具后比较独立产物。shader-pair 在同一窗口执行直接准备后的 W1
即时非零输入，再撤销并准备 W2 的暂停/转向轨迹；资源 source 可以改变，不能
据此宣称纹理相同。记录同一窗口/QRhi、实际资源摘要、缓存配置、准备与收尾。
Main.qml 模式加载正式资源和默认管线缓存策略，使用新建隔离数据及对称缓存起点；
它是实际 QML 路径，不等同于完整 main.cpp 启动性能。保存计时外同一次稳定 grab
与最终完整图像以比较整个输出。保留所有无效输入，有限对照不以重跑替换失败。
