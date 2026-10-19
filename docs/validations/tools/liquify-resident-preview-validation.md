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
