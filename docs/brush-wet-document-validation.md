# 湿混文档笔触验收

2026-10-15，Windows/MSVC/Rust及Qt6.8.3 Release。Rust实际Document已接二维载色，新增专用起笔/结束方法；无新ABI/BrushSettings/state/VBR字段，Qt没有湿混入口，不宣称整个湿混系统完成。只运行相关回归，未运行完整tools/check.py。

- brush_wet_document：12项全部通过，日志artifacts/brush-wet-document-core.log。真实红蓝二维搬运、调用方初始载体保持、成功返回载体、单历史及像素undo/redo/取消；分页与输入拆分/图层偏移；目标/可见合成取样只写目标且能读前次印章结果；0opacity/0flow仅拾色；选区外拾色、羽化覆盖与压力opacity上限各作用一次；蒙版可见灰度与合成取样明确拒绝；透明锁逐像素精确alpha；纯载色条件27工具模式与普通绘画数值核对；图片/局部及纸纹/散布/灰度双路径16组合分页像素一致；时间平滑同时间线及历史；低压亚像素/画布外/错误橡皮与段预算原子回滚；结束入口不混淆。
- 内部3项wet回归全部通过，日志artifacts/brush-wet-document-failure.log：暂存写失败恢复已绘印章；历史写失败恢复像素、保留旧历史和可撤销的旧条目；4ms平滑生成的多次静止压感印章合计预算，超限整笔恢复。原始工具载体始终未采用在途结果。
- 相关旧画笔混合6项、旧双色9项、涂抹3项及二维载色7项均通过，日志artifacts/brush-wet-document-regression.log；本轮去重共40项相关Rust用例。非湿混仍走原语义。
- paint-core/paint-ffi全目标clippy -D warnings、cbindgen头--check、Release构建、静态/共享C/C++4项ABI smoke和现有Qt真实描边/捕获/VBR/历史用例均通过。日志brush-wet-document-{clippy,header,build,abi}.log及brush-wet-document-qt.txt。

载色准备先完整读取，再逐瓦片提交像素，最后提交本印章载色；原图/贡献账本依旧分页。零权重格点不读颜色，整数位置不读取权重为零的双线性邻点，图像覆盖计划复用。当前印章直径允许正的亚像素值，初始载色网格标称仍1..512。载色是工具状态，文档undo/redo恢复像素而不重新计算或撤销工具载色。

仍待完成：颜色动态/双色湿混组合暂明确拒绝，独立IO/准备桥/ABI/Qt及自动清洗/重载调度未接；完整ABR、收尖、其他扩展和实体设备/跨平台/性能验收不在本轮通过范围。不以合成回归耗时宣称交互延迟达标。
