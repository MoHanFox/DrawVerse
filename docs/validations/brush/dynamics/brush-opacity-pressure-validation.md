# 独立不透明度压感验收

2026-10-14，ABI1.36 / 状态v6 / VBR1.4 / behavior3 / opacity-pressure。普通绘画、擦除与蒙版使用独立局部压力上限；流量仍控制达到上限的速度。尺寸/流量的旧开关、曲线与默认行为保留。关闭不透明度压感保留编辑曲线与最低比例，全部重置为关闭/默认时可规范化回旧格式。

执行与保存证据：

- paint-core `brush_opacity` 7 项与 `brush_pressure` 5 项通过：同压力往返不突破上限、升压加深/降压不变浅（含原地只改变压力）、流量独立、最低比例、禁用、图片孔洞与选区、蒙版/擦除及取消/撤销重做。沿既有分页覆盖瓦片；没有全画布缓存。
- paint-io `brush_pressure` 4 项通过：v6与VBR1.4往返、有界控制验证、v5与包minor3拒绝新对象、数字enabled/null拒绝、关闭时非默认曲线保留、原v5/VBR1.3仍产生相同压力像素。来源基线仍不执行或伪报未支持参数。
- paint-ffi `pressure_packages_round_trip_through_native_entry` 通过：现有原生package接口读写实际opacity-pressure包并保留开关。
- Qt 4 项通过，记录在本地 `artifacts/brush-opacity-pressure-ui-tests.txt`：真实点击传递分类的不透明度压感开关、保存恢复、VBR往返；原生笔触在50%压力/20%最低比例/40%不透明度得到约24% alpha，并在起笔后关闭开关仍保留捕获配置。旧程序笔尖、独立预设尺寸、旧VBR接口回归通过；无QML警告。
- 核心/IO/FFI all-target clippy `-D warnings`、Release Qt构建/cbindgen检查、C/C++静态/共享4项ABI smoke通过。

公开Schema：`core/crates/paint-io/tests/fixtures/schema-history/dynamics/vbr-opacity-v1.4.schema.json`，JSON可解析；未运行第三方JSON Schema验证器（当前Python未安装jsonschema），实际读写器拒绝边界由Rust测试验证。未跑完整tools/check.py；未做数位板实机验收、跨平台验收或与PS真实输出逐像素核对，不宣称完整PS兼容。

完整笔刷系统仍未完成：随机/倾斜/方向/散布/纹理/双笔刷/颜色等动态、平滑与湿混、ABR动态完整映射、包分组扩展及完整格式冻结继续后续。本阶段不是整体完成声明。
