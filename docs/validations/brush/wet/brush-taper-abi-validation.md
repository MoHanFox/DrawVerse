# 起收笔原生入口验收

2026-10-16，Windows。对应[设计](../../../brush/wet/brush-taper-abi-design.md)。ABI1.51新增STROKE_TAPER（bit46）及显式prepare_state_taper、library_package_read_taper/write_taper，复用原prepared起笔/工具两色/湿混租约。支持级别保存在准备Recipe和资源重建中；原Legacy/Wet入口继续拒绝新起收笔。未修改DTO布局、state18或VBR1.22格式。

最小相关验证已通过：

- `cargo test --manifest-path core/Cargo.toml -p paint-ffi --test async_session public_ --locked`：6项通过，包含3项新起收笔及3项旧湿混公开入口回归。新测试覆盖启用/关闭、干绘/湿混的准确准备字节及真实异步两笔，完整线性像素剪贴数据逐字节对照Rust Document；单笔历史、撤销重做和湿混载体revision/中心颜色一致。开始后准备另一修订、第二笔开始后销毁库句柄，仍保持原捕获。
- 新公开入口的版本/feature、参数错误清零、失败草稿保留当前目录均通过。图片VBR在新资源目录写读，size-only和无效输出指针不安装PNG；完整输出后资源存在、配置JSON一致，坏JSON不覆盖文件。持有旧图片票据、准备新无图片修订后再次读取旧票据，完整起收笔和图片身份保持，资源准备Recipe重建支持级别未丢失。
- `cargo test --manifest-path core/Cargo.toml -p paint-ffi --lib old_native_entries_reject --locked`：2项旧Wet/Taper保护用例通过，旧接口支持边界和失败不发布/不写文件保持。
- paint-ffi所有目标Clippy（-D warnings）通过。使用paint-api-gen生成头后，Windows Release及生成头字节检查通过。
- C/C++ static/shared四项ABI通过；共享smoke实际调用新增准备（含旧合法state兼容）和新包接口的坏路径/清零分支，既有DTO尺寸和偏移断言保持。完整新state18执行由上述公开Rust C ABI测试覆盖，不以坏路径调用替代像素验证。
- Qt6.8.3 windows平台、software Quick、环境倍率1：现有双色控件/像素/保存交换及湿混涂抹强度/手指策略/历史两项通过（初始化清理共4 passed，0 failed）。未添加尚未存在的起收笔Qt测试或参数入口，跨平台验收后置。

日志：`artifacts/brush-taper-abi-{tests,legacy,clippy,header,release,smoke}.log`和`brush-taper-abi-qt.txt`（本机输出不入库）。现有工具链包装器执行，无新依赖；无关全量未重跑。Windows开发版在确认应用未运行后同步构建。

下一阶段：Qt完整state18镜像/关闭值与resetState基准、目录复制/重开/VBR、真实白色自动预览及设置页接入；更新Qt最低ABI与所需能力后改用新入口。当前仍无用户可见起收笔设置，不能据此宣布完整笔刷系统完成；湿混双色/橡皮、完整ABR/16位、非分隔、VBR分组及其余整体验收继续后续。
