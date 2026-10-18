# 双色双笔刷整笔合成内核验收

2026-10-15，无ABI/state/VBR变化。DualColor配置及separated-stroke-v1真实路径已进入Rust，主颜色矩/第二覆盖分别分页，溢出/前后顺序/背景色及两贡献混合模式执行；整笔opacity只在复合后作用一次。六种旧灰度模式不变。旧IO对任何非默认色彩配置Unsupported，包括双笔刷或色彩模块关闭时的保留值。本阶段不公布用户入口，不能把核心验证当作Qt功能完成。

- brush_dual_color 9项通过，真实像素：配对/独立第二路径、反复回笔内部主色及外侧第二色、前后/溢出与背景来源、透明主色和软边min裁切不平方、27贡献模式与27工具模式在有色半透明底上的精确结果、选区/蒙版/透明锁精确alpha、正负局部图层坐标、页池驻留1瓦片与常驻结果/拆分输入一致、取消和撤销重做、独立静止不透明度压感更新两场、相对尺寸两压力规则和起笔捕获、图片孔洞/空第二图片、纸纹与颜色动态组合、第二巨大散布预算预检与逻辑瓦片失败回滚、未定义橡皮/涂抹及设备橡皮拒绝。
- color_storage_failure_rolls_back_the_whole_active_stroke通过：真实页写失败后无活动笔触、文档瓦片及历史回滚，再次正常绘画成功。
- brush_dual_color_boundary通过：双笔刷/颜色模块开关四种组合，旧DTO/状态编码明确拒绝而非丢字段。
- 相关旧brush_dual 9、brush_dual_path 7、brush_dual_size 4项通过，含旧图像/方向/动态/蒙版/压力/分页/历史行为。新绘画和存储测试使用Release执行；IO边界使用Debug。
- paint-core/paint-io all-targets Clippy -D warnings通过。日志brush-dual-color-core-tests/new/storage/io/clippy.log。没有运行全仓tools/check.py或Qt交互测试。
- Release Qt应用/Rust FFI及生成头一致性构建通过（brush-dual-color-core-build.log）。工作预算按主/第二重算范围同时计入内部两模式和工具混合成本。应用未运行时更新开发目录exe/DLL，核对DLL与验证构建SHA256一致。

当前合成按受影响瓦片重算，尚未进行实体设备延迟或大图性能验收，不声称达到最终绘画性能。后续保存交换版本、原生/Qt双色入口与真实预览、非分隔逐印章模式、自动描边预设及复合光标继续推进。橡皮和涂抹双色语义未定义，明确拒绝启用；不阻止旧灰度双笔刷工具复用。收尖、湿混及高级ABR仍未完成。
