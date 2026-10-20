# 图层行驻留验收

2026-10-21，Windows Release，Qt 6.8.3。

- 扩展 `qt_workflow_tests::maskLinkButtonMovesIndependentlyAndPersistsWithUndo`：真实点击链接、移动、undo/redo 期间持续断言链条与图层行指针未销毁，链条透明度为 1；原有独立/链接蒙版坐标、组蒙版与 ORA 重开断言保留。
- 软件 Qt 与 D3D11 各通过一次，无 QML 警告。完整 Release 程序与工作流测试构建通过，输出 `build/qt/bin/Release`。
- 未做实体连续手感或全量测试，不将保留 UI 对象等同于所有复杂组合已支持 GPU 移动。
