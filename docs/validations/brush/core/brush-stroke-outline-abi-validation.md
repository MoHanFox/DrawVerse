# 整条笔触描边原生入口验收

2026-10-19 Windows。对应[原生入口设计](../../../brush/core/brush-stroke-outline-abi-design.md)。

ABI1.60、`PAINT_FEATURE_STROKE_OUTLINE`、`paint_brush_library_prepare_state_outline`及VBR 1.0描边包读写已接通。生成头包含新常数和三个导出，`cargo run -p paint-api-gen --locked -- --check`与格式检查通过。

`cargo test -p paint-ffi --lib --locked`六十六项通过，含既有准备、资源释放、冷重建、来源票据、分组和旧入口保护。`cargo test -p paint-ffi --test async_session --locked outline_public`新增两项通过：

- 启用/关闭描边预设经新准备入口真实绘制，像素与核心Document参考一致，单历史撤销重做及在途修订替换保持；旧taper准备对同一state41返回Unsupported。
- 已有图片笔尖来源经新包入口写入和读取，资源进入目标库，关闭值/来源/位置保持；普通、taper和wet-dual读取及普通写入均拒绝且不改目标。

`cargo clippy -p paint-ffi --lib --test async_session --locked -- -D warnings`通过；全部paint-ffi测试目标编译通过。未运行全量`tools/check.py`、Release/CTest ABI、Qt、跨平台或设备验收。Qt镜像仍待后续，本阶段不宣称整项描边完成。


## 2026-10-19 继续开发前复核

工作区干净，已有核心、IO 和 ABI 提交。依赖升级至 ZIP9/XML 新版后发现编译失败，已按新接口显式处理文件名解码结果、取得独立字符串并使用文本字节迭代；安全路径、重复项和预算检查保持。修复后核心描边13项、IO描边3项、ABI描边2项通过；ORA files18项、VBR image13项、IO内部7项通过，paint-io全部测试目标编译通过，生成头一致。没有运行全量验证。
