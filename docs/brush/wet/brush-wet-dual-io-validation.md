# 湿混双色保存交换验收

2026-10-16，Windows。对应[设计](brush-wet-dual-io-design.md)。实现代码已保存在检查点 `47b27ef`，本验收按当前工作树和实际日志确认，不改写该提交。

WetSettings 的显式独立双载色策略保存在 `wetMix.dualColor:"independent-carriers-v1"`。包括关闭值在内要求 state19、VBR1.23、behavior16 和 wet-dual-color 能力；缺省保持旧湿混语义及旧序列化字节，null/未知模型/错误类型明确拒绝。实际墨水仍属工具状态。Rust目录/包能够编译并执行标记配置，旧Legacy/Wet/Taper原生准备及包入口暂拒该策略，防止旧Qt丢值；没有扩展本轮C ABI或开放组合UI。

最小验证通过：

- `brush_wet_dual_exchange` 6项通过：启用/关闭×配对/独立两路径，state19及VBR1.23往返的真实两笔完整像素及两载体全部格点采样相等，undo/redo保持；标记、旧版本夹带、behavior/能力/配置profile和降级严格拒绝。默认策略恢复后旧状态字节一致，混包选择最高minor23、独立旧预设仍为minor0，持有的旧目录修订不受编辑影响。
- 同源PNG供主/第二笔尖和纹理使用，并与起收笔、原ABR来源组合；新资源目录重建后的实际两笔像素、双载色一致，原包/记录字节不改，取消保存保留原文件。仅新增关闭策略标记也不能写入旧ABR转换基线。合法/关闭/非法公开实例均实际经Rust编译执行或明确拒绝。
- `old_native_entries_reject_wet_dual_policy_including_closed_without_publication_or_resource_install` 通过：开启/关闭策略在三种旧准备支持级别均Unsupported且当前目录不变；三种库包入口的探测和完整缓冲均Unsupported、required清零，不安装PNG；写入及旧独立包写入不覆盖已有文件。
- 既有起收笔交换6项、湿混交换4项和公开C ABI `async_session public_` 6项通过，共16项旧Rust函数回归。保留旧格式及来源行为，不冒称新公开C ABI已支持组合。
- 新完整Schema `contracts/vbr-wet-dual-v1.23.schema.json` 及3个实例已加入；Draft202012检查、7个新旧实例及6个标记/能力变异结果符合预期。Schema保留先前图片/动态/来源/起收笔子集；严格数值和曲线语义仍按各既有运行时规则校验。
- 三相关crate全目标Clippy（-D warnings）通过。Windows Release、生成头一致性、四项C/C++ static/shared ABI通过。
- Qt6.8.3 windows / software Quick / 环境倍率1：原湿混跨文档偏好保存/VBR导入及湿混涂抹强度/手指/历史两项通过（初始化清理共4 passed，0 failed）。本阶段未重复不相关参数页键盘输入用例，前一阶段记录的偶发输入提交问题仍未定位，不将本轮结果当作该问题已修复。

日志：`artifacts/brush-wet-dual-io-{tests,guard,legacy,public,schema,clippy,release,abi}.log`、`artifacts/brush-wet-dual-io-qt.txt`。本机产物不入库；未运行无关全量或跨平台验收。

确认程序未运行后开发版已同步最终构建。下一阶段增加完整原生准备/包能力与双角色手动装载/状态接口，再接Qt镜像、重置基准和组合控制。湿混橡皮/双色涂抹、非分隔双笔刷、高级ABR/16位、VBR分组与整体验收仍待后续，完整笔刷系统未完成。
