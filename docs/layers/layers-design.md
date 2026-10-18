# PS 风格图层控制：设计

本模块优先完成用户新增的图层面板要求，组/蒙版沿后续模块推进。参考图中的棋盘格对应透明像素锁，不是图层蒙版。UI 只保留透明像素、位置、完全锁三种锁；不透明度与填充分开。用户已澄清“图层样式”仅指混合模式下拉框，本轮不实现 fx。混合模式使用公开的线性颜色合成公式，不承诺 Photoshop 私有算法逐位兼容。依据：[Adobe 锁定](https://helpx.adobe.com/photoshop/desktop/create-manage-layers/transform-manipulate-layers/lock-layers.html)、[不透明度与填充](https://helpx.adobe.com/photoshop/desktop/create-manage-layers/apply-layer-effects/set-layer-opacity-and-blending-modes.html)。

比较直接扩充现有 LayerProperties 与新增配置：现有 C DTO、旧函数和调用者结构体均保留。新增 Rust LayerAppearance 保存 fill、blend、锁、整数位置、溶解种子；新 C ABI 1.4 DTO / submit / publication query，使用明确定长字段。图层属性、混合模式与移动由文档事务写入历史，不从 QML 修改内部结构。

锁透明：逐位保留原 Alpha，不向透明区域绘画，橡皮擦不削减 Alpha；位置锁只限制内容移动，不限制绘画；完全锁限制内容、移动、删除及外观，仍可选择/显隐/解锁，撤销不受锁限制。位置采用非破坏性整数偏移，画布外内容保留；必要时用有界的带符号局部瓦片坐标，避免移动后重新画笔丢失负坐标。历史保存完整位置和混合配置。

合成使用既有线性预乘 sRGB 工作流，fill 独立缩放内容，opacity 再缩放整个层；目前没有图层效果，本模块二者相乘；Photoshop 特殊八种混合的 Fill 曲线未实现。混合使用标准透明源/目标的颜色混合公式。保留普通层现有缓存快路径；位置/混合配置的依赖瓦片与元数据进入缓存判等，不把每帧整张图重算或把图层缩略图读到 UI 线程。

QML 面板顶部放混合模式、不透明度、三种锁和填充，下方紧凑行提供显隐、预览、名称和锁状态；混合下拉框选择提交一个操作。百分比键入或弹出滑条一次提交，避免拖动产生数十条历史。位置以移动工具/接口实现；底部新建、删除。未支持的滤镜/文本/智能对象类别不显示伪功能。

保存：OpenRaster 标准元数据之外需要有版本的 DrawVerse 扩展保存锁、fill、混合配置和位置，不丢失配置；合成导出保留最终效果。外部 OpenRaster 读者只保证基本可互操作，不宣称其他软件完整理解 DrawVerse 扩展。未知扩展版本或无效数据必须明确拒绝。栅格格式保存仍只是合成导出。

验收：透明锁半透明边缘/跨瓦片/橡皮擦、位置锁和移出画布再移回、移动后绘画、完全锁和可解除、百分比输入、独立 fill/opacity、混合公式数值/跨瓦片边界、低驻留分页与编码历史恢复、缓存失效、ORA 配置回环、旧 ABI 布局与新接口、Qt/QML 浮窗面板交互及 DPI、完整 check、原生性能对比。Python 插件最后。


实现补充：列表使用 ListView 虚拟化，可见委托请求原始图层缩略图，不再遍历/创建全部 QML 行。复用后台视口槽 2（新 API 限制槽 2/3、96px）；Qt 每次最多一个 64px 请求，安静 200ms 后提交，并缓存最多 128 张非可见预览加当前可见预览。开始绘画/新修订取消过时作业，预览 PNG 编码在 Qt backend worker。文档 publication 不持有预览快照，不在每个输入点克隆所有瓦片。移动在松手时提交，拖动时暂不实时预览；键盘每次 1px / Shift 10px。DVH2 是临时历史内部编码，不作为持久工程格式。

OpenRaster 依 [标准层栈规范](https://www.openraster.org/baseline/layer-stack-spec.html) 读写定义的 15 种 Normal/颜色混合 SVG 名称，不支持其他 Porter-Duff 操作。新增 urn:drawverse:layers:1 / dv:version=1 扩展保存 fill、blend、locks、offset/local 坐标、原 opacity 与 seed；未知版本、冲突标准/扩展混合、非法 flags/位置必须拒绝。栅格层仍量化为 8-bit sRGB。特殊模式的标准 composite-op 回退 src-over，外部应用可使用正确 mergedimage 预览，但忽略扩展后逐层结果可能不同。
