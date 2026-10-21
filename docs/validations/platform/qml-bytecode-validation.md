# 打包 QML 字节码验收

2026-10-22，Windows / Qt 6.8.3 MSVC。设计见 [打包 QML 字节码](../../platform/qml-bytecode-design.md)。仅更改构建、资源初始化与相应验收；ABI1.79/state45/behavior37/VBR1.0保持，没有新增GPU计算。

## 缓存实际生效

首次候选只生成缓存，静态库未保留注册对象，运行时 Theme URL 缓存查找失败；不是通过构建就宣称生效。统一 UiResources 初始化后，新增验收遍历62个打包QML，全部具有当前Qt兼容的内置编译单元，且资源文本与对应源码逐字节一致。源码和图片的原资源URL保持，Theme单例与相对目录导入正常。最初禁用生成根qmldir导致构建依赖缺失，已保留Qt生成的根描述；中间OLD策略候选有弃用诊断，最终改用NO_GENERATE_EXTRA_QMLDIRS，没有屏蔽警告。对应日志qml-bytecode-registration.txt、qml-bytecode-delivery-final.txt。

## 计时范围

新增 packagedBrushPanelMatchesFreshSourceLoading：五轮交替，从相同文本的独立临时源码目录与内置URL加载笔刷设置，每次新建引擎，核对实际大小控件、分类及参数不变且无QML警告。复制文件不计时；计时包括组件加载和对象创建，不包括窗口首次显示、GPU上传或物理冷盘。最终独立对照中位：软件内置39.385ms / 新源码813.082ms，D3D11进程33.241 / 823.173ms。早期无编译负载软件样本23.957 / 654.777ms；有并行构建的批次38.480 / 949.932ms和40.368 / 1037.555ms，不据此声称稳定帧时。日志qml-bytecode-load-{software,d3d11}.txt与工作流日志。

真实489项配置副本，D3D11本轮首轮GUI最长间隔26ms，之前源码构建样本478ms；热轮18ms、双文档16ms，部分/完整隔离工具切换21/18ms，目录重建0。软件首轮68ms、热轮57ms、双文档93ms，工具切换23/38ms，但完整隔离的跨工具选笔阶段仍有256ms间隔，目录重建0。该异常保留，不宣称P1或全部笔刷切换性能完成。两个真实探针各3行通过，日志qml-bytecode-live-{software,d3d11}.txt。首次QML编译部分已处理，其他资源/软件渲染和持续手感继续跟踪。

## 功能回归

- 缓存/源码一致性、源码对照、两次源码编辑、真实六类控件台、Live保存重载、创建多文档后语法错误恢复、欢迎准备/无文档/保存关闭/失败提示：软件最终12行、D3D11 12行通过。软件首批在Live语法错误后原子写第二版文件失败，未修改该断言；单独复测及完整最终批次通过，失败日志保留。日志qml-bytecode-workflow-software.txt、qml-bytecode-preview-retry-software.txt、qml-bytecode-workflow-final-software.txt及qml-bytecode-workflow-d3d11.txt。
- 实际分类懒创建、离屏预览取消、参数不重建目录且实际起笔、橡皮擦除与画笔独立选择、折叠分类及浮窗颜色/透明度：两后端各8行通过，qml-bytecode-ui-verified-{software,d3d11}.txt。修复测试中等待固定空指针而未重新查询delegate的问题；旧切回画笔继承橡皮选择的断言按用户独立记忆需求改为恢复原画笔，同时保留真实绘画、擦除、参数与三步历史核对。
- 变换/液化鼠标和原生笔队列结束后快捷键、变换保留浮窗：软件5行通过；D3D11含折叠Warp不透明/半透明像素对照6行通过，qml-bytecode-transform-*.txt。没有宣称实体笔冻结或驱动压感全部覆盖。
- 高DPI CTest修正两处已改名的测试引用，55个选择器全部能在真实测试程序中查到；其中刷/擦独立选择和直接历史两项在2倍缩放软件4行通过，qml-bytecode-highdpi-software.txt。未声称整个55项高DPI套件已运行。

## 交付

完整Release/生成头、Debug应用部署、四ABI消费者及diff检查通过；标准build/bin/Release/drawverse.exe更新，Debug为cmake-build-debug/bin/Debug/drawverse.exe。两应用独立D3D11启动，临时偏好下完成真实三笔绘制及截图，图像包含正确画布/控件/笔触。日志qml-bytecode-delivery-final.txt、qml-bytecode-debug-final.txt；图片qml-bytecode-{release,debug}.png。预览宿主独立构建并通过上述源码/临时配置及重载验证，没有修改Qt Creator用户文件。

未跑全测试或跨平台/其他Qt SDK/实体持续输入。构建时增加62个字节码源的首次编译工作，后续按源依赖增量更新；首建更慢，正式运行不再重复该解析工作。复杂T2、H1会话字段引用及物理冷盘、I2/I3/I6实体输入与P1剩余部分仍待。
