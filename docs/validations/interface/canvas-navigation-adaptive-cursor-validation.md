# 平滑导航与自适应轮廓最小验收

日期：2026-10-19；分支 `feat/wgpu-support`。设计见 [导航/轮廓](../../interface/canvas-navigation-adaptive-cursor-design.md)，接入规则与代码示例见 [GPU 加速](../../platform/gpu-acceleration-design.md)。

## 环境与改动

Windows 11，Qt 6.8.3，RTX 4060；Qt 显示使用 RHI/D3D11，wgpu 自动设备配置保持。仅修改 Qt/C++ 显示与导航、QML、shader 与文档；Rust 算子、C ABI 1.62、VBR 与文档格式未变化。

- 平滑缩放约 140ms，对数倍率、三次缓出，连续输入累积目标；拖动直接跟手、松手指数减速，再按下/绘画/隐藏/失活停止。
- 128 屏幕像素周边预取，最多 360 像素方向前瞻；view1 默认 512 长边整图概览，复用已有导航器帧。四条带仅填充高清 ROI 外部，保留透明度与旧帧。
- 圆形、实际图片轮廓和液化轮廓共用局部 GPU 黑白自适应 shader；取消亮度和自定义颜色控件。软件显示黑白线回退。既有副笔尖几何接入共用 alpha 源，但不恢复已退役的双色溢出功能。
- 巨大椭圆完全包住窗口且边界不在窗口时，跳过背景捕获。资源以像素/边长和窗口裁剪限额约束。
- 纹理替换先移除引用节点再释放纹理，新增节点先设有效纹理再挂入树，兼容硬件与软件渲染器的同步节点通知。

## 最终结果

| 检查 | 结果与覆盖 |
| --- | --- |
| RHI/D3D11 Qt 最小 5 项 | 5 业务用例全部通过（含初始化/清理共 7 PASS），约 6.70s |
| 软件 Qt 最小 6 项 | 6 业务用例全部通过（含初始化/清理共 8 PASS），约 6.82s |
| C/C++ 静态/动态 ABI | 四项通过 |
| 完整 Windows Release | 通过，包含 Qt 部署、许可与 shader 烘焙 |
| 生成头文件一致性 | Release 构建中的 `paint-api-gen --check` 通过 |
| 补充文档链接/补丁空白检查 | 通过 |

硬件用例：

```text
qt_ui_tests canvasNavigationAnimatesAnchorsAndInertiaWithoutHistory
  overviewKeepsTransparentPixelsAndCoversFastPan
  adaptiveCursorContrastsBothSidesAndTracksLiquifyPreview
  cursorLegacyBrightnessIsIgnoredByAdaptiveDisplay
  liquifyResidentMeshRendersAndCommitsWithoutCpuFrames
```

使用 `QT_QPA_PLATFORM=windows; QT_QUICK_BACKEND=rhi; QSG_RHI_BACKEND=d3d11`。真实像素断言同一轮廓在黑侧为白、白侧为黑；验证普通圆形、带孔图片笔尖、灰度双笔刷不产生退役溢出光标、液化网格、15000px 不可见轮廓捕获跳过及隐藏，不写绘画历史。显示用局部 GPU 纹理；只有验收抓图会将图像读回 CPU。

2000/15000px 液化用例分别约 73/71ms，包括 50ms 测试轮询与异步抓帧；不能当作实际连续帧延迟。会话源身份复用、撤销重做、取消和完成物化均通过。

软件用例：

```text
qt_ui_tests canvasNavigationAnimatesAnchorsAndInertiaWithoutHistory
  overviewKeepsTransparentPixelsAndCoversFastPan
  cursorLegacyBrightnessIsIgnoredByAdaptiveDisplay
  maximumLiquifySizePublishesPixelsAndKeepsLocalAndDocumentHistory
  compactStatusReflectsDocumentAndZoomControlsPreserveCenter
  navigatorTracksVisibleDocumentAndSharesContentAcrossWindows
```

使用 `QT_QPA_PLATFORM=windows; QT_QUICK_BACKEND=software`。覆盖动画中间帧与最终锚点、连续缩放/缩小、惯性中断、预取/概览限额、透明快速平移无重复 over、设置移除、软件液化与导航器跨窗口。导航器断言等待缩放和松手惯性结束后再保存视图，适应新的交互时序。

命令中的换行只用于排版，实际工具参数为同一命令行。日志位于本机忽略目录 `artifacts/gpu-navigation-rhi-tests.txt`、`artifacts/gpu-navigation-software-tests.txt`、`artifacts/gpu-interaction-ui-build.txt`，真实 GPU 图例为 `artifacts/adaptive-cursor.png`。

## 构建产物与边界

当前 Release 为 `build/verify-ui/bin/Release-wgpu-navigation/drawverse.exe`。既有 `Release-wgpu` 程序运行中并锁定 EXE，构建采用新目录，未终止用户进程。系统盘空间不足，测试进程 TEMP/TMP 使用仓库忽略的临时目录，没有改全局配置。

扩展检查 `canvasZoomKeysAndGlobalColorsWorkAcrossDocumentsAndFloatingWindows` 在颜色同步断言失败：主文档 `#ff2ea99d`、另一文档 `#80336699`。用改动前 `Release-wgpu/qt_ui_tests.exe` 和新产物分别独立运行，均得到相同失败，属于既有问题。本轮不修改该颜色同步行为，也不宣称全量 UI 通过。其缩放部分之前已通过，独立的导航/状态/导航器用例最终全部通过。

未运行全量 `tools/check.py`、跨平台或实体输入测试；连续滚轮/数位板手感、实际 GPU 占用、多硬件与极端窗口尺寸仍待实机复核。普通栅格图层合成与绘画算法仍使用既有 CPU 路径；不把 Qt 纹理显示写成所有功能已完成 wgpu 加速。
