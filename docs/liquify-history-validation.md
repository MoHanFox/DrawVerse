# 液化会话历史最小验收

2026-10-12，ABI 1.22。保持既有 DTO 布局，cbindgen 生成头严格检查和 Release 构建通过。

- 核心 `cargo test --release -p paint-core --test liquify`：11 个功能测试通过，1 个手动性能测例未运行。新增覆盖整条笔触、位移场恢复、重置、撤销后分支、空笔触、步数裁剪及正式历史隔离；既有像素 fingerprint 不变。
- 核心 `--lib spilled_history_failure`：1 个测试通过。低内存预算真实落盘，写入失败保留预览及待结束笔触，重试、撤销、重做和取消精确恢复。
- FFI `cargo test --release -p paint-ffi liquify`：1 个 actor、2 个异步会话测试通过，覆盖新边界/撤销/重做操作、发布深度、像素恢复、取消和单次正式提交。C/C++ static/shared ABI 冒烟测试 4 个通过。
- 核心及 FFI all-targets Clippy（`-D warnings`）通过。
- Qt 原生 Windows、缩放 100%、software：仅运行 `liquifyCanvasTransactionOptionsAndDocumentLock`。覆盖鼠标/数位笔整笔触，Ctrl+Z / Ctrl+Shift+Z，主窗和浮动画布的 [ ]，1px 边界与参数保存；液化初始条目、确定/取消的正式历史和实际列表滚动恢复。继续验证 Enter/Esc、文档锁定、专注和浮窗恢复。

测试日志为 `artifacts/liquify-history-{core,spill,ffi,abi,clippy,build}.txt`、`artifacts/liquify-history-native-1.txt`。按用户要求只做相关最小验证，未运行完整 `tools/check.py`，未重跑性能测例。
