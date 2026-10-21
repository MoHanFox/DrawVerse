# 复杂移动有界交互采样验收

2026-10-21，Windows，同工作区Release；[设计](../../tools/layer-move-bounded-fallback-design.md)。

## 核心与原生

- `cargo test --manifest-path core/Cargo.toml -p paint-render --locked`：23项通过，含新增2个单元/2个集成。区段工作量上限、负视口、Normal/Multiply/PassThrough组、固定/祖先蒙版、偏移、精确线性参考、sRGB误差≤1、取消/非法请求、最终像素和历史均覆盖。
- `paint-ffi`的`complex_move_coarse_frames_restore_full_resolution_on_commit_and_cancel`：通过。两个视口在固定蒙版移动时从1024降为512，松手/取消恢复1024，最终偏移及历史保持。
- 既有`layer_move_resident_packets_absolute_updates_and_final_barrier`通过；常驻源和最终发布屏障不变。
- Clippy（paint-ffi lib含依赖，warnings deny）与workspace格式检查通过。

## 窗口与构建

- `qt_transform_tests::compoundLayerMovesKeepResidentSources`七行在软件Qt与真实D3D11各通过：链接自身蒙版、带蒙版根组、连续多选、两级Normal/PassThrough组内子层、固定自身蒙版、祖先蒙版。真实鼠标32次移动、松手前显示、一次历史、确认和撤销；前五类硬件常驻源身份与四顶点保持，后两类确认使用回退。
- 完整Release构建（主程序、三类Qt测试、四套ABI）通过；cbindgen头一致。四套C11/C++20静态/动态ABI依次运行通过。
- 产物：`build/qt/bin/Release/drawverse.exe`。Qt测试使用96px画布验证功能，4K性能只依据下面的CPU基准，不混为实体4K交互验收。

## 固定性能样本

`cargo run --manifest-path core/Cargo.toml -p paint-render --release --example move_preview_bench --locked -- 3840 2160`；对应2000×2000再跑一次。完整彩色层、固定自身蒙版、70%祖先组、白底；5次绝对位移，各自中位数。旧CachedCpuRenderer与新有界渲染对同一快照比较，输出请求最长边1024。

| 文档 | 完整预览 | 有界预览 |
| --- | ---: | ---: |
| 3840×2160 | 394.346ms | 42.927ms |
| 2000×2000 | 252.732ms | 48.740ms |

仅CPU画布渲染，不含输入、排队、Qt显示或结束后的最终帧；不是实体连续输入时延。预览最长边512、最近邻及色阶近似；任意深组/混合的合成成本仍与层数有关，首次资产、冷页IO和最终完整渲染仍可能耗时。未跑全量、跨平台或实体持续手感验收。ABI1.74/state44保持，无新增GPU计算。

## 2026-10-21：稀疏坐标合成及真实4K窗口

核心新增 `sampled_rows` 三项通过：连续/稀疏结果逐像素完全一致，覆盖全部27种普通混合（含按原坐标溶解）、普通/孤立剪贴/PassThrough剪贴三条路径、有效黑色组蒙版、透明度/fill、正负偏移、瓦片边界与重复坐标；另外验证隐式白底、空/无序/越界/数量校验。低驻留12瓦片文档只请求两块瓦片的坐标，页面读数不超过2；连续整行需至少10次冷页读，结果一致。

`cargo test -p paint-render --release --locked`共23项通过。新增16K及MAX_DIMENSION稀疏画布/负及全外视口，输出512×8，逐像素与精确中心参考一致；这只是范围和采样工作量验证，不是16K密集整图性能验收。共享groups/clipping/pass_through及新采样/interactive定向Release28项通过。原生 `complex_move_coarse_frames_restore_full_resolution_on_commit_and_cancel` / `layer_move_resident_packets_absolute_updates_and_final_barrier`各1通过，覆盖两个视口、完整分辨率恢复及结束屏障。

真实Qt `complex4kLayerMoveRestoresFullFramesAndHistory`两行使用3840×2160完整红色层、有效黑色固定自身蒙版/祖先蒙版；鼠标12次拖动，松手前512px回退、固定遮挡原位、Esc取消/确认恢复完整帧、单历史及undo/redo像素均通过。连同原七类复合移动回归，软件Qt与D3D11各11行（含init/cleanup）通过，无失败或跳过。自动序列起点到抓图软件142–143ms、D3D11 168–190ms，包含12次QTest鼠标事件及轮询/抓图，不是单输入延迟，未设时间阈值。首轮新测试误用白色蒙版种子（显示而不遮挡）导致初始像素断言失败；已修正黑色，保留遮挡断言后通过，没有改生产蒙版规则。

同机、同一既有 `move_preview_bench`种子/请求，5次绝对位移各自中位数：

| 文档 | 改前区段采样 | 改后稀疏坐标采样 |
| --- | ---: | ---: |
| 3840×2160 | 46.208ms | 20.461ms |
| 2000×2000 | 48.657ms | 32.193ms |

仅CPU预览帧合成，不含GUI、准备、排队、GPU显示、确认或冷盘；种子带固定自身蒙版结构和70%祖先组，其白色蒙版笔触不产生遮挡，实际有效遮挡由上述新4K窗口用例另验。相同帧像素中心、512px尺寸和色阶近似保持，未改变采样质量换取耗时。基准旧完整渲染的4K421.869/397.348ms、2000px264.361/255.013ms用于观察同机波动，不算本次优化对象。

严格Clippy（core/render/ffi全部目标）、fmt、生成头、四套ABI以及完整标准Release构建通过。产物 `build/bin/Release/drawverse.exe` 已更新，ABI1.78/state45/VBR不变。首次常驻资产、深组层数、冷页IO、最终完整渲染及实体持续手感仍待，M1部分完成。

日志：`artifacts/move-sparse-{before,after}-{4k,2k}.txt`、`move-sparse-final-{core,render,software,d3d11,clippy,fmt,header}.txt`、`move-sparse-{regression,native,abi,release-build}.txt`；测试种子修正后Qt目标重编译见 `move-sparse-mask-build.txt`。
