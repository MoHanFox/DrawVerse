# 图层组与透明背景验收

2026-10-08，Windows 11 x64 / MSVC / Rust 1.99.0 / Qt 6.8.3。设计：[图层组](groups-design.md)、[透明背景](transparency-design.md)，接口：[ABI 1.5](../contracts/abi.md)。插件最后，蒙版为下一独立模块。

## 实现

- 后序连续子树、稳定父 ID、最多 16 级组 / 4096 节点；组采用隔离合成，混合、opacity/fill 在子树合成后应用一次。分组、解组、移入组、移出到上一级、组内新增；重新归组保持像素位置。
- 组的完全/位置锁继承给后代；组无透明像素锁。移动组预检全部后代，一个事务修改像素层整数位置；任一锁、越界或历史失败均不部分提交。删除组删除完整子树，保留至少一个像素层。组节点不能直接绘画。
- 结构历史仅含元数据，不复制像素或瓦片引用；删除子树使用已有无损压缩/磁盘历史。DVH3 为内部易失历史编码，含节点类型与父 ID；不作为文件格式。写失败保留原文档、修订及历史，读失败不部分恢复。
- 普通图层保留原 Normal 合成快路径。组使用逐行隔离合成和有界缓存，依赖可见像素瓦片与组元数据；隐藏/零填充祖先不读取冷页。publication 用一次后序扫描生成层级元数据，避免每层线性搜索导致二次复杂度。
- ABI 1.5 新增 24-byte LayerHierarchy、40-byte GroupRequest、publication 查询与 FIFO 操作；旧 DTO/符号不变，cbindgen 自动生成。UI 只经 PaintCoreClient 提交小命令；组操作、渲染和 IO 均在 Rust 后台。
- QML 文件夹、缩进、展开/折叠、分组按钮和组操作菜单；折叠状态在同一文档的停靠/浮窗间共享，代际改变清除；移入菜单仅打开时创建条目。图层/组缩略图继续空闲后台渲染。
- ORA 标准嵌套 stack、显式 isolate、组与像素层属性、选中节点和 dv 扩展回环；重开重新分配 ID 并恢复父子映射。外部隔离组可导入，穿透/非零组位置/过深/未知属性明确拒绝。ORA 总节点仍限 256，像素仍为 8-bit sRGB，非标准混合依靠 dv 扩展。
- 插入需求完成：主画布和导航用同一 24×24 白灰纹理平铺，每格 12 逻辑像素。空白、全部隐藏和局部透明显示正方形格子，绘画纹理覆盖在上方；图案定位画布左上角，缩放后格子尺寸稳定。没有画布大小的背景位图、逐像素 CPU 合成或成千上万个方块委托；导出 alpha 不变。

## 验证结果

完整 `python tools/check.py` 通过：145 项 Rust 测试、fmt、clippy -D warnings、doc test、release 构建、生成头漂移；四组 C11/C++20 static/shared ABI、Qt Test 和高 DPI 共六个 CTest 组全部通过。日志 `artifacts/groups-transparency-full-check.log`。

新增核心回归覆盖隔离组只扣一次 opacity、层级/深度/循环拒绝、结构撤销、锁继承、移动全量预检、最后像素层保护、子树删除及无损冷页恢复。噪声子树强制磁盘历史，逐位比对撤销像素；大型结构命令注入写失败后文档/历史不变，正常提交能编码、撤销和重做。

渲染覆盖全部 27 种组混合、嵌套、LOD、组显隐/Fill、移动/重新归组/undo/解组后的缓存失效，和核心采样及无缓存输出比较。隐藏与零 Fill 祖先通过 page_reads 检查确认不加载冷页。文件测试覆盖嵌套 ORA、父子关系及选中组、原图层扩展、三种合成导出和外部组/恶意层级；旧“组不支持”案例更新为“穿透组不支持”。

FFI 合计 35 项回归（21 lib + 13 async + 1 storage）；新增 DTO 尺寸/偏移、名称同步复制、publication 失效、循环错误、重新归组/undo/解组与非法请求。C/C++ 四种真实链接程序也执行组操作并断言旧/新布局。

Qt Test 20 个功能项（加初始化/清理共 22）通过；两倍 DPI 5 个功能项（共 7）通过。新增用真实 QML 控件测试分组、组不能绘画、组内新层、折叠/浮窗保持、组 opacity/移动/锁、菜单移出、撤销/解组、保存重开。透明背景测试抓取实际窗口像素，检查四个方格白/灰交替、缩放正方形、笔触遮盖、两个图层全隐藏、导航一致及 PNG alpha=0。

常用 `build/qt` 构建和六组 CTest 也通过（`groups-native-build.log` / `groups-native-tests.log`）。原生 Windows 图形后端执行两项新功能测试、截图并检查布局；日志 `groups-native-preview.txt`，截图 `artifacts/transparency-canvas.png` / `artifacts/groups-panel.png`。截图显示组 raw 缩略图、50% 组 opacity 的笔触及透明背景；归回浮窗会创建独立面板组，较小面板中的子行可滚动。

## 性能与范围

性能在构建/测试全部完成后逐项运行。固定原生 600 点负载：DPR 1.5、1529×1019 输出，进程累计 CPU 359.375ms，画面间隔 p50/p95 16/17ms，追上最终修订 32ms，GUI 定时器 p95 9ms。上一轮 CPU 328.125ms、p95 同为 17ms；单次测量有波动，不承诺新增功能无成本。该数值含启动/退出和全部线程，不是任务管理器瞬时百分比。报告 `artifacts/groups-ui-performance.json`。

核心 120 帧、1800×1200：普通层平均 5.114ms / p95 5.499ms；相同笔触放入两级隔离组平均 5.729ms / p95 7.420ms。均为缓存渲染，13152 命中 / 369 未命中，10003024 字节 / 122 项；日志 `groups-flat-performance.log` / `groups-nested-performance.log`。复杂模式/大量组/冷页负载不能由此推导，Rust GPU 尚未实施。

8K 单笔普通层：绘画 3616.697ms、提交 252.372ms、冷/热概览 432.858/9.652ms、undo/redo 285.762/274.860ms；16384 live 瓦片逻辑 1GiB、驻留 256MiB。完整撤销/重做和像素位比对通过；日志 `groups-8k-performance.log`。

尚未实现穿透组、蒙版、选区、一般变换、ICC/GPU/PSD，真实数位板及 macOS/Linux 仍待对应阶段验证；不等同于 Photoshop 私有算法一致性。

```sh
# 激活平台编译器、Rust、CMake、Qt 后在项目根目录
python tools/check.py
cmake --build build/qt --config Release
ctest --test-dir build/qt -C Release -R "^(abi_|qt_ui_)" --output-on-failure
./build/qt/bin/Release/drawverse.exe

# 性能测量逐项运行
python tools/measure_painting.py build/qt/bin/Release/drawverse.exe artifacts/groups-ui-performance.json
cargo run --manifest-path core/Cargo.toml -p paint-render --example painting_bench --release --locked
cargo run --manifest-path core/Cargo.toml -p paint-render --example painting_bench --release --locked -- --groups
cargo run --manifest-path core/Cargo.toml -p paint-render --example paging_bench --release --locked
```
