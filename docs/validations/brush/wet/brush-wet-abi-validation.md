# 湿混原生入口 ABI1.50 验收

2026-10-15，Windows/MSVC/Rust、Qt6.8.3 Release。ABI1.50/WET_BRUSH bit45新增共享工具句柄、状态/清洗/准确准备装载、异步会话绑定和显式湿混state/包入口。state17/VBR1.15/behavior14保持；Qt参数页尚未接，旧prepare/package入口继续拒绝新配置。只验证本阶段及相关路径，未运行完整tools/check.py。

- async_session::wet_api三项公开接口测试通过，日志artifacts/brush-wet-abi-public.log。32/40字节DTO和中心字段offset24，版本50/能力标记、空载/手动清洗装载、描述reserved/size/radius越界拒绝且原载体不变；错类型/过期句柄及错误输出清零。修改当前库后指定旧准备仍使用旧load，指定新准备才用新load。
- 公开会话绑定验证跨core错误归属与序号清零；实际两个独立core/session共享工具，真实两笔图层序列化像素与Rust配置湿混参考逐字节一致，中心载色一致。绘画期间info/clean/reload返回Busy且输出清零，活动会话重绑异步Busy保持当前笔触；取消不采用新载色，undo/redo保持像素；销毁工具令牌和库令牌后，已绑定会话/已捕获笔触仍可成功绘画。
- 新库包读写入口完整保留关闭非默认wetMix；旧入口Unsupported且已有文件不改，size-only/完整读取与文件重开保持。不是旧程序子集无资源包接口的扩展。
- 原生wet_七项通过，日志brush-wet-abi-native.log：Weak限额包含销毁令牌后仍被强引用的64个工具，释放一个所有者可再创建；共享图片笔尖/纹理经新包入口落资源后，state17准备保持完整湿混和同源Arc，捕获预设在外部库令牌销毁后仍可编译；旧入口拒绝及既有四项异步湿混/历史/取消/预算/New/stop回归保持。限额单位测试使用与会话同类的Arc保留来隔离全局并发，未创建64个实际绘画会话。
- 去重共10项相关Rust用例。workspace/all-targets cargo check、paint-ffi全目标clippy -D warnings通过（brush-wet-abi-{check,clippy}.log）。
- cbindgen生成头并由Release构建内--check确认，应用/FFI构建通过（brush-wet-abi-{header,build}.log）。四项C/C++静态/共享smoke通过（brush-wet-abi-smoke.log）：DTO布局、实际create/info/clean/reload拒绝/destroy调用；C检查版本/功能位。现有Qt颜色设置/捕获/真实预览/VBR/保存重开/绘画历史回归通过（brush-wet-abi-qt.txt）。Qt仍使用旧入口，不宣称湿混交互完成；部署工具仍提示SDK缺translations/catalogs.json，--no-translations构建成功。

外部工具最多64个仍被令牌/排队绑定/会话持有的对象，默认内部会话工具不占外部配额；全局tombstone限制仍有效。共享Arc在取出后释放注册表锁，准备装载只try_result，不在调用线程等待编译。手动动作不产生文档历史；40字节状态只报告载体中心预乘RGBA，未初始化与busy/error区分由返回状态和INITIALIZED标志决定。

后续：Qt state17镜像/关闭配置/重置基准与保存、统一共享工具绑定/寿命、湿混分类参数与手动清洗装载按钮、实际有色底预览；随后推进双色湿混、收尖、非分隔、高级ABR及完整设备/跨平台/性能验收。新原生能力不等于完整笔刷系统或用户界面已完成。
