# 湿混双色 Engine 归还验收

2026-10-16，Windows。对应[设计](brush-wet-dual-engine-design.md)。

真实 Session actor 的 typed Begin 在有效湿混双色时借用完整 WetInk，调用明确 Document 入口；End 在 Document 像素/历史成功返回后整对 commit_ink。单载体 End 继续通过旧主载体 commit 保留未操作的第二载色。失败路径、跨文档独占租约和工具状态不进入历史的边界保持。不增加ABI/格式或公开准备入口，旧IO/Qt仍拒绝组合。

验证：

- 6项新 `wet_dual_actor` 测试通过。两路径×起收笔开关×主颜色动态共8组真实串行队列，每组两笔，与直接 Document 的完整线性像素图层剪贴字节相等；主/第二载体settings/edge/revision及全部格点中心采样相等。修改起笔方副本不影响在途捕获，两个undo/redo恢复像素且不回退工具载色。
- 显式Cancel、Cancel(true)过载取消路径、无效超预算输入、New、NewWhite和stop分别不采用两角色；跨文档Busy、重复Begin不偷租约/不改选择，错误目标释放新租约，成功后另一个文档继续使用完整载色并对照Document像素。实际图层瓦片预算失败恢复原图/历史/两角色；actor panic后stop释放工具且不采用。这里验证actor的Cancel(true)路径，未冒称新测试实际塞满输入队列。
- 无像素拾色成功仍整对采用且不新增历史；切回普通单载体湿混后，副载体revision及全部采样保持。
- 4项既有 `wet_actor` 及1项 `wet_smudge_actor` 通过；6项 `async_session public_` C ABI旧湿混/起收笔准备、两笔像素/历史、载色/Busy、销毁令牌保留捕获及包资源回归通过。共17项Rust函数验证通过。
- paint-ffi全目标Clippy（-D warnings）、Windows Release及既有生成头一致性、四项C/C++ static/shared ABI通过。
- Qt6.8.3 windows / software Quick / 环境倍率1：湿混涂抹独立强度/手指/历史用例通过；手动载色及预览参数页用例首次在键盘输入湿度15后未提交到.15而失败，单项复核通过（3 passed含初始化清理）。本轮未修改该输入链路，偶发失败根因未定位，不声称整组首次全绿或已修复；保留两次日志，后续若再次出现需进一步定位。没有测试尚未开放的组合UI。

日志：`artifacts/brush-wet-dual-engine-{tests,legacy,smudge,public,clippy,release,abi}.log`、`artifacts/brush-wet-dual-engine-qt.txt`（首轮）、`artifacts/brush-wet-dual-engine-qt-input-recheck.txt`（单项复核），本机产物不入库。未跑无关全量或跨平台验收。

确认程序未运行后开发版已同步最终构建。下一阶段为完整保存版本/能力、原生准备和Qt组合入口，不再把Engine双载色归还作为缺失项；公开C ABI仍没有新组合准备能力。湿混橡皮/双色涂抹、非分隔双笔刷、完整ABR/16位、VBR分组等继续后续，整个笔刷系统仍未完成。
