# 液化常驻纹理预览验收

2026-10-19，Windows 11 / Qt 6.8.3 / RTX 4060 Laptop GPU，分支 `feat/wgpu-support`。设计见[常驻预览设计](../../../tools/liquify-resident-preview-design.md)。

## 范围

ABI 1.62 新增显式 deferred begin 与有身份票据的源/网格读取。兼容无选区顶层 normal 栅格栈在拖动时只执行 Elastic、发布有界几何，Qt 场景图复用三张源纹理；完成才全精度重采样。15000px 为画笔直径。CPU 设置、软件场景图与复杂图层组合保留像素路径。共享算子、原图、完整稀疏位移场与历史仍由核心管理。

## 最小验证

- 核心 `cargo test -p paint-core --release --locked --lib deferred_drag`：1 项通过，覆盖四类工具、15000px、非零图层偏移、负坐标源、拖动无 Remap/RemapBlocks、源/像素瓦片身份保持、局部历史/取消/重置，以及完成与既有像素路径完全相同。
- 既有 `liquify`：11 项通过、1 项性能用例保持 ignored；`liquify_elastic`：3 项通过，含画布外恢复和历史。日志 `artifacts/liquify-resident-core-regressions.txt`。
- ABI `cargo test -p paint-ffi --release --locked --test async_session deferred_liquify_packets`：1 项通过，验证源身份、精确网格 revision、静态源允许同会话旧 revision、错误 generation、容量探测/不足不改缓冲区、直接预览期间 CPU frame 为 BUSY、完成后恢复帧。
- 定向 Clippy：`cargo clippy -p paint-core -p paint-ops -p paint-ffi --lib --locked -- -D warnings` 通过。最终 FFI 重编译受 Rust 1.99 分离 std metadata 的链接模式问题影响，改用 `cargo clippy -p paint-ffi --lib --locked -- -D warnings --emit=metadata` 通过（覆盖其核心依赖），未修改工具链；格式与 cbindgen 生成头检查通过。
- 四项 C/C++ 静态/动态 ABI 运行通过。完整 Windows Release 构建通过，应用位于 `build/verify-ui/bin/Release-wgpu/drawverse.exe`；用户仍运行原 Release，未终止或覆盖运行中的应用。

- 真实 D3D11 场景图 `liquifyResidentMeshRendersAndCommitsWithoutCpuFrames`：1 项通过，2000/15000px 均自动选择直接预览；验证线性空间半透明正常合成（RGB误差≤3）、网格实际改变画面、CPU帧不变、静态源身份不变、64事件合并最多两次变形修订、局部撤销重做、2000px取消与15000px提交/文档撤销。显示网格49601节点。日志 `artifacts/liquify-resident-qt-gpu.txt`。
- 软件 Qt `maximumLiquifySizePublishesPixelsAndKeepsLocalAndDocumentHistory`：1 项通过，原像素预览/局部历史/完成单历史及文档撤销重做正常。日志 `artifacts/liquify-resident-qt-software.txt`。新建/打开已继承会话计算执行器，移除 Qt 随 document generation 再提交一次相同硬件设置的行为，避免与起笔竞争。

Qt硬件用例的64输入突发至异步 GPU 图像取回均约73ms；该数字含 `QTRY_VERIFY` 的50ms轮询步长及异步抓图等待，不能当作正常显示帧延迟或16ms预算已通过。核心及场景图节点同步另经临时计时：实际变形约2ms，节点更新约0.24–0.97ms。临时计时代码已移除。最初使用同步 `QQuickWindow::grabWindow()` 与待处理的线程渲染更新相互同步，产生约1秒 sync 阻塞；改用真实场景图异步 `grabToImage()` 后消失，未改产品渲染循环。基本渲染循环加同步抓图的诊断组合曾抛异常；最终验证使用 Windows 默认 threaded RHI + 异步抓图，不宣称该诊断组合通过。

## 定向性能

