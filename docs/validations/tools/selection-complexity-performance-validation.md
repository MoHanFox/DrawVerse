# 套索、魔棒与油漆桶性能验收

2026-10-19，Windows 11 / Qt 6.8.3 / MSVC 2022，分支 `feat/wgpu-support`。公开 ABI 保持 1.63，选区、文件与预设格式不变。[设计及代码入口](../../tools/selection-complexity-performance-design.md)。

## 固定基准

同机 Release，基线为 `8e945b9` 的算法配合同一基准入口。每种场景三轮取中位数；优化后最终测量在构建结束后顺序执行，避免与编译争用 CPU。魔棒入口为 `core/crates/paint-core/examples/wand_performance.rs`，桶为 `bucket_performance.rs`。轮廓夹具见 `ui/tests/selection_tests.cpp`。

| 场景 | 优化前 | 优化后 | 范围 |
| --- | ---: | ---: | --- |
| 3840×2160 空白透明画布，连续、不透明白色桶填充 | 1996.184ms | 193.603ms | 2040 输出瓦片，含发现、实际像素写入和历史提交 |
| 1024×1024，单图层连续魔棒 | 247.275ms | 53.157ms | 容差20、AA、变化颜色源 |
| 同图，单图层全局魔棒 | 104.639ms | 45.490ms | 顺序扫描 |
| 同图，四图层合成源连续魔棒 | 422.631ms | 98.391ms | 同一合成取样规则 |
| 同图，四图层合成源全局魔棒 | 239.972ms | 87.768ms | 同一合成取样规则 |
| 256×192 碎片/孔洞掩码轮廓构造 | 35.403ms | 0.671ms | 单次几何构造，前后10323路径元素 |

魔棒夹具以16px条带与异或位置改变颜色，四层场景上层透明度0.2，选择结果前后文本 FNV 摘要均为 `c48fa654fb75dd26`。该固定场景覆盖大区域访问，并非任意复杂照片的耗时上界。桶夹具每轮新建空白文档，验证右下角实际像素以及撤销/重做；不把此结果外推为复杂4K图层栈的耗时。上述数字不包含 Qt 取帧、纹理上传及屏幕呈现延迟。

4096顶点密集套索原构造已约0.112ms，优化后记录0.505ms；此微小样本未证明套索几何构造提速。套索的本轮收益主要来自像素合成复用、无效全画布裁切消除和显示纹理保留。轮廓计时只用于记录，不设不稳定的毫秒断言。原始记录在忽略目录 `artifacts/selection-{wand,bucket}-{before,after}.txt` 与 `selection-outline-{before,after}.txt`。

## 核心、原生与接口

- `cargo test -p paint-core --lib color_region --locked`：4项通过。随机小图跨63/64/65/137边界与四邻接参考逐权重一致、孔洞、分数权重、访问预算，以及全局长窄画布超过4096源瓦片但匹配瓦片很少的兼容性。
- `cargo test -p paint-core --test bucket --test wand --test selection --locked`：6+2+10项通过；最终区域缓存修正后另复跑 bucket/wand 8项通过。新增桶逐像素参考覆盖24种组合：对齐/负偏移/跨瓦片偏移、透明锁、羽化选择、连续/全局、部分瓦片边界与撤销。现有测试覆盖平均取样、AA、组合、蒙版、组锁、资源失败和持久化。
- `cargo test -p paint-core --lib history_failure_is_atomic --locked`：1项通过，桶历史写入失败保持像素/历史原状。
- `cargo test -p paint-ffi --lib actor_tests --locked`：4项通过。新增纯选区 lasso/invert/undo/redo/wand/clear 保持像素 Arc 身份、更新发布修订、保持渲染屏障；下一次真实绘画、撤销绘画和新建文档正确刷新。现有常驻预览屏障、取色坐标和失败释放仍通过。
- 公开 API 四项定向通过：`wand_request_validation_copy_mask_query_history_and_undo`、`bucket_dto_validation_copy_real_pixels_and_single_history`、`common_selection_abi_copies_lasso_points_and_publishes_settings`、`selection_abi_publication_validation_history_and_stroke_gate`。
- `cargo clippy -p paint-core -p paint-ffi --lib --locked -- -D warnings --emit=metadata`、`cargo fmt --all --check`（core目录）和 `cargo run -p paint-api-gen --locked -- --check` 通过。metadata参数沿用当前MSVC环境规避链接工具问题，不替代Release构建。
- 最终Release的 C11/C++20 静态/共享四 ABI 用例通过。

核心与原生记录：`artifacts/selection-region-tests.txt`、`selection-core-tests.txt`、`selection-final-region-consumers.txt`、`selection-native-tests.txt`、`selection-public-api-tests.txt`、`selection-final-ctest.txt`。

## Qt 与 Release

- `qt_selection_tests`：3项功能，含初始化/清理共5 passed。随机掩码、对角触点、灰度128阈值、孔洞、Replace/Add/Subtract/Intersect/Invert、密集套索与越界裁切逐点正确；ctest offscreen/software通过。
- Windows software：8项功能，含初始化/清理共10 passed、0 failed。魔棒鼠标/合成平板/浮窗、套索/多边形持久化、密集套索绘画与撤销、桶输入路由与单历史、轮廓组合、蚂蚁线计时、默认光标、纯选区帧身份测试通过。
- Windows D3D11 RHI：5项功能，含初始化/清理共7 passed、0 failed。魔棒、套索、桶、默认光标与纯选区帧身份测试通过。
- 新 `selectionOnlyFramesKeepImageIdentityAndPaintingRefreshesIt` 验证 selectAll/undo/redo 更新发布修订且保留 QImage cacheKey；实际绘画改变 cacheKey 并出现真实像素。既有 CanvasItem 以该身份复用 QSG 纹理。该测试验证缓存条件，不是GPU占用率或持续FPS测量。
- 笔刷轮廓线宽在当前代码中已默认0，本轮只确认新配置与Qt回归，不覆盖用户已保存的线宽；0仍显示自适应细线。
- 完整Windows Release构建通过。产物 `build/verify-ui/bin/Release-wgpu-selection/drawverse.exe`，Qt依赖与许可同目录；最终 `paint_ffi.dll` 已从本轮Rust Release复制到该目录并进行ABI/Qt回归。未终止旧版用户程序。

Qt记录：`artifacts/selection-qt-software.txt`、`selection-qt-hardware.txt`；构建记录 `artifacts/selection-ui-build.txt`。测试临时目录只在子进程指定，未修改系统TEMP策略。

## 边界

区域生长、选色、轮廓几何、蚂蚁线绘制仍在CPU；硬件后端复用Qt画布纹理，没有新增wgpu选色算子。帧像素复用仍经过既有ABI取帧与Qt内容比较，不宣称整条链路零拷贝。多条复杂选区路径的通用布尔组合仍可能开销较大；CPU软件显示保留。

连续区域缓存和待处理段栈有显式资源上限，超限报错而不部分发布。未运行全量 `tools/check.py`、跨平台、实体输入持续手感或用户原始复杂图文件验收；合成Qt事件与固定基准不代表任意图像立刻完成。
