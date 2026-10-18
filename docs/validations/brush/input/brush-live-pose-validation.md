# 实时动态光标验收

2026-10-14，ABI1.38 / BRUSH_POSE=8589934592。新Pose句柄create/bind_prepared/read/clear/destroy；24字节PaintTipPose半径/角度/圆度。每客户端悬停与绘画各一个句柄，跨笔触复用，clear释放捕获资源；就绪后只持有所选不可变预设，释放整目录ticket。读取使用try_result，不等待编译或在GUI读取文件。基础几何与目录签名按参数变化缓存，移动不序列化目录；原生缓存就绪后不复制ID字符串。

Qt实际轮廓使用原生返回值。未知悬停压力保留名义大小，旋转/倾斜通道仍有效；落笔后按压力缩放。方向采用实际输入位移向量，角度计算在Rust。绘画期间保留起笔几何/图片身份与轮廓，后续编辑不改变本笔光标；结束/取消释放并切回当前配置。简化圆按既有偏好使用名义大小。视口只变换显示，失活/离开数位板重置输入，原轮廓缓存及DPI线宽保留。

最小验证通过：

- 核心brush_device共8项，新增pose已知/未知压力、角度/圆度与绘画几何共用、方向与坏向量；原设备像素/图片角落/捕获/历史回归通过。
- FFI brush_pose：准确准备、可重复绑定、库销毁后仍读捕获结果、clear/错flags/陈旧句柄清输出。准备期间Busy分支检查输出清空；未做故意阻塞编译的时延基准，不把此测试称为GUI延迟测量。
- Qt 5项，见本地artifacts/brush-live-pose-ui-tests.txt：真实Qt TabletHover/Press/Move/Release与窗口/item两种递送，角度90°、60°倾斜后的圆度、悬停名义大小/落笔压力变化；绘画中改参数仍保持捕获值，释放后新配置生效；鼠标方向；旧静态程序光标与图片孔洞/旋转/缩放/DPI实际渲染；切换程序笔尖时本笔图片路径保留，取消后恢复；设备设置/真实预览与保存回归。无QML警告。
- Release构建/cbindgen检查、核心/IO/FFI all-target clippy -D warnings、C/C++静态/共享4项ABI smoke通过。新增24字节布局/字段偏移与create-bind-read-clear-destroy实际调用。

未跑完整tools/check.py、真实数位板硬件、跨平台或性能基准；未宣称PS/Procreate手感达到目标。随机/散布/纹理/双笔刷/颜色/平滑/湿混、ABR动态及VBR分组扩展仍继续，完整笔刷系统未完成。
