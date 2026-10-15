# 湿混共享工具载色与原生会话验收

2026-10-15，Windows/MSVC/Rust及Qt6.8.3 Release。本阶段实现Rust共享WetTool/独占WetLease及原生Engine湿混起笔/结束；不新增C ABI，ABI1.49的湿混准备/库包临时拒绝继续有效。未运行完整tools/check.py。

- brush_wet_tool三项通过：共享克隆/跨线程独占，busy期间清洗/重载/快照/再租用拒绝；租约drop保留旧载色，consume commit才采用；旧快照保持不可变，非法尺寸重载原子失败、手动清洗。真实Document取消/提交及undo/redo隔离工具状态，错误段回滚；持有线程panic释放租约，独立工具互不占用。与brush_wet_settings七项及brush_wet_color五项共15项核心用例通过，日志artifacts/brush-wet-owner-core.log。
- 原生wet_actor四项通过，日志brush-wet-owner-native.log：真实异步两笔、颜色动态与载色/混合参数捕获，实际图层剪贴序列化像素和三个网格采样与直接Rust参考一致，单笔历史及两步undo/redo不修改工具；取消和有效大段预算失败恢复图层/旧载色，New及stop释放在途占用，取消超载分支保持；共享工具的两个会话报告Busy，重复Begin不释放原笔触，其他文档冲突不会改变原活动图层；0opacity真实拾色仍提交载体但不产生像素历史。
- 原生旧湿混准备/包拒绝一项和既有actor_tests三项通过，日志brush-wet-owner-{gate,actor}.log。液化预览/控制屏障、吸色坐标与panic会话可释放保持。去重共23项相关Rust用例。
- paint-core/paint-ffi全目标clippy -D warnings、Release构建及生成头--check、四项C/C++静态/共享ABI smoke通过，日志brush-wet-owner-{clippy,build,abi}.log。既有Qt实际颜色设置/捕获/预览/VBR/保存重开/像素历史回归通过，日志brush-wet-owner-qt.txt。不是Qt湿混交互验证。构建部署工具仍提示SDK缺translations/catalogs.json（--no-translations构建成功）。

工具锁只保护载体快照/占用及有界手动动作，不持锁执行Document或等待工作线程。lease不可克隆，checked序号不回绕；结束历史成功后执行无额外可失败工作的consume提交。文档undo/redo不回退工具载色，失败/取消/关闭通过drop释放并保留上一笔采用状态。会话可由with_storage_and_wet_tool注入共享工具，当前生产普通构造为独立工具，跨文档应用级共享尚需C ABI/Qt绑定。

本轮验证了真实预算失败和取消路径，未额外注入原生历史/磁盘写失败或构造真实输入队列溢出；不得把超载Cancel分支回归说成队列压力验收。文件替换的防泄漏清理由同一释放路径实现，尚未单独做文件加载与湿混并发测试。

后续：有界外部工具句柄、准备/资源桥、C ABI控制/状态与跨文档绑定；Qt完整湿混参数/关闭保存/基准/有色预览、手动清洗装载入口；双色湿混、收尖、非分隔、高级ABR与完整设备/跨平台/性能验收仍待完成。不将本阶段视为完整用户湿混工具或整个笔刷系统完成。
