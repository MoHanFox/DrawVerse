# 固定白色预览验收

2026-10-16，Windows优先。用户明确取消试画，未接入的试画代码/设计及构建更改已删除，无试画入口；总体设计和验收计划移除该要求。对应[设计](brush-white-preview-design.md)。

自动预览仍由隔离Rust会话执行真实笔尖和动态，工具混合固定正常；后台240×60结果将RGBA8888预乘RGB替换为原alpha值，alpha字节保持。显示为纯白，透明/半透明及孔洞保留，包括颜色动态、双色和湿混输出。当前两色不再进入预览请求或缓存身份，换色/交换不生成新请求；真实画布仍使用原始两色和颜色动态/混合。颜色偏好保存、预设和文件格式不变。

最小相关验证：

- Windows11 / Qt6.8.3 / windows平台、software Quick、环境倍率1：新增实际工作区用例通过，含初始化清理3 passed、0 failed（artifacts/brush-white-preview-final-qt.txt）。程序、HSV颜色动态、双色溢出、湿混与真实PNG孔洞图片五类，逐像素检查所有可见像素RGB255，透明与半透明都存在；主文档generation/revision/modified/历史保持，当前两色/交换/工具混合不改变缓存或触发额外请求。
- 五个相关既有功能用例通过（artifacts/brush-white-preview-qt.txt中各自PASS）：双色关闭值/版本/基准及过期token、双色实际两色绘画/捕获/交换/重开、颜色动态实际颜色/alpha/历史、湿混手动载色/清洗/真实像素与预览工具隔离、全局颜色及历史色重启。旧“换色使预览变色”和有色湿混缩略图断言按新用户规则更新；真实画布颜色断言保持。
- 首轮新增用例只创建独立客户端，未走实际QML预览去抖/原生规范化后的重请求流程，没有收到最终预览。改为创建真实工作区，五类配置全部通过；未降低像素或缓存检查。
- Release及生成头字节检查通过（artifacts/brush-white-preview-release.log）；C/C++ static/shared四项通过（artifacts/brush-white-preview-abi.log）。无Rust算法或C ABI/格式变化，无关全量测试未运行，跨平台后置。

完整笔刷系统继续进行，起收笔/其他高级动态、完整ABR/16位、湿混双色与橡皮、VBR分组等仍待后续；试画不再属于目标范围。
