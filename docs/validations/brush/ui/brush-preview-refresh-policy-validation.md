# 笔刷预览刷新规则验收

2026-10-26，Windows / Qt6.8.3 / Release。设计与免刷新清单见docs/brush/ui/brush-preview-refresh-policy-design.md。

实现使用独立BrushPreviewPolicy具名规则表：关闭模块、压感、笔尖、种子、副笔尖、描边宽度与来源、标准样本各自管理。BrushLibrary setter只保存真实参数，统一changed入口比较有效预览参数；请求、原生恢复及跨工具缓存复用也使用该参数。真实执行和保存保留关闭参数，无Rust/ABI/state变化。

定向范围：16类关闭模块保留缓存与在途回包、持久化恢复关闭值及启用后失效；压感曲线、倾斜圆度最低值、喷枪、固定三值/颜色/光标/混合、零抖动种子、备用描边宽度、主副图片硬度；种子在强度非零时仍刷新；隔离工具共享等效预览、错误状态保留与旧任务拒绝；真实引擎强制重生成与保留图像相同，启用后像素不同；既有标准预览及五类白色/alpha/文档隔离回归。

首轮模型18通过/2失败均为新增持久化断言将Qt Invalid与JSON null视为不同，资源值实际均为空；改用QJsonObject比较保存语义，未更改生产校验或免刷新断言。记录artifacts/preview-policy-model-software.txt。

最终六个相关功能组（16类数据行）软件与D3D11各23 passed、0 failed，含初始化/结束；artifacts/preview-policy-final-{software,d3d11}.txt。真实引擎检查确认修改关闭参数后不新增请求，强制重新生成的PNG与原图完全一致，开启形状动态后实际PNG不同。跨工具隔离和错误状态测试确认有效参数相同时不丢缓存/错误、不拒绝正确在途回包；变化后拒绝旧token。

默认笔尖规范化不再借无效的最低圆度字段触发设备能力；预览显式保留启用倾斜/旋转动态所需的模拟样本。对应两项原有生成预览/保存重开测试软件4 passed（含初始化/结束），artifacts/preview-policy-active-device.txt。

Release应用/测试与生成头检查通过，build/bin/Release/drawverse.exe于2026-10-26 22:21:12更新，最终构建日志artifacts/preview-policy-delivery-build.log；diff检查通过。没有运行全套或Rust/四ABI构建（此次无Rust/ABI变化）。没有新增实体笔输入或性能计时结论。

## 2026-10-27 压感开关免刷新

统一规则表以standard-pressure替代原先仅关闭时忽略曲线的规则。主笔尖、副笔尖和描边来源笔尖的标准预览固定开启大小/流量线性压感，不使用不透明度压感或用户曲线/最低值；真实笔刷和保存数据仍使用用户值。缓存键与实际预览请求同时规范化，避免重新生成后出现不同图像。

按用户要求仅做最小验证：三项开关及曲线修改保留在途token和已生成图像，保存重开保留真实开关/曲线；既有条件规则回归；真实引擎修改后强制生成PNG完全一致；既有压感编辑器/导出导入及实际笔触alpha验证通过。四个功能在软件与D3D11各6 passed（含初始化/结束）、0 failed，日志artifacts/preview-pressure-{software,d3d11}.txt。未新增实体笔验收或性能计时。

Release应用/测试构建及生成头检查通过，日志artifacts/preview-pressure-build.log；标准build/bin/Release/drawverse.exe更新于2026-10-27 01:00:08。无Rust源码/ABI1.80/state45变化，未跑无关全套。

## 2026-10-28 滑条最小回归

brushSliderPreviewWaitsForRelease 与 brushSettingsCreatesOnlyActiveCategoryAndLabelsChoices（新增真实滑条按住 200ms/松开检查）软件及 D3D11 各两项通过。参数实时改变、拖动保留图片且零请求、释放一次请求及无变化不重绘。旧 brushPresetsKeepIndependentSettingsAndRejectStalePreviews 半径预期48/实际24断言失败保留于 build/bugfix-ui-software.txt，未修改该断言，不报告全套通过。
