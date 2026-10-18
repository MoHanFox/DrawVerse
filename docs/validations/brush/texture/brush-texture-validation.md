# 纹理内核验收

2026-10-14。设计见 [纹理覆盖](../../../brush/texture/brush-texture-design.md)。Document固定纸纹和Tip局部纹理、不可变覆盖资源/周期mip/编译计划、TEXTURED专用路径和共用写入已实施。ABI仍1.41，尚无IO/原生Qt纹理入口；非默认纹理保存明确拒绝。

实际检查：

- `paint-core --test brush_texture`：7项通过。真实周期孔洞/分段一致与精确历史、密集1%间距下固定灰纸纹在40%/100%不透明度不被填平、Tip移动和跟随笔尖角度、纹理角度/深度/反相、关闭/零深度/恒等纹理旧像素保持；图层偏移仍对齐文档、图片/选区/擦除/蒙版/手指涂抹共用、颜色贡献与纸纹上限组合；资源在途捕获、共享Mask和周期mip分别去重、失败目录修订不变、段预算拒绝不改像素及取消恢复。
- `paint-core --lib brush_texture`：2项通过。周期缝/负坐标、3像素奇数边缘缩小平均、浮点余数可能等于周期的边界处理、缺资源和非法参数拒绝。周期mip按面积加权逐级8位量化，旧image clamp/mip未修改。
- 核心相关回归通过：`brush_mask` 5、`brush_opacity` 7、`brush_color_dynamics` 9、`smudge` 3、`brush_program` 3、`brush_catalog` 5项。
- IO `brush_texture_io` 1项通过：启用和关闭资源不能被旧state丢弃，目录仍持有资源；`brush_color_exchange` 5项通过。
- 三包all-targets clippy（`-D warnings`）及相关Rust格式化通过；编译覆盖原生层。旧C参数编辑保留新纹理字段，直接旧请求使用默认无纹理；没有C DTO或生成头布局变更。

未跑完整 `tools/check.py`，未做Qt构建、实体设备/跨平台/专项性能基准。周期取样按最大8访问计入段预算，不通过改变间距降低质量；没有新增整图纹理缓存。纸纹对绘画/擦除/蒙版是固定整笔覆盖限制，涂抹仍按每印章搬运强度作用。真实资源IO/VBR与原生Qt下一阶段接入，其他纹理组合/ABR映射、双笔刷/平滑/湿混及完整格式冻结继续后续；不宣称PS全部纹理或完整系统完成。
