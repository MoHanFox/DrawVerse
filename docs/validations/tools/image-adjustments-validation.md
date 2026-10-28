# 图像调整验收（2026-10-28）

## 当前修订：可拖动模态窗口与常驻 GPU 预览

- 窗口使用独立全局坐标拖动，ApplicationModal，标题栏单独透明且不接磁吸；紧凑布局。关闭按钮默认背景透明，hover 不透明 #e81123，离开恢复，真实鼠标验证。
- 独立 AdjustmentCurve 样式：网格、灰阶带、方形节点、真实源直方图；色阶直方图/三输入手柄/两输出手柄及数值框；HSL 彩色轨道/圆形手柄/数值及实际着色。
- 参考本地 PhotoCraft adjust_preview.rs，Qt/D3D11 在原目标层采样后、蒙版和混合前调整；固定原图和身份，中性 BEGIN 预备资源，持续更新复用纹理/页表/轴/缓冲/SRB，只更新 uniform 或变化的 LUT，生产预览无回读。确认才核心物化候选、单条历史，保持预览到最终主画布采用。
- ABI1.88 新增不可变 publication 原图 4×256 直方图（最多262144真实源样本，alpha/选区加权）；HSL parameters[3] 扩展着色0/1，旧布局保持。

| 本轮验证 | 结果 |
| --- | --- |
| 核心 adjustment | 4项通过，增加原图直方图/选区/着色/历史断言 |
| paint-ops / paint-gpu lib | 10 / 2项通过；GPU 8192个透明及半透明像素六组合对照，误差断言 <2e-5，独立已知颜色与着色断言 |
| Qt 软件 | 6功能行通过，另init/cleanup；真实标题拖动、手柄在释放前更新、关闭按钮hover透明/红色/恢复、模态、取消、单历史及撤销重做 |
| Qt 实际 D3D11 | 21功能行通过，另init/cleanup；原6界面行、12行（三调整×普通/组/剪贴/蒙版混合）、3行实际模态GPU拖动/选区CPU后备/注入设备失败后备 |
| 持续调参资源与像素 | 每个12行连续12个请求，revision/历史不变；零新增源瓦片上传、页表及参数缓冲重建，uniform复用计数增长；捕获预览与最终核心提交后GPU画面每通道差≤1；最终原图撤销/取消精确恢复 |
| 四ABI / 相关严格Clippy / fmt / 生成头 | 通过；新增直方图大小/旧publication拒绝及着色排队 |
| 原通用GPU合成定向回归 | 实际D3D11共95项通过（含init/cleanup）：documentPixels、根混合/固定蒙版、组边界、剪贴依赖、变换复用；56根/28组窗口RGBA32F捕获经Rust独立核心校验，最大误差分别1.1920929e-7 / 5.9604645e-8 |
| Release | 标准build/bin/Release已更新；实际D3D11应用--preview冒烟生成截图，无QML加载错误 |

最终记录：`artifacts/adjustment-v2-final-d3d11.txt`、`adjustment-v2-final-software.txt`、`adjustment-v2-abi.txt`；截图 `artifacts/adjustment-v2/`，构建 `adjustment-v2-resident-build.log`。原合成回归 `adjustment-v2-composition-final-d3d11.txt`，独立校验 `adjustment-v2-composition-reference.txt`，实际应用截图/日志 `adjustment-v2-app.png` / `adjustment-v2-app.txt`。

修订中失败保留：首次EXE被用户运行实例占用，用户保存关闭后链接成功；新增Rust代码初次语法错误（浮点字面量、重复pub）已修正；首轮GPU抓图未持续驱动帧导致超时，按既有observer协议修正；其后一次提交交接在锁内发出同步信号导致重入卡住，停止该测试进程并移出锁后复验通过。两轮完整矩阵原日志 `adjustment-v2-d3d11.txt` / `adjustment-v2-projection-d3d11.txt` 保留：Screen在白底上不可见的夹具改为Multiply，等待真实视口像素密度和CPU帧匹配；中性BEGIN提前准备会话资源后原持续复用断言保持并通过。未删失败断言或把原轮次写成全绿。原合成回归首次调用缺少必须的DRAWVERSE_G2_REPORT_DIR和DRAWVERSE_G4_FIXTURES，42通过/53失败，原日志 `adjustment-v2-composition-d3d11.txt` 保留；补齐现有夹具与新捕获目录后同组95项通过。

