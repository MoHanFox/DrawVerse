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

## 普通 18：变换交叉/退化拖动

- Qt 仅静默处理原生校验返回的交叉/退化 UPDATE，不采用无效控制点，其他错误保留。
- Windows Release Qt：invalidIntermediateTransformKeepsLastGeometryAndRecovers 通过；退化、交叉两种输入后控制点不变且无错误通知，后续有效移动和取消正常。
- 同跑 emptyTransformAndCancelPublishClosedState 通过，保留会话期间修订通知。

## 普通 20：微小套索

- 有限的重复/共线路径在 Qt 提交前成为无操作，保留既有选区、修订与历史，不弹错误。
- Windows Release Qt：degenerateLassoIsANoopAndSelfIntersectionStillWorks 通过，覆盖自由/多边形套索各三种退化路径，并验证零有向面积的自交套索仍可正常提交。
