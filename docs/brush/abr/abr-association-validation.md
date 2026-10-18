# ABR样本关联验收

2026-10-13：现代Pascal样本身份/可变前缀、重复拒绝、sampledData严格关联、末尾NUL规范化、同名预设顺序与共享资源、computed不强加图片及完整未映射描述符保留已实现。没有用户导入入口/新ABI。

默认针对性9项测试通过：关联2、分节3、描述符桥1、旧采样3；含可变身份、重复身份、同名共享、computed、缺引用/未知形状/内嵌NUL/取消及既有raw/RLE/边界/孔洞回归。paint-io全目标Clippy -D warnings通过。日志 `artifacts/abr-association-tests.log`。

显式外部诊断1项通过（默认ignored，因为文件不分发；本轮实际以--ignored执行）：ag-psd上游sample-and-pattern，资源文件最后修改commit `c34c9ff731720e0b1083298d09b31838310a5ed4`，公开文件分别下载到ignored artifacts。ABR SHA256 `e0d1ac1a947bb62d4b4eb31fda08e2be10b2a35434e722434226c8a8b158abeb`；参考PNG SHA256 `b44e17959fa648d549d6c110f9e4816508f9943fdf938106abae9b66a2a09a23`。

诊断确认6.2、一个112×105样本、身份a8f34db0-4814-634e-807c-c61aecde18fd、名称Bobbys Brush 743与sampledBrush正确关联，并逐像素对照上游参考PNG alpha完全一致。文件来源：[测试目录](https://github.com/Agamnentzar/ag-psd/tree/master/test/abr-read/sample-and-pattern)；[许可](https://raw.githubusercontent.com/Agamnentzar/ag-psd/master/LICENSE)明确图片/笔刷资产不属于MIT许可，因此未把它们提交或作为可分发授权语料。该结果不证明完整PS渲染等效或资产再分发许可。日志 `artifacts/abr-association-external.log`。

未运行Qt/ABI或全量仓库检查；仍需参数/动态转换与具体支持报告、来源持久化、用户导入、授权可分发真实语料及PS像素参考。角度/取样位置/混合等执行等价性不能由描述符解析或参考alpha推定。
