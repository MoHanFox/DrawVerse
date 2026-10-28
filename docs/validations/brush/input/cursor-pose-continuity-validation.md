# 主线 B2 — Cursor Pose Continuity & Direction Diagnosis 验收

2026-10-23。关联 [Issue #2](https://github.com/MoHanFox/DrawVerse/issues/2) 的⑥；没有进入主线 C，没有方向滤波或通用性能优化。

## B1 合并与本轮版本

- B1 完整差异核查：`d846e8a03c63a590defc314c792fc2e6be3d6cfd`，产品/测试仍为 `c8c9096`，之后仅文档。复用 23 个功能函数、27 个功能数据行证据，阶段一未重新构建 B1 或重跑整个 B1 组合；阶段二仅为下述两项扩展失败建立独立基线对照。
- [B1 PR #5](https://github.com/MoHanFox/DrawVerse/pull/5) 普通 merge commit 已合并，准确 head SHA 校验；远端确认 MERGED，无冲突、阻塞审查或必需检查，无管理员绕过。
- 合并及 B2 实际基线：`a844abe8797e286af36d429c2533b10496eace22`。fetch 后确认包含 B1 HEAD，才创建 `fix/issue-2-cursor-pose-continuity`。
- B2 修复/测试提交：`f2fb70486b0a41ba2fb9d0ae041fe83ac219fb6a`；本报告及索引另作收尾提交，不改变产品。未沿用已合并分支开发。
- Windows 10 19045；原 Qt 6.8.3 MSVC x64、MSVC 14.43、CMake 4.4.4、Ninja 1.12.1、Rust 1.99.1；无环境/依赖升级。
- Native windows 可见 Release。实际 QRhi：D3D11 / AMD Radeon(TM) Graphics，1002:1638 flags0；日志有实际设备初始化。逻辑 DPI96，TransformTests DPR1，UiTests 与运行包 DPR1.1。不是仅依据环境变量判断。
- 请求 auto，启动及 TransformTests 确认实际 wintab；虚拟 Qt 设备不等于实体 WinTab 驱动认证。

## 失败序列与修复

新测试先在基线产品、仅增加测试时执行：`before.txt` 的 **5 个功能数据行均失败**，不是在修复后推测旧版行为。

| 条件与独立预期 | 旧版实际 | 修复后相同条件 |
|---|---|---|
| 已捕获旋转笔触，角度60°、圆度.35、pressure=.6、radius12；同一调用栈改下一笔的角度控制/角度17°/radius28/圆度.8 | 可见 Canvas 和实际 CanvasPane QML 都从60°回退0°；半径7.2→12 | 保持60°、圆度.35、半径7.2；下一个有效样本rotation45/pressure.8立即显示45°、半径9.6 |
| 同画布旧笔从(230,230)到(240,240)，再由另一笔首次进入(210,230) | 首位置被算为−161.565°，预期尚无方向时基础0° | 首位置0°，其后真实向下10px立即90° |
| 同序列后真实鼠标接管，再提供相应 Quick Hover | 同上−161.565° | 0°→90° |
| 同序列后 Window Leave，再由笔重新进入 | 同上−161.565° | 0°→90° |
| 同序列后同客户端建立新文档，再进入 | 同上−161.565° | 0°→90° |

原始 `captured-pose-before.json`：captured-valid / next-stroke-settings-pending 分别为60°/0°、半径7.2/12，均 visible=true、drawing=true。同栈颜色 pending 本身仍为60°，没有将它写成颜色即时显示缺陷。原 A 固定斜线角度稳定的有限观察，不能排除上述状态问题或真实微小位移问题。

根因及边界：

1. `brushCursorPose` 的全局 persistencePending 早退挡住了**已有捕获绑定**。现在只有 hover 路径等待新偏好及目录匹配；在途使用既有 m_strokePose，未加最后有效姿态缓存。关闭、未就绪、液化、未绑定和 native read 失败仍使用原基础回退；不隐藏真实错误，不新增重试机制。
2. 显示位置历史原来没有设备和文档身份，悬停退出也未清理。现在以 CanvasItem 自身、client/document generation、pointing device、笔尖 tool 和连续归属为边界；生命周期/悬停离开清理。有效接触越过画布边缘仍保留其接触方向。下一笔 Press 仍按既有语义重置方向。
3. Quick Hover 只继承同设备、同笔尖的最近笔样本几何；新笔不会借用旧笔的旋转/倾斜。新有效样本立即接管。

**必须失效与接管**：切换笔刷/尺寸/形状时，hover 原有清除、catalog source/preparation 匹配与当前 base 姿态路径保持；不显示旧笔刷来掩盖准备。捕获笔触内的参数变更属于下一笔，结束时立即转为当前17°/radius28/圆度.8，即使仍 pending。换 client/document/device、离开、隐藏、禁用、窗口交接清除显示采样历史；无样本时回当前基础姿态。没有“所有 ready=false 一直保留旧轮廓”的规则。

## 三类诊断结果

| 类别 | 本轮观察 | 结论边界 |
|---|---|---|
| A 显示状态跳变 | 上述 captured ready→pending base→captured 恢复已证明不必要，已修；hover 新设置准备仍会回当前基础姿态 | 没有证明所有 hover 准备回退均可消除；不无限保留旧轮廓 |
| B 方向输入/上下文 | 四类首位置污染已修；静止重复5次维持45°，慢移.1/.05px三次为26.565°，真实转弯立即90°；缩放1→1.5和平移后向下移动仍90° | 不等于排除真实噪声 |
| B 合成微位移 | .0001逻辑px 四向往复仍依次0°/90°/180°/−90° | 这是代码响应的合成探针，不是学生驱动噪声分布；未改变1e-9方向阈值，无新滤波参数 |
| C 表示/对称性 | (−10,+.01)/(−10,−.01) 为+179.9427°/−179.9427°；椭圆模180°差约.1146° | 数值差约359.885°不代表椭圆旋转近一圈；只适用于此对称椭圆，不外推不对称图片笔尖 |

角度、半径、圆度、可见性、位置/位移、设备/画布及 pending 记录于 `final/extended-direction/direction-diagnosis.json`（19个状态）和 `final/captured-pose.json`（6个状态）。原 InteractionTrace 仍有界200000条、默认关闭，仅补采样上下文/方向及最终几何字段；无逐事件同步写盘，测试结束统一导出。ready 与 pending/captured、相邻单调时间和调用 Scope 一起解释；单次 ready=false 不能独立证明可见抖动。没有普通文本或系统键盘记录。

## 专项结果与已有失败

| 运行日志 | 功能函数 | 功能数据行 | 结果（不含 init/cleanup） |
|---|---:|---:|---|
| final/transform.txt | 12 | 13 | 13通过，0失败，0跳过 |
| final/ui.txt | 7 | 10 | 8通过，2失败，0跳过；下述修复前对照 |
| final/color.txt | 1 | 1 | 1通过，0失败，0跳过 |
| **合计（按独立用例计数，不叠加重跑）** | **20** | **24** | **22通过，2既有失败** |

每组 init/cleanup 各2行，不计上述功能用例。新增4个功能函数/7行全部通过；`final/simple-dynamic.txt`、`final/direction-coverage.txt` 分别是补充简单/动态显示及慢移/实际平移断言后的1行定向补跑，均通过，不重复累计。其余组合的产品实现未变化。

- 新捕获用例在真实 CanvasPane QML 上作**同栈**断言；没有用 QTRY 等到暂时回退过去。QTRY 只等待真实准备、历史与帧状态，不作为延迟指标。
- 相同位置、压力、旋转与按键的有效 Press/Move/Release 序列（自然时戳不同，未启用时间动态），参考笔触与中途改下一笔参数/颜色的笔触，完整 QImage 像素逐一相等；单历史、撤销、重做后的完整帧仍相等。没有修改 tabletSample/mouseSample 或核心提交路径。
- 同设备 Quick Hover 保持80°/.25，新设备无几何时0°/.5；补充压力.5下实际/简单轮廓宽度12/24，切回实际12，pending时仍按捕获样本显示45°。
- 既有实时压力/倾斜/旋转与捕获、Windows Ink缓存Hover/帧、原生光标恢复；B1接触归属、窗口交接、Space漏松键、静止快捷键、失焦/隐藏/销毁、重复Press及画笔/液化离笔、连续24笔加后继编辑均通过。
- 颜色仅跑 `beforeDebouncePixelsSaveAndReopen`：未到保存防抖时绘画的实际新色、保存和重开通过。

两项既有失败原样保留：

- `brushCursorPreferencesAreGlobalPersistentAndNeverExported`：新文档客户端 cursorWidth=2 未传播至首客户端，实际0。
- `cursorImageQmlWhiteAlphaIndependentOutlineCapturedStrokeAndRestart`：复合第二图片的 isVisible=false。其涉及历史双重/第二轮廓路径，未放宽断言、改为skip或将其算通过。

`expanded-standalone.txt` 在B2单独运行仍为这两项失败。为区分回归，新增独立工作树 `E:\Git\DrawVerse-main-b2-baseline`，准确检出B1合并提交a844abe；独立CMake build和Cargo target，复制既有Cargo缓存后按原入口实际构建（不是替代构建契约）。`baseline-expanded.txt` 在该未修改产品/测试上也得到完全相同的两项断言失败，2 failed，0 skipped；基线配置与构建日志均保留。未扩大修复。

首轮新捕获用例最后的 .8 与 f32 返回0.800000011921精度不匹配，记录在 `after-first/ui.txt`；最终独立预期改为 `qreal(float(.8))`，未放宽此前60°、半径、QML、真实像素或方向归属断言。

没有跑全量 check.py、A矩阵、软件后端矩阵或新CI。增量构建的 cbindgen 检查通过；无依赖、锁、ABI、格式、state版本或Rust代码变更。

## 重复运行

沿用英文入口的 Developer PowerShell 和既有 Qt/CMake/Ninja PATH：

```powershell
Set-Location E:\Git\DrawVerse-local-build
cmake --build build --target qt_transform_tests qt_ui_tests qt_color_state_tests drawverse_runtime -j 2
$run=Join-Path $PWD ('artifacts/issue-2-main-b2/repro-'+(Get-Date -Format yyyyMMdd-HHmmss-fff))
New-Item -ItemType Directory -Path $run -ErrorAction Stop | Out-Null
$env:PATH=(Join-Path $PWD '.tools/qt/6.8.3/msvc2022_64/bin')+';'+$env:PATH
$env:QT_QPA_PLATFORM='windows'; $env:QT_QUICK_BACKEND=$null
$env:QSG_RHI_BACKEND='d3d11'; $env:QSG_INFO='1'; $env:QSG_RHI_DISABLE_DISK_CACHE='1'
$env:QML_DISK_CACHE_PATH=Join-Path $run 'qml-cache'
$env:TEMP=Join-Path $run 'temp'; $env:TMP=$env:TEMP
New-Item -ItemType Directory -Path $env:TEMP | Out-Null
# 仅这些专项测试显式启用有界诊断，设置输出目录；正式应用不读取此测试变量。
$env:DRAWVERSE_B2_EVIDENCE_DIR=$run
& ./build/bin/Release/qt_ui_tests.exe capturedCursorPoseSurvivesPendingPreferences cursorDirectionStartsAtOwnershipBoundary liveTabletPoseTracksTiltRotationPressureAndCapturedStroke windowsInkTabletPositionSurvivesCachedQuickHoverAndFrames tabletDirectHoverEntryAndWindowCursorRestore -o "$run/ui.txt,txt"
& ./build/bin/Release/qt_transform_tests.exe cursorDirectionTurnsRebasesAndWraps quickHoverDoesNotReuseAnotherPensGeometry tabletContactMoveBelongsToPressedCanvas tabletWindowHandoffClearsPreviousOutline missingSpaceReleaseRecoversWithoutPointerMovement tabletFocusAndMouseUngrabCancelOnlyAcceptedStroke shortcutKeysUpdateLikeSpaceWithoutPointerMovement physicalShortcutsUpdateQmlWithoutPointerOrKeyEvents shortcutCursorSurvivesModifierlessTabletMoves duplicateTabletPressStillEndsOnRelease tabletProximityLossEndsStrokeAndEditingRemainsUsable continuousStrokesKeepPublishingFramesAndAllowLaterEdits -o "$run/transform.txt,txt"
& ./build/bin/Release/qt_color_state_tests.exe beforeDebouncePixelsSaveAndReopen -o "$run/color.txt,txt"
```

已知扩展失败另用 `qt_ui_tests.exe brushCursorPreferencesAreGlobalPersistentAndNeverExported cursorImageQmlWhiteAlphaIndependentOutlineCapturedStrokeAndRestart` 复现，保留失败，不混入通过数字。Qt合成设备和事件为本轮主输入；只有复用的B1键盘用例用了OS SendInput，没有原生笔包注入、实体数位板或肉眼无闪烁验收。

## 完整运行包及学生复测

B2新目录 `E:\Git\DrawVerse-local-build\artifacts\issue-2-main-b2\runtime`，含Qt DLL、platforms/qml、LICENSE、BUILD.json和隔离脚本。EXE SHA256：`CCDFE4156004448DD439A40B75D1C51061568570B1DB8C7E6C5C73DBC724655E`。复制整个目录；没有执行附带VC安装器。移除Qt SDK PATH的preview完成三笔、截图并退出0；包内启动脚本普通可见窗口也正常关闭0，日志在runtime-preview/和launcher-smoke/。

B1包仍为 `artifacts/issue-2-main-b1/runtime`，EXE摘要仍为`6EBD5B0D7ADB94EC7EF170F97A4F0F3BBCCBFABA761AEF6398F187904E5ADD0C`，没有覆盖或删除。对照工作树 `E:\Git\DrawVerse-main-b2-baseline` 及原颜色对照工作树均保留，独立CMake/Cargo产物；仅复用Qt SDK和复制的编译缓存。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File E:\Git\DrawVerse-local-build\artifacts\issue-2-main-b2\runtime\Start-DrawVerse.ps1 -TabletApi auto
```

每次创建新的 sessions/student-*，配置/笔刷/工作区/暂存均隔离；不读写原用户笔刷与绘画文件，不自动上传。正式普通运行仍不开 InteractionTrace。本轮没为实机新增通用事件采集器，测试JSON不能冒充学生真实流。

学生清单，当前均为实体待验收：

1. 记录板型号、驱动、Windows缩放、屏幕、请求和实际输入后端；保留会话启动日志、BUILD.json。先用隔离默认笔刷，不覆盖真实资源。
2. **B1**：静止悬停按/松Ctrl/Alt/Space，普通按钮点击、菜单打开关闭、画布和浮窗往返、连续绘画/离笔回来、鼠标接管；观察Windows光标残留和肉眼闪烁。
3. **B2**：设大小24、圆度30%的非圆形轮廓，角度控制direction；分别静止、慢移、停下再移和明确转弯。再用可用的倾斜/旋转动态，切笔刷、尺寸、工具及文档，缩放/平移后移动。不要用圆笔证明角度稳定。
4. 调参/切笔刷时区分在途捕获轮廓与抬笔后的新设置。若仍跳变，录制画面及按顺序的操作，记录当时笔刷、参数、接触/悬停、是否跨窗口和输入设备；仅保存到新会话。需要学生真实近静止位移/倾斜/旋转数据后再决定是否做B2后续方向噪声处理，不能从.0001px合成探针确定滤波参数。

**结论**：自动化确认两类状态缺陷已修：在途捕获姿态被pending挡回基础、显示方向历史跨输入上下文污染。真实微小位移仍按原算法响应；真实资源、实体设备、肉眼抖动及输入到显示延迟未验证。不能宣称主线B或Issue⑥已全部关闭。仅推送B2功能分支，不创建PR、不合并，不进入主线C。
