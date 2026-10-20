# 图层元数据更新时保留列表行

2026-10-21。移动改变位置、蒙版链接改变属性后，QVariantList 重新赋给 ListView 会销毁可见行。链条 Canvas 重建前短暂为空，busy 又改变透明度；缩略图重复释放与申请也增加交互工作。

LayersPanel 用 ID 对齐 ListModel：移动现有行、插入新行、删除消失行，只替换 entry 属性。搜索与折叠仍决定可见集合，蒙版保持内嵌；改名、滚动与缩略图对象不因同 ID 的属性更新重建。内嵌蒙版仅在 ID 改变时转移预览引用。链接按钮 busy 时禁止重复点击但保留显示亮度。

该调整消除 UI 重建，不扩大核心常驻移动源的支持范围；组内单层、固定蒙版等回退仍需单独优化。没有 ABI、文件格式或 GPU 算子变化。

验收见 [记录](../validations/layers/layer-row-residency-validation.md)。
