# 起收笔实际绘画验收

2026-10-16，Windows。对应 [设计](brush-taper-document-design.md)，前置 [数值与调度内核](brush-taper-kernel-validation.md)。

已接Rust Document真实执行：`begin_tapered_brush_stroke`与`begin_configured_tapered_wet_brush_stroke`捕获参数，现有结束/取消/历史入口继续共用。仅活动笔尾保存图层/两份贡献场的首次写入Arc和有界执行检查点，未克隆整笔before或完整贡献映射；恢复后调用原绘画、橡皮、涂抹和双色算子。第二图片笔尖在最终尺寸缩放后重新计算mip/几何。原双流1e-9位置平局规则只规范累计弧长，保留设备点和顺序。滤波单次输入累计计算所有笔尾重绘预算，不能通过拆成4ms点绕开限制。

实际相关验证：

```text
cargo test --manifest-path core/Cargo.toml -p paint-core --lib taper_document --locked
cargo test --manifest-path core/Cargo.toml -p paint-core --lib brush_smoothing --locked
cargo test --manifest-path core/Cargo.toml -p paint-core --test brush_dual_color --test brush_dual_path --test brush_blend --test brush_wet_smudge --test brush_opacity --test brush_texture --test brush_taper --locked
cargo clippy --manifest-path core/Cargo.toml -p paint-core --all-targets --locked -- -D warnings
cmake --build build/verify-ui --config Release
ctest --test-dir build/verify-ui -C Release -R "^abi_(c|cpp)_(static|shared)$" --output-on-failure
```

使用现有工具链包装器、Qt6.8.3 windows平台/software Quick、环境倍率1。最终10项新Document用例通过，8项平滑回归通过，7个相关集成测试文件共52项通过，Clippy通过；Release及cbindgen生成头字节检查通过，4项C/C++ static/shared ABI通过。Qt实际现有双色绘画/捕获/交换/重开与湿混涂抹强度/手指策略/历史两项通过（含初始化清理4 passed，0 failed）。对应日志均在本机`artifacts/brush-taper-document-{tests,smoothing,regressions,clippy,release,abi}.log`和`brush-taper-document-qt.txt`，不入库。

- 新增仅测试参考路径：预先知道最终滤波弧长，立即冻结每个调度命令，按最终曲线只执行一次，完全不恢复活动尾部。40组最终像素逐点相等：10种算子/配置乘普通、选区、蒙版、偏移+透明锁四种目标，含随机散布/形状/颜色、方向/倾斜、主图片、纸纹/局部纹理、灰度/双色配对与独立路径。另验证第二图片缩放和极接近间距双流平局。
- 双色独立路径每次输入后，与对应当前终点的一次性参考逐像素相等，检查旧尾部被清理而非叠画。原始快照保持，真实撤销/重做和取消全部恢复，完成仅新增一条历史；无变化的蒙版橡皮允许不新增历史。首轮测试曾错误要求所有蒙版橡皮操作都发生变化，修正该测试前提后通过，像素检查保持。
- 湿混绘画、湿混涂抹和旧涂抹与一次性参考像素一致；湿混返回载体revision、网格大小及逐格颜色一致。实际文档仅允许1块像素驻留，触发真实暂存分页。该压力用例约60秒，未反复扩大到无关整套测试。
- 1024px长轨迹，整笔before超过8块瓦片，活动尾部记录不超过4、三类日志各不超过2块；独立透明度曲线验证上限不被当成流量，关闭和双零长度与旧绘画逐像素相等。
- 高平滑/拖绳下只使用滤波弧长，释放前后像素相等，不在原始鼠标终点补线。记录窗口预算失败、滤波累计重绘预算失败、非法后续输入及真实暂存盘写入失败均整笔恢复且不增加历史，包含湿混路径；普通不带尾部的相同滤波输入可成功，确认额外预算来自实际重绘工作。

尚未完成：起收笔预设字段/完整关闭基准、state/VBR保存交换、原生准备和C ABI、Qt设置及自动白色预览参数。当前不在用户界面提供起收笔，也不宣称完整笔刷系统已完成。大笔尖/极密集/超长活动尾部仍可触发明确预算错误并回滚，没有提高间距或静默丢印章。无ABI或格式版本变化；跨平台验收按用户要求后置。Windows开发版在确认应用未运行时同步本次构建，现有界面行为保持，后续接入完整参数链路。
