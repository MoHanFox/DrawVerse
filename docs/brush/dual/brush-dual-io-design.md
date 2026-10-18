# 双笔刷状态与 VBR 交换

state12 / VBR1.10 / behavior9 / dual-cover。dualBrush 与 texture、tipResource 同属预设级对象，第二笔尖资源不混入主 parameters。必需字段：model=paired-dab-v1/path-coverage-v1、enabled、mode（multiply/darken/lighten/subtract/difference/screen）、diameter、spacing（配对必须null，路径必须0.01..1）、hardness/roundness/angle/flow、pressureSize/pressureFlow、resource（hash或null）。可选 pressureDynamics（仅size/flow）、tipDynamics、scatterDynamics、shapeJitter，沿已有严格DTO保存完整曲线/seed和关闭值。

数值先f64验证再进入核心；模型、未知字段、null对象、缺失必需null字段、错误类型和第二不透明度对象拒绝。程序第二笔尖resource=null可启用；图片引用严格绑定原PNG身份，匿名资源明确拒绝保存。默认DualBrush可省略不升级旧格式。配对与路径模型不能互换间距语义。工具颜色、执行中的覆盖场/计数器不写入预设。

状态仍受2MiB总JSON上限约束；编码也检查此上限，不能返回解码器无法重开的“大目录”。预设数量上限与JSON/资源字节上限同时生效，不把1024条目上限解释为任意复杂配置均能超过字节预算保存。

主笔尖、纹理、第二笔尖和原始来源基线PNG取并集，同源共享覆盖Arc并只保存一次完整PNG，预算仍包含周期mip。第二笔尖不改写源ABR基线或原始字节。1.10清单上限6144资源，ZIP总上限7169（manifest1+presets1024+PNG4096+records1024+originals1024）；旧minor保持各自资源上限。Schema公开模型和版本，读取拒绝降版新对象。

本轮ABI保持1.42、Qt仍state11；旧库package读写暂拒绝dualBrush对象，避免旧前端丢数据。读required=0并不写caller/持久资源，写不改目标。下一阶段升级原生资源交接与Qt后解除。旧程序子集接口继续其原范围。

验收：配对/路径及全部模式/关闭配置/完整第二动态、最低间距；state12/VBR1.10恢复两条真实笔触逐像素一致；主/纹理/第二同源与独占资源预算；错误模型/范围/资源/旧版本/能力拒绝、无升级旧对象；来源并集与6144/7169边界；原生旧前端拒绝原子性。完整双笔刷和笔刷系统不能仅以IO阶段完成声明。
