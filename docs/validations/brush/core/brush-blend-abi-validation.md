# 画笔混合原生入口验收

2026-10-14，ABI1.44，设计见 [brush-blend-abi-design.md](../../../brush/core/brush-blend-abi-design.md)。新增16字节PaintBrushBlendDesc及paint_session_begin_prepared_preset_with_blend、PAINT_FEATURE_BRUSH_WRITE_BLEND=549755813888。原Tool/Background/Point DTO布局和所有旧导出签名不变，旧prepared入口显式正常；state12/VBR1.10无改变。

新模式/颜色在函数返回前复制，与准确目录票据一起进入actor；actor等待原票据再应用捕获模式，库销毁和后续修改不改变已提交笔触。错误模式/保留值/大小/对齐/空指针以及非正常擦除组合在排队前拒绝、sequence=0。

```text
cargo test --manifest-path core/Cargo.toml -p paint-ffi --test async_session --test brush_catalog --locked
cargo clippy --manifest-path core/Cargo.toml -p paint-ffi --all-targets --locked -- -D warnings
cargo run --manifest-path core/Cargo.toml -p paint-api-gen --locked
```

异步35项/目录2项通过，新增prepared_blend验证上述拒绝边界、调用后改模式/颜色、库销毁后的真实Multiply/Screen像素、单条笔触历史与精确撤销重做。原背景捕获测试扩展到新正常入口，与旧有背景入口逐像素相同、均不同于白色默认旧入口，且颜色动态读取捕获两色。既有准确准备/新版草稿/资源生命周期等异步回归通过。

Qt Release完整构建通过，cbindgen生成头严格字节检查通过。C/C++静态/动态ABI smoke四项通过：新DTO大小/偏移/能力，以及从四种链接调用新符号并验证空指针失败清零输出。cargo fmt、clippy及diff检查通过。

本阶段未添加Qt模式下拉、工具偏好或有色底模式预览，当前用户界面仍正常绘画。没有全量tools/check.py、Qt新功能行为、实体设备/跨平台/整体性能或PS逐像素验收；完整用户入口下一阶段接入。双色与自动描边/平滑入口/湿混及ABR完整映射仍待后续。
