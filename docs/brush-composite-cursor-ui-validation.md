# 可选复合光标Qt验收

2026-10-15，Windows/MSVC/Qt6.8.3 Release，消费ABI1.49，无state/VBR或新ABI变更。本轮只运行相关最小回归，未运行完整tools/check.py。

- DPR1五个用例全部通过：compositeCursorPreferenceIsOptionalStrictAndOutsidePresets、compositeCursorOptionalPressureImagesAndCapturedStroke、liveTabletPoseTracksTiltRotationPressureAndCapturedStroke、imageBrushCursorPreservesHolesTransformsAndScreenWidth、tabletHoverTracksOutlineThroughWindowAndItemDelivery。
- DPR2四个用例全部通过：复合光标、数位笔动态、图片轮廓和brushCursorPreferencesAreGlobalPersistentAndNeverExported。日志artifacts/brush-composite-cursor-ui-{1,2}.txt；Release构建及git diff --check通过。
- 默认关闭、严格布尔/旧配置默认值、目录与VBR定义不变、重置不清全局偏好、关闭重开保存；实际QML点击通用开关。
- 实际原生姿态：主24px/第二32px，半压固定第二20px与跟随16px；简化显示32px标称；第二独立旋转90度/圆度0.5；在途切换预设仍保持捕获几何。灰度、裁切、关闭和橡皮不显示第二圈，仅悬停选项在绘画时隐藏。
- 两张不同PNG生成不同SVG轮廓；关闭复合光标时第二轮廓未生成，开启后加载；当前和在途主/第二身份分开保持，绘画中改回程序笔尖仍用原图片，抬笔后释放捕获显示。截图artifacts/brush-composite-cursor-ui-{1,2}.png已目视检查，两套轮廓及DPR固定线宽正确。
- 数位笔伴随Hover事件不再按鼠标样本覆盖姿态；旧用例显式指定真实鼠标设备、把物理指针与模拟样本对齐到像素格，并在异步位置投递结束后发送一对方向样本。保留原旋转/倾斜/压力/在途/鼠标接管断言，新增数位笔设备Hover保留姿态断言。没有跳过失败项或改用纯数据断言替代显示检查。

这是基础笔尖复合轮廓，不预演随机散布/形状抖动、时间平滑队列或独立路径的全部当前印章。实体数位板和其他平台未在本轮验证；非分隔绘制、收尖、湿混及高级ABR等完整目标继续后续。
