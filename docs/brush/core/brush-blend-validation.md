# 画笔颜色混合内核验收

2026-10-14，设计见 [brush-blend-design.md](brush-blend-design.md)，无ABI变化；工具栏/原生新入口尚未接入。

27种颜色混合复用现有Rust BlendMode计算，组穿透及非正常擦除/涂抹拒绝。普通像素、灰度蒙版、透明锁分别处理；非正常模式使用整笔贡献与before目标重算，不改变图层appearance。默认正常路径与旧接口保持原行为。溶解按稳定文档坐标/捕获笔触序号取样；下一笔重新混合，历史恢复实际像素。

相关最小验证：

```text
cargo test --manifest-path core/Cargo.toml -p paint-core --test brush_blend --test brush_program --test brush_mask --test brush_opacity --test masks --locked
cargo test --manifest-path core/Cargo.toml -p paint-core --lib brush_smoothing --locked
cargo clippy --manifest-path core/Cargo.toml -p paint-core -p paint-io -p paint-ffi --all-targets --locked -- -D warnings
```

前者27项（新混合6、旧图片5、整笔透明度7、程序笔尖3、蒙版6）通过，平滑7项通过，clippy与fmt/diff检查通过。新测试覆盖：

- 27模式×透明/半透明/不透明目标逐像素参考；Multiply实际加深及ColorDodge提亮，图层appearance保持。
- 整笔密集回擦及不同事件拆分一致，下一笔再次施加、捕获模式/撤销重做；同笔不会重复混合before结果。
- 27模式的透明锁保留alpha位值，27模式的蒙版可见灰度参考。蒙版先保留旧显示灰度转换，再把可见结果转内部隐藏量，不能直接把隐藏量当作颜色。
- 溶解同笔不闪烁/新笔不同、真实历史；非法工具/组模式拒绝、超预算段在写入前拒绝并可取消恢复。
- 图片笔尖、纹理、第二覆盖、逐印章颜色、透明度压力、羽化选区和分页同时开启时，与独立生成的正常整笔颜色贡献对before目标混合逐像素一致；真实分页有page_writes证据。

未运行全量tools/check.py、Qt新功能测试、ABI新增入口或跨平台/实体设备/总体性能/PS逐像素验收。当前开发版没有画笔混合模式下拉，不提前升级ABI或宣称用户入口完成。下一阶段接原生/Qt工具状态、持久化及有色底真实预览；双色复合/自动描边继续后续。
