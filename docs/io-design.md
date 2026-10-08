# 基础文件 IO：设计与接口（代码前）

后续扩展：当前已支持 27 种模式、dv 层配置/画布外内容，以及最多 16 级嵌套隔离组（ORA 总节点仍限 256），以 layers-design.md 与 groups-design.md 为准。下文保留基础文件模块首轮范围；穿透组、PSD/ICC/蒙版仍未实施。

2026-10-08 文档分页更新：栅格/图层总像素上限调整为 128M，解码器分配预算为 1GiB，导入逻辑瓦片不再限 4096；导出按瓦片 pin 批量读取，错误保持原子保存。其他格式安全限制继续有效，当前范围与实测见 [分页验收](document-paging-validation.md)。下文保留文件模块首轮设计。

本轮先移除笔记面板，并将旧工作区迁移到 v2，只清理 notes 类型及空组，保留其余组归属/标签/浮窗几何。增加 Qt 迁移、持久化与重新打开回归，不能只删除演示名字。

## 格式方案

| 方案 | 兼容性与代价 | 选择 |
|---|---|---|
| Qt 图像插件 | 现有依赖少，但内核耦合 Qt，ORA 仍需独立实现 | 不采用 |
| 原生 C 编解码库 | 成熟但多平台编译、动态依赖与 unsafe 边界更大 | 后续特殊格式再评估 |
| image + ZIP/XML，纯 Rust 编解码 | PNG/JPEG/WebP、跨平台一致、锁定依赖，需限制解码资源 | 本轮采用 |

paint-io 依赖 paint-core/render/task；锁定 image 0.25.8（只 PNG/JPEG/WebP）、zip 2.4.2（只 deflate）、quick-xml 0.37.5、tempfile 3.27.0。codec 和文件系统不接触 Qt。资源策略：输入文件 ≤256 MiB，单幅合成图和 ORA 图层栅格累计分别 ≤16,777,216 像素、边长 ≤16,384，ORA ≤256 图层/1024 ZIP 项、XML ≤1 MiB、累计解压 ≤512 MiB；文档仍受 4096 瓦片预算。百万空白文档的全尺寸栅格导出会明确拒绝，不能隐式降采样。

PNG 保留 alpha，JPEG 明确在指定不透明背景上合成并控制质量，WebP 第一版无损编码。工作像素为线性预乘32F；文件像素为 sRGB 直通道，先反预乘再编码；JPEG 背景合成在线性空间。PNG 导入支持8/16位归一化；JPEG/WebP EXIF方向应用后再建立文档。ICC 尚未有转换引擎，含 ICC 或非 sRGB PNG gamma/chromaticities 的文件明确拒绝，不能假装已完成色彩管理。

OpenRaster：mimetype 首项且 STORED、stack.xml UTF-8、图层 PNG、mergedimage.png、≤256缩略图；XML 顺序从顶到下，内核从底到顶。支持正常混合、可见性、透明度、层名、层偏移及活动层；不支持的组/混合/外部引用/DTD/实体/滤镜明确报错，不悄悄扁平化。超出画布的非透明像素、非默认 DPI 等当前模型无法保留的数据拒绝。输出层PNG裁剪到内容范围；空图层写1×1透明PNG。ORA图层/合成图8位量化，保留可编辑图层但不是32F无损原生格式，历史不写入。

ZIP 不解压到磁盘；规范化检查每项相对路径，拒绝绝对路径、..、反斜杠、重复名称、加密/不支持压缩、大文件与解压超限。XML严格平面结构、有限属性、禁DOCTYPE/通用实体。仅接受当前预算下所需的单磁盘 ZIP32；先验证并构建独立新文档，失败不替换当前文档。

## 后台任务与 C ABI 1.2

旧DTO/函数布局保持；新增 PaintFileRequest / PaintFileJobInfo 和 submit/info/cancel/message 四个函数、文件 IO 能力位。每session一个文件作业，状态为Queued/Running/Succeeded/Failed/Cancelled，结果一直保留到下一作业。提交复制UTF-8路径（≤32KiB，禁止NUL），分配job_id并加入文档FIFO。耗时解码/编码使用独立后台作业，文档actor不等待codec；取消令牌在读取、瓦片转换、输出行与最终替换前检查，codec单次调用不可强杀。

Open捕获原文档generation/revision，完成时仍一致才替换，避免加载期间的新修改被覆盖。Save捕获精确快照，编辑可继续；只有同代际同revision成功保存ORA才清除modified。PNG/JPEG/WebP导出不清除modified。新建/打开替换代际后，旧保存完成不得清除新文档状态。销毁取消文件任务并后台join。

原子保存：同目录NamedTempFile、写完flush/sync_all、检查取消、persist替换；失败/取消前原文件不变，临时文件自动释放。替换为提交点，已提交的作业不能以取消伪称撤回。原子替换不等于所有平台断电耐久性。

## UI 与验收

PaintCoreClient/QML 增加打开、保存OpenRaster、另存、PNG/JPEG/WebP导出、取消、错误/完成提示与文档路径。Qt文件对话框只提供路径，不编码像素；主线程不等读写。未保存打开/新建/关闭统一确认，保存后再继续，失败不丢旧文档。扁平导出不变成可编辑保存，ORA量化/ICC限制写入说明。

Rust：导入构建不带历史/预算、RGBA/16位/透明/方向、四格式回环、ORA层顺序/偏移/属性/UTF-8、畸形/截断/路径/实体/压缩炸弹、取消与原子覆盖、异步代际/修改保存语义。Qt Test：打开/保存/导出/失败保留/可取消/异步关闭、旧笔记清理、现有停靠/数位板/视口回归。统一 tools/check.py 全部执行；不为尚未存在的插件添加假pytest。

规范与依赖依据：[OpenRaster 文件布局](https://www.openraster.org/baseline/file-layout-spec.html)、[图层结构](https://www.openraster.org/baseline/layer-stack-spec.html)、[image 0.25.8](https://docs.rs/image/0.25.8/image/)、[ZIP 2.4.2](https://docs.rs/zip/2.4.2/zip/)、[tempfile persist](https://docs.rs/tempfile/3.27.0/tempfile/struct.NamedTempFile.html#method.persist)。
