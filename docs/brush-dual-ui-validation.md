# 双笔刷原生与 Qt 入口验收

2026-10-14，ABI1.43 / state12 / VBR1.10 / behavior9。设计见 [brush-dual-ui-design.md](brush-dual-ui-design.md)。本阶段完成用户设置与原生资源交换，不代表完整笔刷系统完成。

## 已实现

- 双重笔刷分类及笔尖/设备压力/散布/形状抖动四子页；六种覆盖组合、程序与图片、独立路径及配对模型，第二直径/间距/流量与完整动态独立于主笔尖。关闭仍可编辑，图片硬度由图片决定；压力编辑器复用原控件。
- PNG 导入捕获目标预设及第二角色，改选不串预设，删除目标安全失败。主图片与纹理保持独立。完整关闭值、曲线和64位种子经复制/state12/VBR保留；非法模型、范围和第二不透明度/颜色被拒绝。
- 新默认为关闭、独立路径、24px/25%间距；默认对象可省略，显式配对不变成独立路径。旧无双对象像素与预览保持旧行为。
- 启用双笔刷时真实预览使用两者实际尺寸、统一视口缩放，修改主尺寸会改变相对第二覆盖。预览与绘画仍由 Rust 执行，光标明确只显示主笔尖。
- 库包入口解除临时拒绝，导出包含主/纹理/第二及来源基线PNG并集，同源资源只存一次；缺失资源不覆盖目标。无库资源上下文的旧程序导出接口仍明确拒绝双对象。

## 验证证据

Windows 11 / Qt6.8.3 / MSVC / 软件 Qt Quick；相关最小测试，未运行全量 tools/check.py。

```text
cargo test --manifest-path core/Cargo.toml -p paint-core --test brush_dual --test brush_dual_path --locked
cargo test --manifest-path core/Cargo.toml -p paint-io --test brush_dual_io --test brush_dual_exchange --locked
cargo test --manifest-path core/Cargo.toml -p paint-ffi --test brush_source --locked
cargo test --manifest-path core/Cargo.toml -p paint-ffi --lib brush_preparation::tests --locked
cargo clippy --manifest-path core/Cargo.toml -p paint-core -p paint-io -p paint-ffi --all-targets --locked -- -D warnings
```

分别16/6/9/11项通过；默认路径/显式配对、真实覆盖和旧像素、状态/包恢复、第二独占与三角色共享PNG、失败原子性、在途资源捕获与释放重建均有对应回归。cargo fmt通过。生成头由paint-api-gen生成，CMake严格头检查通过。

Qt Release构建通过。新增：

- `dualBrushStateRetainsClosedDynamicsAndRejectsInvalidData`：关闭配置、完整种子/曲线、复制、最高state12、配对null及VBR恢复、非法数据拒绝。
- `dualSettingsImportPreviewPixelsHistoryAndExchange`：实际分类/复选框/关闭数值输入、子页压力点/散布/形状开关；导入捕获与删除目标；真实预览及主第二尺寸比例；笔触中改配置不影响捕获的笔触；不透明度上限/真实条纹/撤销重做；删除输入后在新资源目录导入VBR绘出逐像素相同帧。

DPR1/2上述真实界面测试通过，无QML警告；截图 `artifacts/dual-ui-1.png` / `artifacts/dual-ui-2.png` 已检查，窄列数值不越界、导入/程序按钮可见，底部预览固定。既有纹理、颜色、压力曲线、来源分类回归通过。C/C++静态/动态ABI smoke共4项通过。git diff --check通过。

## 剩余范围

双笔刷组合轮廓、PS双笔刷逐像素/实体数位板手感、跨平台与总体高级组合性能仍未验收。平滑/收尖、完整湿混、ABR高级动态与旧程序/16位采样、VBR分组扩展/最终冻结继续后续；没有用局部测试宣称整个系统完成。
