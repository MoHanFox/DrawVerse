# 整笔描边 Qt 验收

2026-10-19 Windows Release，对应[设计](../../../brush/ui/brush-stroke-outline-ui-design.md)。

`cmake --build build --config Release --target qt_ui_tests`通过；ABI生成头一致。Qt专项三项通过（`artifacts/outline-qt.txt`）：

- state41完整关闭值、复制基线、重置、纯参数包往返；未知字段与舍入前超界原子拒绝；已有笔刷快照不随源删除改变。
- 真实Main描边分类、位置回写、白色异步预览、红笔蓝描边像素、单历史撤销重做、VBR新目录像素一致、关闭描边后保存重开。无QML警告。
- 已有图片笔刷与纹理两类来源从VBR进入全新资源目录，真实白色预览与画布像素一致；脱离主笔尖引用后仍完整导出来源图片。

Qt同时要求`STROKE_OUTLINE`、`BRUSH_GROUP_STATE`和`BRUSH_GROUP_PACKAGE`能力。原生分组入口保留完整字段，专用预览采用描边入口。没有改变ABI/state/VBR版本。未运行全量check、实体输入、跨平台或GPU；核心不支持的湿混/蒙版等组合仍明确拒绝。
