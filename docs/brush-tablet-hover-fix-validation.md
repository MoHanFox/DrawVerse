# 数位笔悬停修复验收

2026-10-15，Windows / Qt 6.8.3 / Release，原生 windows 平台与 software Quick 后端。

- 双色绘制页不再出现两个创建描边预设按钮；保留已有预设、模板构造与真实描边能力。像素、压力边宽、在途捕获、撤销重做、VBR交换/重开回归通过。
- 新增直接数位笔 HoverEnter/Move（无先行鼠标输入）、笔触期间伴随悬停隔离、松笔后继续悬停、窗口原生光标 Blank 与恢复、真正鼠标接管、画布外鼠标移动、窗口离开、隐藏及离开感应范围回归。
- 原 Tablet 窗口/Item 输入、压力大小、倾斜圆度/旋转、主第二轮廓与真实浮动画布笔触历史均通过。

Release构建通过（artifacts/tablet-hover-build.log）；构建中生成头检查通过，无 ABI 或格式变化。

QT_SCALE_FACTOR=1：6个相关测试函数通过，含初始化/清理共8 passed，0 failed（artifacts/tablet-hover-qt-1.txt）。QT_SCALE_FACTOR=2：4个光标测试函数通过，含初始化/清理共6 passed，0 failed（artifacts/tablet-hover-qt-2.txt）。这是环境缩放覆盖，非实际硬件DPR1/2或实体数位板认证。

测试在原生窗口暴露完成后设置已知恢复光标，所有同步光标断言完成后再等待异步历史，避免等待期间真实桌面鼠标正常接管改变预期；鼠标接管本身通过显式真实鼠标设备事件验证。未运行无关全量测试。实体数位板的驱动与硬件悬停仍需实机确认。
