# 图片模型与自由变换验收

2026-10-13，ABI 1.23。实现前验收条件见 [设计](../../tools/transform-design.md)。

已完成：文件→置入 PNG/JPEG/WebP；嵌入不可变原始图片、派生显示瓦片、图层图片标记；普通图层/图片层/组/多选 Ctrl+T；八点控制、比例链接、Shift/Alt、外侧旋转与15°吸附、Ctrl扭曲/斜切/透视、翻转、X/Y/W/H/角度/水平垂直斜切、六种插值、3×3曲面网格；Enter/双击/工具切换确认、Esc取消；临时与正式撤销重做、设置步数；普通栅格层选区内像素及覆盖移动；菜单与右键栅格化、栅格化撤销；复制与ORA原图保存恢复。

## 验证证据

- `cargo test --manifest-path core/Cargo.toml -p paint-core -p paint-io -p paint-ffi --no-fail-fast -- --test-threads=2`：230项通过，0失败。原报告 `artifacts/transform-all-rust.txt`。涉及原有组默认穿透的两个旧断言已经更新：剪贴测试明确指定隔离正常组，非法混合模式改为28（27是现有穿透模式）。
- 最新事务收尾修正后 `cargo test --manifest-path core/Cargo.toml -p paint-core --test transform`：6项通过。覆盖原图缩小/放大精确恢复、局部/正式历史、栅格化与剪贴板/嵌入模型、失败与取消保留redo、插值/透视/网格/步数、恒等网格、暂存历史、组与重叠多选原子恢复、空确认无变更、选区像素和覆盖恢复。`artifacts/transform-final-targeted.txt`。
- Rust核心/IO/FFI全target Clippy `-D warnings` 通过，生成头 `paint-api-gen --check` 严格字节通过；MSVC/Qt6.8.3 Release构建成功。报告 `artifacts/transform-clippy-final.txt`、`transform-header-final.txt`、`transform-build-final.txt`。
- Windows原生软件渲染Qt最小测试：`placedImageFreeTransformRasterizeAndShortcuts` 与 `liquifyCanvasTransactionOptionsAndDocumentLock`；验证置入、实际控制点拖动、局部undo/redo、网格节点拖动、方向键、数值与斜切角历史、切换工具自动提交、Enter/Esc、栅格化历史、ORA恢复、浮动画布Ctrl+T，原有液化事务/文档锁定继续正常，零QML警告。报告 `artifacts/transform-native-1.txt`，预览 `artifacts/transform-ui-1.png`。
- C/C++ static/shared 四个 ABI consumer 通过。未运行仓库统一完整 `tools/check.py`，上述结果是本模块与所涉及层的验证，不宣称全仓库验证。

## 边界

本轮是基础自由变换：自定义3×3插值曲面，不包含PS专有变形预设、任意细分网格与切线编辑；采样器是公开CPU实现，不承诺与PS专有重采样内核逐位一致。原图是嵌入图片，外部链接与智能滤镜不在此模块。CPU一次输出上限16,777,216像素；选区覆盖沿用16MiB上限；64页原图工作缓存，逐瓦片分页输出，资源失败保留旧预览。图片像素写入必须先显式栅格化。