使用 `CARGO_TARGET_DIR=build/gpu-target`，执行 `cargo run -p paint-gpu --release --locked --example liquify_compute_performance -- auto --resident`，15000px 另加 `--large`。场景为真实有色/透明的 2048×1536 原图，六次连续推移或旋转；每次包含算子上传/网格读回、核心场更新和预览网格生成，不包含 Qt 轮询、显示或静态源准备。局部历史收笔另计，完成读取原始全精度源。日志为 `artifacts/liquify-resident-profile-{2000,15000}.txt`。

| 画笔直径 / 工具 | 六输入合计 | 单输入范围 | 收笔历史 | 完成重采样 |
| --- | ---: | ---: | ---: | ---: |
| 2000px 推移 | 17.41ms | 2.51–3.52ms | 0.43ms | 433.91ms |
| 2000px 旋转 | 14.57ms | 2.21–2.74ms | 0.39ms | 413.73ms |
| 15000px 推移 | 11.25ms | 1.71–2.54ms | 0.50ms | 468.59ms |
| 15000px 旋转 | 15.96ms | 2.04–3.02ms | 0.44ms | 714.30ms |

首次 wgpu 延迟编译移至进入会话的恒等网格预热；本次记录首次进入约151ms / 22ms，后续约0.4–0.5ms，驱动冷状态会影响结果。静态源代理准备另在后台执行，未计入这些核心数字。不能把单次记录当作所有设备、持续输入手感或 Photoshop 等价结论。

## 限制与环境

- 无选区、顶层正常混合的普通栅格栈才能启用直接显示。组、蒙版、剪贴及特殊混合继续原像素预览。
- 每张代理最长边2048，显示网格最多257×257；源代理与三角插值允许预览近似，完成使用原始像素与权威场。未实现智能对象网格持久化。
- wgpu 计算与 Qt RHI 显示分别拥有设备，网格仍需读回；不存在跨库零拷贝纹理共享。设备选择作用于计算，Qt 显示设备由 Qt 决定。
- 现有全局细化与8M网格预算仍保留；大范围粗网格后切换极小画笔可能触达细化预算，不宣称所有组合恒定成本。
- C 盘可用空间约160MiB，低于默认暂存安全余量512MiB，最初测试初始化失败。测试仅在子进程将 TEMP/TMP 指向 `artifacts/resident-test-temp`（D盘）；没有改全局临时目录、用户配置或删除用户数据。
- 未运行全量 `tools/check.py`、跨平台、实体输入/连续手感或长时间 GPU 占用验收。

## 2026-10-21 套索液化常驻预览

选区不再强制关闭兼容普通根层的deferred预览。CPU场权重和静态遮罩共用规则间距的按行覆盖采样；ABI1.77 bit2/kind2提供最长边2048的目标坐标R8遮罩和原图UV变换。Qt缓存静态遮罩，片元按覆盖混合恒等源与变形源；拖动只更新位移网格，最终仍用完整原图/选区物化。

- 核心deferred测试一项覆盖四工具×无/硬/羽化选区12组合，确认与像素路径逐瓦片相同、源瓦片身份不变、拖动零Remap及局部/文档历史正确。选区采样三项通过，新增规则间距0.25/1/2/8.5/128、自交多边形/羽化/AA及减去椭圆与标量结果一致。
- 公开液化11项通过、1手工项忽略。原生新选区包与既有无选区网格包各1项通过，验证尺寸/覆盖、身份过期、容量不足不写入、结束后NOT_FOUND及CPU帧不变。
- Qt新增硬边与8px羽化套索两行在软件及真实D3D11各通过，实际2048×1536画布/2000px画笔，检查红色形变、选区外蓝色及白色不变、恒等显示、静态mask身份、GPU期间CPU帧不变、局部撤销/重做、取消/确认及文档历史。输入至抓图72–74ms含测试轮询，不能称显示帧延迟。
- D3D11既有2000/15000px无选区网格一项通过；四ABI、paint-core/paint-ffi所有目标严格Clippy、fmt/生成头及完整标准Release通过。

