# 2026-10-20 分项修复验收

对应[问题清单与设计](../../../planning/2026-10-20-bugfix-design.md)。在 master 按项提交并推送；以下只记录实际完成的项。

## 严重 1：自由变换退出

- 修复 Qt 漏发 active 关闭与 busy 结束通知；空图层开始失败后也恢复界面。
- 回归：qt_transform_tests::emptyTransformAndCancelPublishClosedState，覆盖空图层、取消、实际 Escape、QML 覆盖层和历史不增长。
- Windows Release 输出：build/qt/bin/Release。
- 不代表全量、跨平台或实体连续拖动验收。

## 普通 1：初始背景乱码

- 白色新文档的背景名称从错误编码字面量恢复为“背景”。不改用户已有文档名称。
- paint-core masks 的 initial_white_background_has_readable_name_without_allocating_pixels 通过；验证 4096×2160 名称、白底、零瓦片和零修订。
- Release 随本批验收构建更新到指定目录。
