# 图片笔尖轮廓验收

2026-10-13：ABI 1.30，Rust固定0.5覆盖轮廓、孔洞/多区域/鞍点判定、外缘闭合、归一几何，资源仓库有界SVG缓存与Qt后台请求已接入。显示按原始比例/圆度/角度变换，双色线宽随界面单位保持；鼠标移动/半径改变不重扫资源。过期资源身份结果拒绝；透明空路径与未准备/出错简化轮廓有区别。

相关验证：4项核心测试通过（含16种2×2阈值布局、准确矩形外边、3个闭合区域、灰度插值、透明/阈值、复杂度明确拒绝）；7项资源IO测试通过（新增同Arc缓存命中、8项淘汰/重建、不持有mask、无效身份/取消）；1项资源FFI集成通过（轮廓buffer需求/完整返回）；4项Qt回归通过，新增真实鼠标移动后的渲染像素、图片孔洞/比例/角度、固定线宽、简化开关与无历史变化。

Qt新增渲染测试在Windows、Qt6.8.3软件后端、QT_SCALE_FACTOR=1/2通过。已查看 `artifacts/brush-outline-ui-1.png` / `artifacts/brush-outline-ui-2.png`，实际显示旋转矩形内部孔洞及分離小区域。初版假hover截图为空，改为真实鼠标移动并要求捕获像素包含描边后重新验证，不能用属性通过替代实际渲染。

Release增量构建及生成头检查通过；C/C++静态/共享4项ABI烟测通过；核心/IO/FFI全目标Clippy -D warnings通过。日志 `artifacts/brush-outline-build.log`、`artifacts/brush-outline-final-build.log`、`artifacts/brush-outline-qt-1.txt`、`artifacts/brush-outline-render-1.txt`、`artifacts/brush-outline-render-2.txt`。未运行完整仓库检查、其他操作系统或真实数位板硬件验证。

复杂纹理超过65536线段时明确提示并显示简化轮廓；精细阈值拓扑是分段线性网格近似。半透明图片悬停预览、屏幕误差受控简化、动态倾斜/旋转、VBR图片资源/ABR/高级动态/湿混继续后续，不能据此认为完整笔刷系统已完成。