支持边界：有选区、CPU统一设置、软件/其他API、原通用GPU预算或能力不支持走原核心候选预览；设备失败亦恢复CPU。仍全分辨率后LOD，无PhotoCraft大图低分辨率代理或可见区域优化；无端到端性能、实体笔、跨物理屏幕或总check结论。设备故障为测试注入，非真实TDR。

## 首版记录（下述范围由上述修订更新，保留原结果）

设计见[曲线、色阶和HSL](../../tools/image-adjustments-design.md)。本轮为实际功能接入，不代表全仓库验证或大图性能门槛通过。

## 实现与确认范围

- 曲线 RGB/红/绿/蓝、色阶五参数、HSL 三参数，菜单实际启用。
- ApplicationModal 无边框独立窗口，PanelWindow/Theme 浮窗样式；无 Workspace 磁吸/停靠接线；原 SpeedCurveEditor 复用。
- 每次预览从原始瓦片重算，选区/羽化/偏移正确；取消不修改历史，确认一条具名历史，撤销/重做恢复实际像素。
- 有界CPU/wgpu算子，统一 CPU/自动设备设置；CPU/GPU共同颜色语义。GPU每批回读候选，未实现纯GPU常驻调参预览，未测交互性能。
- ABI1.87 / 新历史42–44 / 新会话flag64；原DTO、文档及笔刷state保持。组/显式蒙版/置入图/全锁目标拒绝，菜单禁用这些目标；只处理活动栅格层。

## 定向验证

| 验证 | 结果 |
| --- | --- |
| paint-core adjustment | 3项通过：默认/取消/提交/撤销重做、原图来源与非法值、羽化/偏移/低驻留分页历史 |
| paint-ops lib | 10项通过，含独立红→绿、灰度、透明与半透明数值断言及原算子回归 |
| paint-gpu lib | 2项通过：实际GPU 8192像素四参数组合对照CPU（断言最大误差<2e-5），原设备失效信号回退 |
| Qt软件 | 三功能×CPU/auto共6行通过，另init/cleanup，总8；真实模态窗口、鼠标确定、激活后Escape、预览与撤销/重做画面逐像素恢复 |
| Qt D3D11 | 同6行通过，另init/cleanup，总8；断言实际窗口graphicsApi为Direct3D11；具名历史检查通过 |
| C11/C++20 × 静态/共享 | 4项通过；新增DTO大小65576/LUT偏移24、非法参数输出归零、排队BEGIN/UPDATE/CANCEL及活动flag检查 |
| 相关严格Clippy、fmt、生成头 | 通过 |
| Release与完整应用D3D11启动绘画冒烟 | 标准build/bin/Release已更新，--preview成功生成实际应用截图，无QML加载错误 |

Qt记录和截图在忽略目录 `artifacts/adjustment-*`；最终像素与具名历史轮为 `adjustment-final-software.txt`、`adjustment-final-d3d11.txt`，此前轮次日志保留。Release构建日志为 `adjustment-final-build.log`。不将init/cleanup计成功能行。

## 原失败与边界

1. 初始构建因终端未加载MSVC头文件路径失败；加载Developer PowerShell后继续。并行QML缓存编译两次报告访问拒绝，单任务构建后通过；不将原失败删除或归因于算法。
2. 首个低历史预算夹具使用128字节，构造时被现行文档预算契约拒绝；改用既有合法最小级别 `2*TILE_BYTES+256` 后原像素与撤销断言通过。
3. 首轮Qt三个功能在重新显示后立即发送Escape时失败，保留 `adjustment-software.txt`。补窗口显示内容焦点恢复，并在测试中等待实际窗口激活后发送真实按键；随后软件与D3D11通过。
4. 新代码的Clippy格式/测试模块位置断言曾失败，修正后严格定向检查通过。

没有运行总check、所有Qt功能矩阵、性能基准或实体笔/跨物理屏幕测试；旧失败保持。不宣称Photoshop/PhotoCraft完整功能或逐像素等价。取消等待当前计算批次完成，无用户批次内中断；无调整层、直方图、自动色阶、分色范围或智能对象支持。


## 2026-10-28 紧凑窗口及导航续接

窗口改为NonModal，标题与浮窗一致28px/9px并保留全应用UiScale；紧凑控件与Ctrl+M/L/U入口已接通。原ApplicationModal记录为历史行为。预算/采样与最终最小验证见 `docs/platform/general-gpu-composition-design.md`、`docs/validations/platform/general-gpu-composition-validation.md` 末节。
