# 画笔库交互修复验收

2026-10-18，Windows，对应[设计](brush-library-interaction-fixes-design.md)。

- 使用用户提供的 `未标题画笔.abr` 临时导入验证：该画笔带纹理 `minimumDepth`，修复前预览为空并报告“预览暂不可用”，修复后预览生成成功。诊断文件未加入仓库。
- `brushTexturePreviewStateVersionIncludesDepthFields` 通过：普通纹理为 11，深度字段为 21，advanced 模式为 30，latest 模式为 31。
- `brushGroupsAppearInLibraryTreeAndSearchKeepsParent` 通过：点击组行可收起/展开，搜索保留父组，创建额外组并移动画笔后列表滚动位置保持。
- `brushAndLayerListsWheelScrollWhileDragging` 通过：画笔库与图层列表在拖拽状态接收滚轮并改变 `contentY`，拖拽坐标靠近上缘时自动向上滚动。
- Windows Release 完整构建成功，正式 `drawverse.exe` 与 Qt 测试目标均重新链接；没有运行全量测试、跨平台、实体设备或 GPU 验收。
