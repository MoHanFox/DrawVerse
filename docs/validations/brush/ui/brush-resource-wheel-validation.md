# 资源列表滚轮验收

2026-10-21，Windows Qt 6.8.3 Release。

用实际共享 Picker 嵌入可滚外层，100 个资源、首尾各连续 50 个原生 QWheelEvent，验证内层到边界、外层位置不变；补充 pixelDelta 与 ScrollMomentum、区域外滚轮仍移动外层以及无 QML 警告。

软件 Qt 和 D3D11 均通过。共享图层行的紧凑组/原位改名两项软件回归也通过。Release 程序与工作流测试构建通过，输出 `build/qt/bin/Release`。未覆盖实体触摸板手感和跨平台。
