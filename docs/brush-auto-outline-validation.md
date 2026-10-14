# 自动描边预设验收

2026-10-15，Windows/MSVC/Qt6.8.3 Release，现有ABI1.48/state16/VBR1.14。只运行相关最小回归，未运行完整tools/check.py。

- outlineTemplatesAreAtomicIndependentResettableAndBounded通过：两个模板完整定义/唯一用户UUID/一次发布，原自定义参数保持、工具模式及光标不变，关闭编辑后重开恢复完整基准、VBR导入基准、1024上限无任何状态变化。
- outlineButtonsPressureCaptureCrossingsExchangeAndHistory在DPR1/2均通过：真实点击两个按钮、真实引擎预览；压力0.5时主半径6px、固定第二半径10px与跟随8px的实际像素边界；起笔后切换另一模板仍保持已捕获固定宽度；单条历史/真实undo与redo；交叉回笔内部只有主色、40%整笔不透明度最高103/255；导出VBR后独立客户端整幅RGBA一致；保存关闭重开完整resetState恢复。
- DPR2另跑双色入口与图片双笔刷/来源页重置两项回归通过。日志artifacts/brush-auto-outline-{1,2}.txt；Release构建成功，git diff --check通过。

第一次UI点击在分类布局更新之前未触发，测试等待按钮几何及下一帧后正常触发；没有用直接调用替代按钮点击。无Rust算法/ABI/格式变化，本轮不重复全内核矩阵。复合光标仍仅显示主笔尖，非分隔绘制、收尖、湿混与高级ABR等继续后续。
