# 画笔混合Qt入口验收

2026-10-14，ABI1.44入口（本轮最终构建ABI1.45含平滑曲线）。画笔27模式真实写入目标层；工具状态brushTool独立持久化、文档共享及写入者移交，预设/完整重置基准/VBR不保存临时模式。擦除和涂抹显式正常，不丢画笔选择。非正常预览在同一目标层生成有色底，排队捕获模式，改变模式使全部旧token失效。

最小Qt回归：brushWriteBlendIsStrictGlobalToolStateOutsidePresets、brushBlendToolbarPreviewCaptureHistoryAndSharedPersistence，在DPR1/2通过；实际菜单选择、Multiply/Screen真实预览及像素、修改模式的在途捕获/下一笔/历史、预览不改主文档、擦除正常/菜单可见性、多文档及写入者移交、最后新值26真实后台保存和重开、严格非法及旧状态/token验证。双笔刷、曲线、重置与涂抹相关DPR2回归通过。Release Qt与生成头检查通过，四个C/C++静态/动态ABI smoke通过。

仅对应阶段验证，未运行tools/check.py全仓；双色贡献/自动描边、平滑路径IO/Qt、湿混、ABR完整动态映射继续后续。
