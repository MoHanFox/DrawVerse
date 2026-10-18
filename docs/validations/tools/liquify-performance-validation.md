# 液化性能最小验收

2026-10-12，ABI 1.21 不变。Release 构建与生成头严格检查通过，paint-core / paint-ffi 的 all-targets Clippy（-D warnings）通过。

相关测试：

- paint-core `--test liquify`：9 个功能回归 + 1 个手动 Release 性能测例通过，覆盖历史/压力/选区/蒙版/锁/失败回滚、固定像素校验及画布外无变化输入。
- paint-core `--lib coverage_cache`：1 个缓存回归通过，验证整数与像素中心覆盖互不混用、边界/上限和身份失效。
- paint-ffi：1 个 actor 屏障回归、2 个 async_session 液化回归通过；连续 dab 保留 render floor，确认（即使未再增加像素版本）、重置和取消提高屏障。
- Qt 原生 Windows、100% 缩放、software：液化、涂抹、油漆桶、魔棒各 1 个相关交互用例通过。液化验证标题分隔宽 1px、共用复选框指示器 13×13px、鼠标/数位笔、专注与会话控制。魔棒原有信号隐式参数警告改为显式参数后，仅重跑魔棒并通过，无警告。

性能测例为 512×512 文档、直径 256px、24 次混合推拉/膨大/扭曲输入，仅计核心变形耗时，不包含合成、FFI 或显示。优化前 263.7637ms；中间版本 154.703ms / 143.2139ms；最终版本 135.7482ms，约减少 49%。最终像素 fingerprint 同为 5537713511484391265。此为同机单场景测量，不推断所有尺寸或实时帧率。性能测试不设置脆弱的固定时间门槛。

日志在 `artifacts/liquify-performance-{before,after,selection,ffi,clippy,build}.txt`、`artifacts/liquify-performance-native-1.txt`、`artifacts/liquify-performance-wand-1.txt`。

核心复现：`cargo test --release -p paint-core --test liquify -- --include-ignored --nocapture`（core 工作目录）。Qt 运行 `qt_ui_tests.exe liquifyCanvasTransactionOptionsAndDocumentLock smudgeSharedBrushParametersInputAndDelayedHelp bucketOptionsMouseTabletAndFloatingCanvasFillOnce wandOptionsSelectRealMaskOnceAcrossMouseTabletAndFloatingCanvas`，设置 Windows 平台、software 后端、缩放 1。

未运行完整 tools/check.py。输入仍逐条保留 FIFO，不改变累计路径；整帧传输、每次输入的事务索引克隆及 GPU/近似预览仍是后续优化方向。