共享回归补充：公开选区10项，4K选区变换五行及复合移动七行，软件/D3D11各12行通过。软件首次整批的masked-group抓图失败，单独及最终整批重跑通过；保留首次失败日志，不称每次均稳定。软件既有大尺寸液化一项通过，未屏蔽或放宽断言。

固定有色/透明2048×1536原图、四边套索/2px羽化、2000px笔刷、六输入、三次中位数：像素预览1441.085ms，场更新和网格生成17.376ms；两路径完成后摘要均28330f25c20ea1cb。只测核心拖动，不含源/遮罩准备、首次上传、Qt显示及最终确认；不能代表实体持续手感或PS等价。

日志artifacts/selected-liquify-{core,coverage,selection,public,native,native-shared,software,d3d11,pixel-perf,mesh-perf,shared-d3d11,abi,clippy,fmt,header,build}.txt。性能示例原来只对deferred执行确认，导致预览与最终摘要不一致；已统一两条路径确认后比较。

目标自身蒙版/组内及其他不支持组合仍像素回退；2048代理/遮罩和网格插值是预览近似。极小选区与巨大笔刷粗场节点漏采语义未改变；12组合有选区时使用128px笔刷，无选区使用15000px，不混称该边界已覆盖。state45/behavior37/VBR1.0保持，用户程序位于build/bin/Release/drawverse.exe。未跑全量、跨平台、实体持续输入或8K/16K整图。

## 2026-10-21 复用祖先快照的组内液化

液化进入时复用移动/变换的resident_hierarchy_context：完全不透明、可见、无蒙版/剪贴且不依赖外部背景的Normal/PassThrough祖先可在显示快照中消去，活动普通层按快照parent与原顺序分割上下栈。原始图层、选区、源、场和历史保留真实层级；通用变换根层判定未放宽。自身/祖先蒙版、半透明祖先及特殊混合仍走像素回退。

- 核心预览4项通过：其中同一测试扩大至4工具×无/硬/羽化选区×根层/两级Normal/两级PassThrough共36组合，拖动零Remap、源瓦片身份/场历史保持，最终逐瓦片与CPU像素路径相同，确认及文档历史不改变parent。另一个测试包含7类不安全源/祖先，均保持回退。新增范围不等于任意深度/任意混合组合已验收。
- 公开液化11项通过、1手工项忽略；共享核心变换14、原生选区mask包1通过。严格core/ffi全目标Clippy、fmt/生成头、四ABI及完整标准Release通过，标准输出build/bin/Release/drawverse.exe。
- 实际2048×1536/2000px笔刷的Qt硬边/羽化根层、两级Normal/PassThrough及祖先蒙版回退，共软件/D3D11各5行通过。验证源/mask身份、GPU拖动CPU帧不变、选区外蓝色与白色不改、局部历史、取消/确认和文档撤销重做；祖先蒙版用例明确不发布resident。既有D3D11无选区2000/15000px路径亦通过。

首次软件批次原根层羽化用例的一次撤销抓图未匹配，独立重跑通过。该断言改为按相同RGB容差等待实际渲染像素匹配，不用固定延时或删除校验；最终软件/D3D11整批通过。首次核心命令使用错误模块过滤得到0项，未作为通过证据，最终按完整真实模块执行4项。

日志artifacts/liquify-nested-{core,rejections,public,transform-core,native,clippy,fmt,build,final-build,software,software-rerun,d3d11,final-software,final-d3d11,shared-d3d11,abi}.txt。ABI1.78/state45/behavior37/VBR保持；未新增wgpu算子或改变CPU最终取样。静态资产进入成本、2048近似、极窄选区粗场边界、复杂蒙版组合及实体持续性能仍待，不宣称全量或任意大图瞬时响应。
