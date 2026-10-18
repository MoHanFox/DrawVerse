# VBR 开发迭代测试快照

这里保存开发期间的 43 个 schema 快照，按 vbr、dynamics、dual、texture、wet、abr 分类。文件名、`$id`、minor 常量和字段范围保留当时含义，已有回归测试用它们确认旧数值范围、模型和来源记录；它们不是当前产品格式契约。

当前产品统一为 VBR 1.0，契约入口为 [contracts](../../../../../../contracts/README.md)。读取按实际字段补全和执行，旧快照中的 minor 不能作为当前功能门槛。新增功能应更新当前契约和运行时测试，不应继续在此新增迭代版本 schema。
