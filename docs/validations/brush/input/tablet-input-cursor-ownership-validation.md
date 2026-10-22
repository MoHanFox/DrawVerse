# 主线 B1 — Tablet Input & Cursor Ownership 验收

2026-10-23。关联 [Issue #2](https://github.com/MoHanFox/DrawVerse/issues/2) 的②及⑥输入污染；这是主线 B1，不是历史颜色专项，也没有进入 B2 方向滤波。

## 版本与环境

- 实际基线：`d89fb9ebada77f870b1433a37d4e2b4bad51c63f`；开始及收尾 fetch 后 upstream/develop 均未变化。
- 修复与测试提交：`c8c9096da2a6245f97918d5502fd1bd677116941`。后续本报告/索引提交不改变产品代码。
- 分支：`fix/issue-2-tablet-input-cursor`，从该基线新建；未沿用颜色分支。
- Windows 10 家庭中文版 19045；原有 Qt 6.8.3 MSVC x64、VS 2022 / 14.43、CMake 4.4.4、Ninja 1.12.1、Rust 1.99.1。没有升级环境或依赖。
- Native `windows` 可见 Release 窗口；实际 QRhi 为 **D3D11 / AMD Radeon(TM) Graphics**，vendor `1002`、device `1638`、flags `0`。不是仅根据环境变量判断。
- 屏幕 LQ156M1JW26，逻辑 DPI 96。TransformTests 最小窗口 DPR 1；UiTests 和正式应用沿用 `configureUiScale()` 的 1.1，正式启动日志 DPR **1.1**。没有把不同入口的倍率写成相同。
- 请求输入后端 `auto`，Qt 实际启用 `wintab`；菜单拒绝测试请求 `wintab` 后也确认实际为 `wintab`。这不等于实体数位板已验收。
- 正式 EXE SHA256：`6EBD5B0D7ADB94EC7EF170F97A4F0F3BBCCBFABA761AEF6398F187904E5ADD0C`。

## 失败证据与最小修改

前四行是在产品代码尚未修改的基线上确认；没有只凭源码猜测复现。事件位置、设备、按钮与接触状态由专项明确设置。

| 修复前序列 / 断言 | 实际失败 | 根因与修改 | 修复后 |
|---|---|---|---|
| 同窗口 A/B 两画布；A Press(150,150)，接触 Move(430,150) 越过 B | A 仍停在 (150,150)，预期 (430,150) | B 后安装的应用过滤器把 A 的接触 Move 当悬停消费；接触归属独立于悬停归属，只有已接纳画布及设备可续接 | A 收到移动；B 无历史；实际红色像素、单历史、撤销/重做和继续绘制通过 |
| 鼠标笔触进行时，无关笔设备 LeaveProximity | `client.drawing()` 变 false | 原来所有画布按 client.drawing 收尾；现在只结束本画布接纳、设备匹配的笔接触 | 鼠标及另一设备的笔触保持；所属笔离笔正常结束 |
| 笔从第一窗口直接 Hover 到第二窗口，未收到旧 Quick HoverLeave | 第一窗口 QML 轮廓仍 true | 只转移原生光标，没有结束旧悬停显示；释放归属同时发布 QML 状态，并避免恢复失效 Blank | 旧轮廓 false、旧窗口非 Blank；新窗口持有；缓存 Hover 不复活旧轮廓，鼠标可接管 |
| 实际启用 WinTab；wmpointer Press/Release 直接到原生混合模式菜单 | `activated` 为 1，预期 0 | 原过滤在 Canvas 窗口边界，菜单可先转成 mouse；两个处理入口前复用同一后端拒绝函数 | 被排除流动作 0、菜单保持；有效 WinTab 风格点击动作 1、模式 0→3、菜单关闭，无绘画历史 |

原始证据分别为 `contact-before.txt`、`ownership-before.txt`、`routing-before.txt`，修复后初次四项为 `ownership-after.txt`。均在 `artifacts/issue-2-main-b1/`。

扩展验收又发现以下缺口。它们在本轮已有部分修改的中间版本上增测，不冒充整版 d89 的重新测量；对应路径修改前的失败仍保留。`pre-space-product.patch` 保留 Space 调查开始时的产品差异。

| 追加失败序列 | 证据 | 收尾 |
|---|---|---|
| OS SendInput 按下 Space，应用内仅吞掉松键；物理已松开、指针不动 | `space-before.txt`：QML pan 在 600ms 验收等待内不恢复 | 复用原 16ms 轮询，连续两次物理松开后清除**已接纳** Space；不通过轮询获取 Space，不读取完整笔尖姿态 |
| A 拥有笔悬停，B 的应用过滤器先消费 Space | `space-owner-before.txt`：窗口 Blank，预期 OpenHand | 有笔悬停归属时，非所属画布让出 Space；即时检查窗口光标与轮廓，不靠后续指针移动 |
| 接纳笔接触后隐藏画布 | `hidden-contact-before.txt`：drawing 仍 true | 隐藏、禁用、取消交互、换窗口/客户端及销毁复用已有取消路径，释放接触归属；隐藏/销毁专项均不新增历史，后继画布可继续绘制 |
| 销毁替代画布，再在同进程建立下一窗口 | `lifecycle-pair-console.txt`：计时事件中 access violation | QQuickItem 基类析构仍发出一次 windowChanged，回调可访问已析构的成员计时器；派生析构入口断开自身信号到自身回调。`lifecycle-reentry-pair.txt` 及最终组合通过 |

生命周期测试也检查了原生光标的旧 Blank 恢复：它可能来自同一画布先前的鼠标轮廓，失去悬停后不能继续恢复该隐藏值；普通 Wait 光标的原有恢复仍由既有专项确认。

早期夹具与开发中失败未删除：最初双客户端夹具未采用运行时共享目录连接，未到达移动断言；最终改用实际共享连接。伪造 UngrabMouse 不会调用 Quick 的失抓回调，事件投递外 grabMouse 也被 Qt 拒绝；最终使用真实 Quick 鼠标抓取再 `ungrabMouse()`。连续窗口测试显式提供 Enter，因为激活窗口并不意味着鼠标已进入；原来的快捷键断言没有放宽。上述夹具失败不计作学生驱动缺陷。

## 最终专项结果

| 入口 / 最终日志 | 功能函数 | 实际功能数据行 | init/cleanup | 结果 |
|---|---:|---:|---:|---|
| qt_transform_tests / `transform-passed.txt` | 14 | 15 | 2 | 17 passed，0 failed，0 skipped |
| qt_ui_tests / `ui-passed.txt` | 7 | 10 | 2 | 12 passed，0 failed，0 skipped |
| qt_color_state_tests / `color-passed.txt` | 2 | 2 | 2 | 4 passed，0 failed，0 skipped |
| **合计** | **23** | **27** | **6（不计功能用例）** | **27 个功能数据行通过** |

具体核对：

- 静止 Ctrl/Alt/Space 的 item/窗口光标和 QML 轮廓；缺 modifier 移动、丢失松键；文本框实际得到 `a b`，没有被画布抢走 Space。
- 仓库 CompactCheckBox：Press 后 8 次按住移动仍为 0 动作，Release 后 1 次；再连续两次合法点击累计 3 次；右侧键不当作左键。真实 CompactSlider 可拖动。输入使用 Qt WindowSystemInterface，再走 Qt 事件与正常控件合成路径。
- 实际 QML 原生/兼容混合模式菜单，鼠标/笔各一行：选择、关闭、菜单外关闭不穿透绘画；下一次独立落笔正常。另复用真实菜单跨浮窗打开/关闭专项。
- 多画布接触归属、另一设备离笔、重复 Press/Release、异常离笔（画笔与液化）、WindowDeactivate、Quick 失抓、隐藏与销毁后继续绘制；只保留应有历史。
- 既有 Windows Ink 缓存 Hover/真实帧、伴随鼠标接触隔离、真实鼠标接管、压力/倾斜/旋转与在途姿态专项通过。没有改采样字段、方向阈值、滤波或 Rust 算法。
- 连续 24 笔的最新帧与实际显示像素，液化/变换取消后第 25 笔仍发布；保持“画布停更但能保存”修复的专项。
- 颜色路径：QML 色块立即更新，以及防抖未到期前绘画的真实像素、保存及重新打开；既有吸色环鼠标/笔拖动也通过。

未跑全量 check.py、A 性能矩阵、软件后端矩阵或无关旧失败；无 CI、依赖、ABI、配置格式变更。增量构建内既有 cbindgen 字节检查通过。

## 复制运行

在现有英文入口的 Developer PowerShell 中，沿用原 Qt/CMake/Ninja PATH。以下创建新的证据目录，不清空旧数据：

```powershell
Set-Location E:\Git\DrawVerse-local-build
cmake --build build --target qt_transform_tests qt_ui_tests qt_color_state_tests drawverse_runtime -j 2
$run = Join-Path $PWD ('artifacts/issue-2-main-b1/repro-' + (Get-Date -Format yyyyMMdd-HHmmss-fff))
New-Item -ItemType Directory -Path $run | Out-Null
$env:PATH = (Join-Path $PWD '.tools/qt/6.8.3/msvc2022_64/bin') + ';' + $env:PATH
$env:QT_QPA_PLATFORM='windows'; $env:QT_QUICK_BACKEND=$null
$env:QSG_RHI_BACKEND='d3d11'; $env:QSG_INFO='1'; $env:QSG_RHI_DISABLE_DISK_CACHE='1'
$env:QML_DISK_CACHE_PATH=Join-Path $run 'qml-cache'
$env:TEMP=Join-Path $run 'temp'; $env:TMP=$env:TEMP
New-Item -ItemType Directory -Path $env:TEMP | Out-Null
$cases=@('tabletContactMoveBelongsToPressedCanvas','proximityLossDoesNotEndMouseOrOtherPenStroke',
 'tabletWindowHandoffClearsPreviousOutline','selectedBackendAlsoFiltersNativeTabletMenu',
 'ordinaryTabletControlsAndTextFocusRemainNormal','missingSpaceReleaseRecoversWithoutPointerMovement',
 'tabletFocusAndMouseUngrabCancelOnlyAcceptedStroke','shortcutKeysUpdateLikeSpaceWithoutPointerMovement',
 'physicalShortcutsUpdateQmlWithoutPointerOrKeyEvents','shortcutCursorSurvivesModifierlessTabletMoves',
 'duplicateTabletPressStillEndsOnRelease','duplicateMousePressStillEndsOnRelease',
 'tabletProximityLossEndsStrokeAndEditingRemainsUsable','continuousStrokesKeepPublishingFramesAndAllowLaterEdits')
& ./build/bin/Release/qt_transform_tests.exe @cases -o "$run/transform.txt,txt"
& ./build/bin/Release/qt_ui_tests.exe tabletDirectHoverEntryAndWindowCursorRestore tabletHoverTracksOutlineThroughWindowAndItemDelivery windowsInkTabletPositionSurvivesCachedQuickHoverAndFrames liveTabletPoseTracksTiltRotationPressureAndCapturedStroke toolComboMouseAndTabletOwnerWindowClicksNeverPaintThrough menusReopenWithoutGhostsAndFloatBesidePanels eyedropperRingComparesColorsDuringMouseAndTabletDrag -o "$run/ui.txt,txt"
& ./build/bin/Release/qt_color_state_tests.exe qmlSwatchesUpdateImmediately beforeDebouncePixelsSaveAndReopen -o "$run/color.txt,txt"
```

按日志中的实际结果判断成功；QTRY 只等待状态/回执，不是交互延迟指标。没有新增通用追踪或性能框架，也没有把截图、等待或进程时长作为性能结果。

## 可运行交付与学生清单

完整部署目录：`E:\Git\DrawVerse-local-build\artifacts\issue-2-main-b1\runtime`，对应修复提交 c8c9096，包含 Qt DLL、platforms、qml、LICENSE 与启动脚本，约 115 MiB。保留原部署附带的官方 VC 运行库安装器，但本轮未执行安装。交付时复制**整个目录**；不是单独 EXE。

```powershell
# 包内入口；每次建立新的 sessions/student-* 隔离配置，不覆盖原用户数据。
powershell -NoProfile -ExecutionPolicy Bypass -File E:\Git\DrawVerse-local-build\artifacts\issue-2-main-b1\runtime\Start-DrawVerse.ps1 -TabletApi auto
# 仓库入口，也可显式选择 wintab 或 windows-ink；实际值以 stderr.log 为准。
powershell -NoProfile -ExecutionPolicy Bypass -File tools/run-tablet-input-check.ps1 -TabletApi auto
```

已从完整复制目录启动：移除 Qt SDK PATH 后，以现有 `--preview` 接口绘制三笔、生成 `runtime-preview.png` 并正常退出；另以学生脚本打开普通窗口，经原生 CloseMainWindow 正常关闭。`launcher-smoke-2/stderr.log` 记录请求 auto、实际 wintab、D3D11 和 AMD 硬件；`binary.json` 可核对 EXE。最初 GUI stderr 未强制输出、以及启动探针读取正在独占写入的日志等夹具问题保留在早期日志，不计入功能通过。首次脚本启动遇到本机 Windows PowerShell 的 Get-FileHash 不可用，已改用 .NET SHA256；该失败与产品行为无关。

`--profile-dir` 将输入偏好、笔刷资源、文档客户端及工作区统一隔离；脚本将临时文件和缓存指向同一新会话，退出后恢复进程环境。本机默认路径的 DrawVerse.ini 启动前不存在、启动后仍未创建（`user-settings-before.json` / `user-settings-after.json` 的哈希均为 null），这不是已有文件的哈希校验。没有写原笔刷资源或绘画文件。

人工验收，当前全部为**实体设备待验收**：

1. 记录板型号、驱动版本、请求/实际输入后端、Windows 缩放及屏幕；保存新会话的启动日志。
2. 新建测试画布，悬停不动依次按/松 Ctrl、Alt、Space；检查轮廓与 Windows 光标是否残留或肉眼闪烁。
3. 笔点击普通开关、混合模式及主菜单，再选择、菜单外点击或 Escape 关闭；一次有效点击只发生一次动作。
4. 在画布、浮动面板/画布和弹窗间往返；以鼠标接管再回到笔，检查位置、轮廓和光标归属。
5. 连续绘制、离笔再回来、重复按下/放开与切换窗口；核对一笔一历史、撤销/重做及继续绘画。只保存到会话目录。
6. 观察固定方向、慢移、近静止位移的残余方向抖动；记录是否仍出现，不把本轮状态修复写成方向稳定算法已经完成。

本轮没有为证明代码问题新增真实逐事件采集器，也没有系统键盘监听。学生脚本的启动元数据是按需入口，关闭应用即停止；不记录普通文本输入，不自动上传。正式普通启动不启用这些图形诊断环境变量，原 InteractionTrace 默认仍关闭。

## 结论与未覆盖

**自动化确认的修复**：排除后端不再绕过菜单过滤；接触归属不被其他画布抢占；无关设备离笔不结束鼠标/另一笔笔触；窗口交接和生命周期退出释放显示状态；Space 漏松键可恢复，非所属画布不抢其显示更新；销毁时不再重启已析构计时器。

**仍需学生实机确认的表现**：Windows 光标肉眼无闪烁、真实驱动是否重复发动作、未标明来源的 OS 伴生鼠标包是否仍污染悬停。Qt 合成设备即使命名为 wintab/wmpointer 也不是实体认证。OS 原生注入仅用于键盘 SendInput，没有进行原生笔包注入。

没有学生真实笔刷、实体数位板连续压力/倾斜/翻面数据，未测输入到显示延迟；不声称学生全部卡顿或⑥已修复。有效微小位移的方向更新保持原样，剩余真实方向噪声候选交给 B2 实采后判断。本阶段止于代码、专项与实体复测交付，不创建 PR、不合并、不关闭 Issue。
