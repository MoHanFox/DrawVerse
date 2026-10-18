# 空图层栈验收

2026-10-16，Windows。对应[设计](empty-layer-stack-design.md)。允许单层、整组及批量删除至零图层，active_layer=0，透明合成与既有棋盘格显示，无补建图层。组内最后像素层删除后可以仅保留空组；锁及事务历史原有校验保留。无目标起笔明确返回 LayerNotFound(0)，不再触发活动层断言；新建根图层及 target=0 的空栈粘贴接通，非空栈仍要求有效粘贴目标。

ORA 空 stack 可保存重开且不补层，活动索引避免下溢；普通空组也可往返，透明 PNG 导出保持。未增加 ABI 布局或文件格式版本；保留旧 LastLayer 错误枚举以避免无关接口删除，生成头仅同步文档注释。

最小相关验证通过：

- paint-core integration：document 28 项、groups 7 项、layer_actions 3 项，共 38 项通过。替换旧“最后图层禁止删除”断言，新增空栈无目标绘画/透明像素、undo/redo/new、组内最后子层、全根批量删除与复制组粘贴到空文档，原锁、分组、分页历史及合并回归保持。
- paint-io files 的 `empty_and_group_only_ora_stacks_roundtrip_without_inventing_layers` 通过：空栈和仅组的尺寸/活动层/透明像素/ORA 及 PNG 往返。
- 三相关 crate 全目标 Clippy（-D warnings）通过；生成头重建及 Release 一致性检查通过；四项 C/C++ static/shared ABI 通过。
- Qt6.8.3 windows / software Quick / 环境倍率1：`deletingFinalLayerShowsTransparencyAndAllowsUndoPasteAndNewLayer` 与 `selectedLayersDeleteDragAndGroupChoicesAreAtomic` 通过，含初始化清理共 4 passed、0 failed。真实 Delete 删除到零、选择清空、删除按钮禁用、32×32 完整帧 alpha 全零、undo/redo、Ctrl+V 空栈粘贴、真实点击按钮删空及新建图层均通过，无 QML 警告/任务错误。

日志 `artifacts/empty-layer-{core,io,clippy,header,release,abi}.log` 及 `artifacts/empty-layer-qt.txt` 为本机产物，不入库。未运行无关全量或跨平台验收。同步前发现 DrawVerse 已运行，因此没有覆盖运行中的开发版；新构建位于 build/verify-ui/bin/Release。
